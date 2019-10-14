use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use std::{sync::Arc, thread};

use crate::{camera::Camera, canvas::{Canvas,Color}, integrator::Integrator, scene::Scene};

#[derive(Clone, Debug)]
pub struct Config {
    pub jobs: usize,
    pub max_steps: usize,
    pub max_reflections: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            jobs: num_cpus::get(),
            max_steps: 200,
            max_reflections: 10,
        }
    }
}

pub struct RenderJob {
    width: usize,
    height: usize,
    expected_tiles: usize,
    recv: Receiver<Output>,
}

struct Output {
    pub tile: Tile,
    pub values: Vec<Color>,
}

/// Render a scene to a channel that can accept and write out pixels.
pub fn render<I: ?Sized + Integrator + 'static>(
    cfg: Arc<Config>,
    integrator: Arc<I>,
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
        expected_tiles: tiles_width * tiles_height,
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

fn render_worker<I: ?Sized + Integrator>(
    cfg: Arc<Config>,
    integrator: Arc<I>,
    camera: Arc<Camera>,
    scene: Arc<Scene>,
    input: Receiver<Tile>,
    output: Sender<Output>,
) {
    let sample_frac = camera.sample_fraction();
    while let Ok(tile) = input.recv() {
        let mut values = Vec::with_capacity(tile.size());
        for (x, y) in tile.iter() {
            let color = camera
                .rays_for_pixel(x, y)
                .map(|ray| integrator.render(&cfg, &scene, &ray) * sample_frac)
                .sum();
            values.push(color);
        }
        output.send(Output {tile, values}).unwrap();
    }
}

/// Write out the contents of a rendering job to a canvas, so that it may be written out.
/// Additionally, show a progress bar on the command line.
pub fn write_canvas(job: RenderJob) -> Canvas {
    let mut canvas = Canvas::new(job.width, job.height);

    let mut pb = pbr::ProgressBar::new(job.expected_tiles as u64);

    for _ in 0..job.expected_tiles {
        let out = job.recv.recv().expect("Failed to read all pixels!");
        for ((x,y),value) in out.tile.iter().zip(out.values.iter()) {
            let pixel = canvas.get_mut(x, y).expect("Invalid pixel!");
            *pixel = value.clone();
        }
        pb.inc();
    }
    pb.finish_print("done");

    canvas
}
