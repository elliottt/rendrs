use std::path::{Path, PathBuf};

use anyhow::Error;
use clap::{Parser, Subcommand};

mod bvh;
mod camera;
mod canvas;
mod integrator;
mod math;
mod parser;
mod ray;
mod render;
mod scene;
mod transform;
mod web;

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
            web::serve(port, threads as usize, scene)?;
        }

        Command::Render { threads, scene } => {
            let path = PathBuf::from(&scene);
            for output in render::render_scene(threads as usize, &path)? {
                match output {
                    render::Output::File { path } => println!("Wrote file {}", path.to_str().unwrap()),
                    render::Output::Ascii { chars } => println!("{}", chars),
                }
            }
        }
    }

    Ok(())
}
