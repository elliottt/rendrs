use crate::{camera::Camera, canvas, ray::Ray, scene::Scene};
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use std::{sync::Arc, thread};

// Implementations of the Integrator trait
pub mod debug_normals;
pub mod debug_steps;

#[derive(Debug)]
pub struct Config {
    pub jobs: usize,
    pub max_steps: usize,
    pub max_reflections: usize,
}

unsafe impl Send for Config {}
unsafe impl Sync for Config {}

impl Default for Config {
    fn default() -> Self {
        Config {
            jobs: num_cpus::get(),
            max_steps: 200,
            max_reflections: 10,
        }
    }
}

pub trait Integrator {
    /// Render the given scene using this integrator.
    fn render(&mut self, cfg: &Config, scene: &Scene, ray: &Ray) -> canvas::Color;
}

pub struct RenderJob {
    width: usize,
    height: usize,
    recv: Receiver<Output>,
}

struct Output {
    pub x: usize,
    pub y: usize,
    pub color: canvas::Color,
}

/// Render a scene to a channel that can accept and write out pixels.
pub fn render<I: Integrator + Clone + Send + 'static>(
    cfg: Arc<Config>,
    integrator: I,
    camera: Arc<Camera>,
    scene: Arc<Scene>,
) -> RenderJob {
    let jobs = cfg.jobs.max(1);

    let tiles_width = camera.width_px / 16;
    let tiles_height = camera.height_px / 16;

    let (in_send, in_recv) = bounded(tiles_width);
    let (out_send, out_recv) = unbounded();

    for _ in 0..jobs {
        let cfg_copy = cfg.clone();
        let integrator_copy = integrator.clone();
        let camera_copy = camera.clone();
        let scene_copy = scene.clone();
        let input = in_recv.clone();
        let output = out_send.clone();
        thread::spawn(move || {
            render_worker(
                cfg_copy,
                integrator_copy,
                camera_copy,
                scene_copy,
                input,
                output,
            );
        });
    }

    let width = camera.width_px;
    let height = camera.height_px;

    thread::spawn(move || {
        // iterate the tiles of the image
        for ty in 0..tiles_height {
            let y = ty * 16;
            let tile_height = 16.min(height - y);
            for tx in 0..tiles_width {
                let x = tx * 16;
                in_send
                    .send(Tile::new(x, y, 16.min(width - x), tile_height))
                    .unwrap();
            }
        }
    });

    RenderJob {
        width: width,
        height: height,
        recv: out_recv,
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

fn render_worker<I: Integrator>(
    cfg: Arc<Config>,
    mut integrator: I,
    camera: Arc<Camera>,
    scene: Arc<Scene>,
    input: Receiver<Tile>,
    output: Sender<Output>,
) {
    let sample_frac = camera.sample_fraction();
    while let Ok(tile) = input.recv() {
        for (x, y) in tile.iter() {
            let color = camera
                .rays_for_pixel(x, y)
                .map(|ray| integrator.render(&cfg, &scene, &ray) * sample_frac)
                .sum();
            output.send(Output { x, y, color }).unwrap();
        }
    }
}

/// Write out the contents of a rendering job to a canvas, so that it may be written out.
/// Additionally, show a progress bar on the command line.
pub fn write_canvas(job: RenderJob) -> canvas::Canvas {
    let mut canvas = canvas::Canvas::new(job.width, job.height);

    let expected = job.width * job.height;
    let mut pb = pbr::ProgressBar::new(expected as u64);

    for _ in 0..expected {
        let out = job.recv.recv().expect("Failed to read all pixels!");
        let pixel = canvas.get_mut(out.x, out.y).expect("Invalid pixel!");
        *pixel = out.color;
        pb.inc();
    }
    pb.finish_print("done");

    canvas
}
