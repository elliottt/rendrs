use anyhow::Error;

mod camera;
mod canvas;
mod integrator;
mod lighting;
mod math;
mod parser;
mod ray;
mod scene;
mod transform;

fn main() -> Result<(), Error> {
    let input = std::fs::read_to_string("test.scene")?;
    let (scene, renders) = parser::parse(&input)?;

    for mut render in renders {
        integrator::render(
            &mut render.canvas,
            &scene,
            render.root,
            &mut render.integrator,
        );
        let width = render.canvas.width();
        let height = render.canvas.height();

        println!("Writing {}", &render.path.to_str().unwrap());
        image::save_buffer(
            render.path,
            &render.canvas.data(),
            width,
            height,
            image::ColorType::Rgb8,
        )
        .unwrap();
    }

    Ok(())
}
