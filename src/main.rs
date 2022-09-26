use anyhow::Error;
use clap::{Parser, Subcommand};

mod bvh;
mod camera;
mod canvas;
mod integrator;
mod math;
mod parser;
mod ray;
mod scene;
mod transform;
mod web;

use parser::Target;

#[derive(Parser, Debug)]
#[clap(author = "Trevor Elliott", version = "0.2")]
struct Options {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Serve,

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
        Command::Serve => {
            println!("serving!");
        }

        Command::Render { threads, scene } => {
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
                        image::save_buffer(
                            path,
                            &canvas.data(),
                            width,
                            height,
                            image::ColorType::Rgb8,
                        )
                        .unwrap();
                    }

                    Target::Ascii => println!("{}", canvas.to_ascii()),
                }
            }
        }
    }

    Ok(())
}
