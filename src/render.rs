use anyhow::Error;
use std::path::{Path, PathBuf};

use crate::{integrator, parser};

pub enum Output {
    File { path: PathBuf },
    Ascii { name: String, chars: String },
}

pub fn render_scene(threads: usize, scene: &Path) -> Result<impl Iterator<Item = Output>, Error> {
    let input = std::fs::read_to_string(scene)?;
    let (scene, renders) = parser::parse(&input)?;

    Ok(renders.into_iter().map(move |render| {
        let canvas = integrator::render(
            render.canvas_info.clone(),
            &scene,
            render.root,
            render.sampler,
            render.integrator,
            threads as usize,
        );
        let width = canvas.width();
        let height = canvas.height();

        match render.target {
            parser::Target::File { path } => {
                image::save_buffer(&path, &canvas.data(), width, height, image::ColorType::Rgb8)
                    .unwrap();
                Output::File { path }
            }

            parser::Target::Ascii { name } => Output::Ascii {
                name,
                chars: canvas.to_ascii(),
            },
        }
    }))
}
