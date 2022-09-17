use anyhow::Error;

mod bvh;
mod camera;
mod canvas;
mod integrator;
mod lighting;
mod math;
mod parser;
mod ray;
mod scene;
mod transform;

use parser::Target;

fn main() -> Result<(), Error> {
    let input = std::fs::read_to_string("test.scene")?;
    let (scene, renders) = parser::parse(&input)?;

    for render in renders {
        let canvas = integrator::render(
            render.canvas_info.clone(),
            &scene,
            render.root,
            &render.integrator,
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

    Ok(())
}
