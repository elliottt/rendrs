use actix::prelude::*;
use actix_files as fs;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;
use anyhow::Error;
use crossbeam::channel::{self, RecvTimeoutError};
use fs::NamedFile;
use notify::event::ModifyKind;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use rand::{rngs::ThreadRng, Rng};
use std::collections::HashMap;
use std::fmt::Write;
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

        let (send, recv) = channel::bounded(1);

        let watcher_path = scene_path.clone();
        let watcher = notify::recommended_watcher(move |event| match event {
            Ok(Event {
                kind: EventKind::Modify(ModifyKind::Data(_)),
                paths,
                ..
            }) if paths.contains(&watcher_path) => send.send(()).unwrap(),
            _ => (),
        })?;

        std::thread::spawn(move || {
            'outer: loop {
                log::info!("rendering {:?}", scene_path);

                // render the scene
                match render::render_scene(threads, &scene_path) {
                    Ok(outputs) => {
                        let outputs = outputs
                            .map(|output| match output {
                                render::Output::File { path } => Output::File {
                                    name: String::from(
                                        path.file_name().and_then(|os| os.to_str()).unwrap(),
                                    ),
                                },
                                render::Output::Ascii { name, chars } => Output::Ascii {
                                    name,
                                    content: chars,
                                },
                            })
                            .collect();

                        log::info!("render done");

                        let scene = String::from(
                            scene_path.file_name().and_then(|os| os.to_str()).unwrap(),
                        );
                        render_server.do_send(RenderResult { scene, outputs });
                    }

                    Err(err) => log::error!("error: {}", err),
                }

                // wait for the next edit
                if recv.recv().is_err() {
                    break 'outer;
                }

                // debounce edits
                loop {
                    let res = recv.recv_timeout(Duration::from_millis(1000));
                    match res {
                        Ok(_) => continue,
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(_) => break 'outer,
                    }
                }
            }
        });

        watcher
    };

    watcher.watch(&scene_dir, RecursiveMode::NonRecursive)?;

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(render_server.clone()))
            .service(web::resource("/").to(index))
            .route("/ws", web::get().to(client_route))
            .service(fs::Files::new("/output", "."))
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
struct RenderResult {
    scene: String,
    outputs: Vec<Output>,
}

#[derive(Clone)]
enum Output {
    File { name: String },
    Ascii { name: String, content: String },
}

#[derive(Message)]
#[rtype(usize)]
struct Connect {
    addr: Recipient<RenderResult>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Disconnect {
    id: usize,
}

struct RenderServer {
    clients: HashMap<usize, Recipient<RenderResult>>,
    rng: ThreadRng,
    last_result: Option<RenderResult>,
}

impl Actor for RenderServer {
    type Context = Context<Self>;
}

impl RenderServer {
    fn new() -> Self {
        RenderServer {
            clients: HashMap::new(),
            rng: rand::thread_rng(),
            last_result: None,
        }
    }
}

impl Handler<RenderResult> for RenderServer {
    type Result = ();

    fn handle(&mut self, msg: RenderResult, _: &mut Context<Self>) -> Self::Result {
        self.last_result = Some(msg.clone());

        // TODO: buffer the last render result in the server, and send it on new client connections
        for client in self.clients.values() {
            client.do_send(msg.clone())
        }
    }
}

impl Handler<Connect> for RenderServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = self.rng.gen::<usize>();

        self.clients.insert(id, msg.addr.clone());

        if let Some(outputs) = &self.last_result {
            msg.addr.do_send(outputs.clone());
        }

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
                log::trace!("Heartbeat failed, disconnecting");
                act.addr.do_send(Disconnect { id: act.id });
                ctx.stop();
                return;
            }

            log::trace!("sending a ping request");
            ctx.ping(b"");
        });
    }
}

impl Actor for RenderClient {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        self.addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => {
                        act.id = res;
                        log::info!("started client {}", act.id);
                    }
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
        self.hb(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        log::info!("stopping client {}", self.id);
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
            ws::Message::Pong(_) => {
                log::trace!("ping response");
                self.hb = Instant::now()
            }
            _ => (),
        }
    }
}

impl Handler<RenderResult> for RenderClient {
    type Result = ();

    fn handle(&mut self, msg: RenderResult, ctx: &mut Self::Context) {
        let mut buf = String::new();
        let mut sep = "";

        write!(&mut buf, "{{ \"scene\": \"{}\", \"outputs\": [", msg.scene).unwrap();

        for output in msg.outputs {
            write!(&mut buf, "{}", sep).unwrap();
            match output {
                Output::File { name } => {
                    write!(&mut buf, "{{ \"type\": \"file\", \"name\": \"{}\" }}", name).unwrap()
                }

                Output::Ascii { name, content } => {
                    let content = content.replace("\\", "\\\\");
                    let content = content.replace("\n", "\\n");
                    write!(
                        &mut buf,
                        "{{ \"type\": \"ascii\", \"name\": \"{}\", \"content\": \"{}\" }}",
                        name,
                        content.replace("\"", "\\\"")
                    )
                    .unwrap();
                }
            }

            sep = ", ";
        }

        write!(&mut buf, "]}}").unwrap();

        ctx.text(buf);
    }
}
