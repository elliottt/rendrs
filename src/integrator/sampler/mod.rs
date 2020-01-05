use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use std::{sync::Arc, thread};

use crate::{
    camera::Camera,
    canvas::Color,
    integrator::{Integrator, Output, RenderJob, Tile},
    ray::Ray,
    scene::Scene,
};

pub mod debug_normals;
pub mod debug_steps;
pub mod whitted;

pub trait LightIncoming: Send + Sync {
    /// Compute the light incoming for a given ray.
    fn light_incoming(&self, config: &Config, scene: &Scene, ray: &Ray) -> Color;
}

pub struct SamplerIntegrator {
    config: Config,
    light_incoming: Arc<dyn LightIncoming>,
}

impl SamplerIntegrator {
    pub fn whitted(config: Config) -> Self {
        SamplerIntegrator { config, light_incoming: Arc::new(whitted::Whitted) }
    }

    pub fn debug_normals(config: Config) -> Self {
        SamplerIntegrator { config, light_incoming: Arc::new(debug_normals::DebugNormals) }
    }

    pub fn debug_steps(config: Config) -> Self {
        SamplerIntegrator { config, light_incoming: Arc::new(debug_steps::DebugSteps) }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    jobs: usize,
    max_steps: usize,
    max_reflections: usize,
}

impl Config {
    pub fn new(jobs: usize, max_steps: usize, max_reflections: usize) -> Self {
        Config {
            jobs: jobs.max(1),
            max_steps,
            max_reflections,
        }
    }
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

impl Integrator for SamplerIntegrator {
    fn render(&self, scene: Arc<Scene>, camera: Arc<Camera>) -> RenderJob {
        let jobs = self.config.jobs;

        let tiles_width = (camera.width_px + 16) / 16;
        let tiles_height = (camera.height_px + 16) / 16;

        let (in_send, in_recv) = bounded(tiles_width);
        let (out_send, out_recv) = unbounded();

        for _ in 0..jobs {
            let cfg_copy = self.config.clone();
            let light_incoming = self.light_incoming.clone();
            let camera_copy = camera.clone();
            let scene_copy = scene.clone();
            let input = in_recv.clone();
            let output = out_send.clone();
            thread::spawn(move || {
                render_worker(
                    cfg_copy,
                    light_incoming,
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
}

fn render_worker(
    cfg: Config,
    li: Arc<dyn LightIncoming>,
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
                .map(|ray| li.light_incoming(&cfg, &scene, &ray) * sample_frac)
                .sum();
            values.push(color);
        }
        output.send(Output { tile, values }).unwrap();
    }
}

#[test]
fn test_tile() {
    let results: Vec<(usize, usize)> = Tile::new(0, 0, 0, 0).iter().collect();
    assert!(results.is_empty());

    let results: Vec<(usize, usize)> = Tile::new(0, 0, 1, 1).iter().collect();
    assert_eq!(vec![(0, 0)], results);

    let results: Vec<(usize, usize)> = Tile::new(0, 0, 2, 1).iter().collect();
    assert_eq!(vec![(0, 0), (1, 0)], results);

    let results: Vec<(usize, usize)> = Tile::new(0, 0, 1, 2).iter().collect();
    assert_eq!(vec![(0, 0), (0, 1)], results);
}
