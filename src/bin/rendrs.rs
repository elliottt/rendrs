
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

    let jobs = parse_usize(&matches, "jobs")?;
    let debug = match matches.value_of("debug") {
        Some("steps") => Some(DebugMode::Steps),
        Some("normals") => Some(DebugMode::Normals),
        _ => None,
    };

    let (scene,cameras) = yaml::parse(scene_path)?;
    let scene_ref = Arc::new(scene);

    for camera in cameras {
        let mut builder = ConfigBuilder::default()
            .set_jobs(jobs)
            .set_width(camera.width_px)
            .set_height(camera.height_px);

        let cfg = match debug {
            Some(ref mode) => builder.set_debug_mode(mode.clone()).build(),
            None => builder.build(),
        };

        let recv = render(scene_ref.clone(), Arc::new(camera), cfg.clone());
        write_canvas(cfg, recv).save(output_path);
    }

    Ok(())
}
