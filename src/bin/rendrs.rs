
extern crate clap;
extern crate num_cpus;
extern crate failure;

use std::{
    sync::{Arc},
};

use failure::Error;

use clap::{Arg,App,ArgMatches};

use rendrs::{
    render::{ConfigBuilder,DebugMode,render,write_canvas},
    scene::yaml,
};

fn parse_usize(matches: &ArgMatches, label: &str) -> Result<usize,Error> {
    let val = matches
        .value_of(label)
        .expect(&format!("Is `{}` missing a default?", label))
        .parse()?;
    Ok(val)
}

fn main() -> Result<(),Error> {

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

    let scene_path = matches.value_of("SCENE").expect("Missing SCENE");
    let output_path = matches.value_of("OUTPUT").expect("Missing OUTPUT");

    let mut builder = ConfigBuilder::default()
        .set_jobs(parse_usize(&matches, "jobs")?)
        .set_width(parse_usize(&matches, "width")?)
        .set_width(parse_usize(&matches, "height")?);

    builder = match matches.value_of("debug") {
        Some("steps") => builder.set_debug_mode(DebugMode::Steps),
        Some("normals") => builder.set_debug_mode(DebugMode::Normals),
        _ => builder,
    };

    let (scene,camera) = yaml::parser(scene_path)?;

    let cfg = builder.build();
    let recv = render(Arc::new(scene), Arc::new(camera), cfg.clone());
    write_canvas(cfg, recv).save(output_path);

    Ok(())
}
