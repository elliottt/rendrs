use actix::prelude::*;
use actix_files as fs;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;
use anyhow::Error;
use fs::NamedFile;
use notify::event::ModifyKind;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use rand::{rngs::ThreadRng, Rng};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::render;

#[actix_web::main]
pub async fn serve(port: u16, threads: usize, scene: String) -> Result<(), Error> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let render_server = RenderServer::new().start();

    let scene_path = PathBuf::from(scene).canonicalize()?;
    let scene_dir = scene_path.parent().unwrap().to_path_buf();

    let mut watcher = {
        let render_server = render_server.clone();
        notify::recommended_watcher(move |event| match event {
            Ok(Event {
                kind: EventKind::Modify(ModifyKind::Data(_)),
                paths,
                ..
            }) if paths.contains(&scene_path) => {
                render_server.do_send(Message::File {
                    name: String::from("norf"),
                });
            }
            _ => (),
        })?
    };

    watcher.watch(&scene_dir, RecursiveMode::NonRecursive)?;

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(render_server.clone()))
            .service(web::resource("/").to(index))
            .route("/ws", web::get().to(client_route))
            .service(fs::Files::new("/static", "web").index_file("index.html"))
    })
    .workers(2)
    .bind(("127.0.0.1", port))?
    .run();

    open::that(format!("http://127.0.0.1:{}/", port))?;

    server.await?;

    Ok(())
}

async fn index() -> impl Responder {
    NamedFile::open_async("./web/index.html").await.unwrap()
}

async fn client_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<RenderServer>>,
) -> Result<HttpResponse, actix_web::Error> {
    println!("web socket!");
    ws::start(
        RenderClient {
            id: 0,
            hb: Instant::now(),
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
enum Message {
    File { name: String },
    Ascii { name: String, content: String },
}

#[derive(Message)]
#[rtype(usize)]
struct Connect {
    addr: Recipient<Message>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Disconnect {
    id: usize,
}

struct RenderServer {
    clients: HashMap<usize, Recipient<Message>>,
    rng: ThreadRng,
}

impl Actor for RenderServer {
    type Context = Context<Self>;
}

impl RenderServer {
    fn new() -> Self {
        RenderServer {
            clients: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
}

impl Handler<Message> for RenderServer {
    type Result = ();

    fn handle(&mut self, msg: Message, _: &mut Context<Self>) -> Self::Result {
        for client in self.clients.values() {
            client.do_send(msg.clone())
        }
    }
}

impl Handler<Connect> for RenderServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = self.rng.gen::<usize>();

        msg.addr.do_send(Message::File {
            name: String::from("foobar"),
        });

        self.clients.insert(id, msg.addr);
        id
    }
}

impl Handler<Disconnect> for RenderServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        self.clients.remove(&msg.id);
    }
}

struct RenderClient {
    id: usize,
    hb: Instant,
    addr: Addr<RenderServer>,
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

impl RenderClient {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Heartbeat failed, disconnecting");
                act.addr.do_send(Disconnect { id: act.id });
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for RenderClient {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("started");
        let addr = ctx.address();
        self.addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        println!("stopped");
        self.addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for RenderClient {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }

            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            _ => (),
        }
    }
}

impl Handler<Message> for RenderClient {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        match msg {
            Message::File { name } => {
                ctx.text(format!("{{ \"type\": \"file\", \"name\": \"{}\" }}", name));
            }

            Message::Ascii { name, content } => {
                let content = content.replace("\"", "\\\"");
                ctx.text(format!(
                    "{{ \"type\": \"ascii\", \"name\": \"{}\", \"content\": \"{}\" }}",
                    name,
                    content.replace("\"", "\\\"")
                ));
            }
        }
    }
}
