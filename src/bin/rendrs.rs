
extern crate yaml_rust;
extern crate num_cpus;

use std::{
    sync::{Arc},
    env,
    fs::read,
    path::Path,
};

use rendrs::render::{ConfigBuilder,Config,DebugMode};

#[derive(Debug)]
struct Opts {
    render_config: Arc<Config>,
    scene: String,
    output: String,
}

fn parse_usize(str: &String, ty: &str) -> Result<usize,String> {
    let width = str.parse().map_err(|_err| format!("failed to parse {}", ty))?;
    Ok(width)
}

fn parse_opts() -> Result<Opts,String> {
    let mut builder = ConfigBuilder::default();

    builder = builder.set_jobs(num_cpus::get());

    let mut args = env::args().skip(1);

    let mut scene_file = None;
    let mut output_file = None;

    while let Some(arg) = args.next() {
        match arg.as_ref() {
            "--debug" => {
                let debug_str = args.next().ok_or("Invalid --debug")?;
                match debug_str.as_ref() {
                    "steps" =>
                        builder = builder.set_debug_mode(DebugMode::Steps),

                    "normals" =>
                        builder = builder.set_debug_mode(DebugMode::Normals),

                    _ =>
                        return Err("Invalid --debug".to_string())
                };
            },

            "--jobs" => {
                let jobs = parse_usize(&args.next().ok_or("missing jobs value")?, "jobs")?;
                builder = builder.set_jobs(jobs);
            },

            "--width" => {
                let width = parse_usize(&args.next().ok_or("missing width")?, "width")?;
                builder = builder.set_width(width);
            },

            "--height" => {
                let height = parse_usize(&args.next().ok_or("missing height")?, "height")?;
                builder = builder.set_width(height);
            },

            file => {
                if Path::new(file).is_file() {
                    if scene_file.is_none() {
                        scene_file = Some(arg);
                    } else if output_file.is_none() {
                        output_file = Some(arg);
                    } else {
                        return Err(format!("Unexpected argument `{}`", file));
                    }
                } else {
                    return Err(format!("File `{}` is missing", file));
                }
            }
        }
    }

    let scene = scene_file.ok_or("Missing scene file")?;
    let output = output_file.ok_or("Missing output file")?;

    Ok(Opts {
        render_config: builder.build(),
        scene,
        output,
    })
}

fn main() -> Result<(), String> {
    let opts = parse_opts()?;

    let scene = read(Path::new(&opts.scene))
        .map_err(|_err| format!("Failed to read scene file `{}`", opts.scene))?;
    let scene_str = String::from_utf8(scene)
        .map_err(|_err| format!("failed to read scene file `{}`", opts.scene))?;
    let yaml = yaml_rust::YamlLoader::load_from_str(scene_str.as_str());

    println!("{:?}", yaml);

    Ok(())
}
