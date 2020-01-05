use crossbeam_channel::Receiver;

use std::sync::Arc;

use crate::{
    camera::Camera,
    canvas::{Canvas, Color},
    scene::Scene,
};

pub mod sampler;

pub trait Integrator {
    /// Render the given scene using this integrator, using this camera.
    fn render(&self, scene: Arc<Scene>, camera: Arc<Camera>) -> RenderJob;
}

struct Output {
    pub tile: Tile,
    pub values: Vec<Color>,
}

pub struct RenderJob {
    width: usize,
    height: usize,
    expected_tiles: usize,
    recv: Receiver<Output>,
}

impl RenderJob {
    /// Write out the contents of a rendering job to a canvas, so that it may be written out.
    /// Additionally, show a progress bar on the command line.
    pub fn write_canvas(&self) -> Canvas {
        let mut canvas = Canvas::new(self.width, self.height);

        let mut pb = pbr::ProgressBar::new(self.expected_tiles as u64);

        for _ in 0..self.expected_tiles {
            let out = self.recv.recv().expect("Failed to read all pixels!");
            for ((x, y), value) in out.tile.iter().zip(out.values.iter()) {
                let pixel = canvas.get_mut(x, y).expect("Invalid pixel!");
                *pixel = value.clone();
            }
            pb.inc();
        }
        pb.finish_print("done");

        canvas
    }
}

struct Tile {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Tile {
    fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Tile {
            x,
            y,
            width,
            height,
        }
    }

    fn size(&self) -> usize {
        self.width * self.height
    }

    fn iter(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let mut x = 0;
        let mut y = 0;
        std::iter::from_fn(move || {
            if y >= self.height {
                return None;
            }

            let res = (self.x + x, self.y + y);

            x += 1;
            if x >= self.width {
                x = 0;
                y += 1;
            }

            Some(res)
        })
    }
}
