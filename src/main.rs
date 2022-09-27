use std::path::{Path, PathBuf};

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Error;
use clap::{Parser, Subcommand};
use crossbeam::channel;
use notify::{
    event::{DataChange, Event, EventKind, ModifyKind},
    RecursiveMode, Watcher,
};

mod bvh;
mod camera;
mod canvas;
mod integrator;
mod math;
mod parser;
mod ray;
mod scene;
mod transform;

use parser::Target;

#[derive(Parser, Debug)]
#[clap(author = "Trevor Elliott", version = "0.2")]
struct Options {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Serve {
        #[clap(
            short,
            long,
            help = "The port to serve the interactive ui from",
            default_value_t = 8080
        )]
        port: u16,

        #[clap(short,
           long,
           help = "The number of threads to spawn",
           default_value_t = num_cpus::get() as u64,
           value_parser = clap::value_parser!(u64).range(1..=num_cpus::get() as u64),
        )]
        threads: u64,

        #[clap(help = "The scene file to render")]
        scene: String,
    },

    Render {
        #[clap(short,
           long,
           help = "The number of threads to spawn",
           default_value_t = num_cpus::get() as u64,
           value_parser = clap::value_parser!(u64).range(1..=num_cpus::get() as u64),
        )]
        threads: u64,

        #[clap(help = "The scene file to render")]
        scene: String,
    },
}

fn main() -> Result<(), Error> {
    let opts = Options::parse();

    match opts.command {
        Command::Serve {
            port,
            threads,
            scene,
        } => {
            serve(port, threads as usize, scene)?;
        }

        Command::Render { threads, scene } => {
            let path = PathBuf::from(&scene);
            render_scene(threads as usize, &path)?;
        }
    }

    Ok(())
}

enum Message {
    Render,
}

#[actix_web::main]
pub async fn serve(port: u16, threads: usize, scene: String) -> Result<(), Error> {
    let server = HttpServer::new(|| App::new().service(hello))
        .bind(("127.0.0.1", port))?
        .run();

    let (input, work): (channel::Sender<Message>, _) = channel::bounded(1);

    let path = PathBuf::from(&scene).canonicalize()?;

    let mut watcher = notify::recommended_watcher(move |res| match res {
        // TODO: it would be great to debounce these
        Ok(Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Any)),
            paths,
            ..
        }) if paths.contains(&path) => {
            input.send(Message::Render).unwrap();
        },

        Err(e) => eprintln!("Watch error: {:?}", e),

        _ => (),
    })?;

    let path = PathBuf::from(&scene);
    let dir = path.parent().unwrap();

    watcher.watch(dir, RecursiveMode::NonRecursive)?;

    std::thread::spawn(move || {
        let scene = path.clone();

        for msg in work {
            render_scene(threads, &scene);
        }

        println!("all done!");
    });

    open::that(format!("http://127.0.0.1:{}/", port))?;

    server.await?;

    drop(watcher);

    Ok(())
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

fn render_scene(threads: usize, scene: &Path) -> Result<(), Error> {
    let input = std::fs::read_to_string(scene)?;
    let (scene, renders) = parser::parse(&input)?;

    for render in renders {
        let canvas = integrator::render(
            render.canvas_info.clone(),
            &scene,
            render.root,
            &render.integrator,
            threads as usize,
        );
        let width = canvas.width();
        let height = canvas.height();

        match render.target {
            Target::File { path } => {
                println!("Writing {}", &path.to_str().unwrap());
                image::save_buffer(path, &canvas.data(), width, height, image::ColorType::Rgb8)
                    .unwrap();
            }

            Target::Ascii => println!("{}", canvas.to_ascii()),
        }
    }

    Ok(())
}
