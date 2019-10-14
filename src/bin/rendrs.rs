extern crate clap;
extern crate failure;
extern crate num_cpus;

use std::sync::Arc;

use failure::Error;

use clap::{App, Arg, ArgMatches};

use rendrs::{
    integrator::{DebugNormals, DebugSteps, Integrator, Whitted},
    render::{render, write_canvas, Config},
    scene::yaml,
};

fn parse_usize(matches: &ArgMatches, label: &str) -> Result<usize, Error> {
    let val = matches
        .value_of(label)
        .unwrap_or_else(|| panic!("Is `{}` missing a default?", label))
        .parse()?;
    Ok(val)
}

fn main() -> Result<(), Error> {
    let cpus = num_cpus::get();
    let cpu_str = cpus.to_string();

    let matches = App::new("rendrs")
        .version("0.1.0")
        .about("Renders scenes")
        .arg(
            Arg::with_name("jobs")
                .short("j")
                .long("jobs")
                .default_value(&cpu_str)
                .value_name("NUM_JOBS")
                .help("Number of concurrent render jobs")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max-reflections")
                .short("r")
                .long("max-reflections")
                .default_value("10")
                .value_name("MAX_REFLECTIONS")
                .help("The maximum number of reflections to allow")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max-steps")
                .short("s")
                .long("max-steps")
                .default_value("200")
                .value_name("MAX_STEPS")
                .help("The maximum number of steps to take when ray marching")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("integrator")
                .long("integrator")
                .possible_values(&["whitted", "debug-steps", "debug-normals"])
                .default_value("whitted")
                .help("Sample Integrator")
                .takes_value(true),
        )
        .arg(Arg::with_name("SCENE").index(1).required(true))
        .arg(Arg::with_name("OUTPUT").index(2).required(true))
        .get_matches();

    let scene_path = matches.value_of("SCENE").expect("Missing SCENE");
    let output_path = matches.value_of("OUTPUT").expect("Missing OUTPUT");

    // TODO: parse the config from the yaml
    let jobs = parse_usize(&matches, "jobs")?;
    let max_steps = parse_usize(&matches, "max-steps")?;
    let max_reflections = parse_usize(&matches, "max-reflections")?;
    let config = Arc::new(Config {
        jobs,
        max_steps,
        max_reflections,
    });

    let (scene, cameras) = yaml::parse(scene_path)?;
    let scene_ref = Arc::new(scene);

    let integrator: Arc<dyn Integrator> = match matches.value_of("integrator") {
        Some("whitted") => Arc::new(Whitted::new()),
        Some("debug-normals") => Arc::new(DebugNormals::new()),
        Some("debug-steps") => Arc::new(DebugSteps::new()),
        int => panic!("Unknown integrator: {:?}", int),
    };

    for camera in cameras {
        let recv = render::<dyn Integrator>(
            config.clone(),
            integrator.clone(),
            Arc::new(camera),
            scene_ref.clone(),
        );
        write_canvas(recv).save(output_path);
    }

    Ok(())
}
