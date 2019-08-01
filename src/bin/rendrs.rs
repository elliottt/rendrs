
extern crate clap;
extern crate num_cpus;
extern crate yaml_rust;

use std::{
    sync::{Arc},
    env,
    fs::read,
    path::Path,
};

use clap::{Arg,App,ArgMatches};
use rendrs::render::{ConfigBuilder,Config,DebugMode};

#[derive(Debug)]
struct Opts {
    render_config: Arc<Config>,
    scene: String,
    output: String,
}

fn parse_usize(matches: &ArgMatches, label: &str, default: usize) -> Result<usize,String> {
    matches
        .value_of(label)
        .expect(&format!("Is `{}` missing a default?", label))
        .parse()
        .map_err(|_| format!("Failed to parse option `{}`", label))
}

fn main() -> Result<(),String> {

    let cpus = num_cpus::get();
    let cpu_str = cpus.to_string();

    let matches =
        App::new("rendrs")
            .version("0.1.0")
            .about("Renders scenes")
            .arg(Arg::with_name("jobs")
                 .short("j")
                 .long("jobs")
                 .default_value(&cpu_str)
                 .value_name("NUM_JOBS")
                 .help("Number of concurrent render jobs")
                 .takes_value(true))
            .arg(Arg::with_name("width")
                 .short("w")
                 .long("width")
                 .default_value("100")
                 .value_name("PIXELS")
                 .help("Width of the image in pixels")
                 .takes_value(true))
            .arg(Arg::with_name("height")
                 .short("h")
                 .long("height")
                 .default_value("100")
                 .value_name("PIXELS")
                 .help("Height of the image in pixels")
                 .takes_value(true))
            .arg(Arg::with_name("debug")
                 .long("debug")
                 .possible_values(&["steps", "normals"])
                 .help("Debug mode")
                 .takes_value(true))
            .arg(Arg::with_name("SCENE")
                 .index(1)
                 .required(true))
            .arg(Arg::with_name("OUTPUT")
                 .index(2)
                 .required(true))
            .get_matches();

    let scene = matches.value_of("SCENE").expect("Missing SCENE");
    let output = matches.value_of("OUTPUT").expect("Missing OUTPUT");

    let mut builder = ConfigBuilder::default()
        .set_jobs(parse_usize(&matches, "jobs", cpus)?)
        .set_width(parse_usize(&matches, "width", cpus)?)
        .set_width(parse_usize(&matches, "height", cpus)?);

    builder = match matches.value_of("debug") {
        Some("steps") => builder.set_debug_mode(DebugMode::Steps),
        Some("normals") => builder.set_debug_mode(DebugMode::Normals),
        _ => builder,
    };

    let opts = Opts {
        render_config: builder.build(),
        scene: scene.to_string(),
        output: output.to_string()
    };

    println!("config: {:?}", opts);

    Ok(())
}
