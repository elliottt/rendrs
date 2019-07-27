
use std::{
    sync::{
        Arc,
        mpsc::{sync_channel,SyncSender,Receiver},
    },
    thread,
};

use crate::{
    camera::Camera,
    canvas::{Canvas,Color},
    ray::Ray,
    shapes::Scene,
};

impl Default for ConfigBuilder {
    fn default() -> Self {
        ConfigBuilder {
            config: Config {
                width: 100,
                height: 100,
                jobs: 1,
                buffer_size: 1000,
            }
        }
    }
}

pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn set_width(mut self, width: usize) -> Self {
        self.config.width = width;
        self
    }

    pub fn set_height(mut self, height: usize) -> Self {
        self.config.height = height;
        self
    }

    pub fn set_jobs(mut self, jobs: usize) -> Self {
        self.config.jobs = usize::max(jobs, 1);
        self
    }

    pub fn set_buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    pub fn build(self) -> Arc<Config> {
        Arc::new(self.config)
    }
} 

pub struct Config {
    width: usize,
    height: usize,
    jobs: usize,
    buffer_size: usize,
}

pub fn render(scene: Arc<Scene>, camera: Arc<Camera>, cfg: Arc<Config>) -> Receiver<(usize,usize,Color)> {

    let (send,recv) = sync_channel(cfg.buffer_size);

    // start jobs
    for i in 0..cfg.jobs {
        // each job will render rows that are (row `mod` jobs == i)
        let scene_copy = scene.clone();
        let camera_copy = camera.clone();
        let cfg_copy = cfg.clone();
        let send_copy = send.clone();
        thread::spawn(move || {
            render_job(scene_copy, camera_copy, cfg_copy, i, send_copy)
        });
    }

    recv
}

fn render_job(
    scene: Arc<Scene>,
    camera: Arc<Camera>,
    cfg: Arc<Config>,
    idx: usize,
    send: SyncSender<(usize,usize,Color)>) {

    let get_pattern = |pid| scene.get_pattern(pid);

    let light_weight = 1.0 / (scene.num_lights() as f32);

    for x in 0 .. cfg.width {
        for y in (idx .. cfg.height).step_by(cfg.jobs) {
            let ray = camera.ray_for_pixel(x, y);
            let mut pixel = Color::black();
            if let Some(res) = ray.march(|pt| scene.sdf(pt)) {
                let pat = scene.get_pattern(res.material.0);
                let mat = scene.get_material(res.material.1);
                let normal = res.normal(|pt| scene.sdf(pt));

                for light in scene.iter_lights() {
                    let point = res.world_space_point + normal * 0.01;
                    let light_dir = light.position - point;
                    let dist = light_dir.magnitude();

                    // check to see if the path to the light is obstructed
                    let light_visible = Ray::new(point, light_dir.normalize())
                        .march(|pt| scene.sdf(pt))
                        .map_or(true, |hit| hit.distance >= dist);

                    pixel += mat.lighting(
                        light, get_pattern, &pat, &res.object_space_point, &res.world_space_point,
                        &ray.direction, &normal, light_visible,
                    ) * light_weight;

                }
            }

            send.send((x,y,pixel)).expect("Failed to send pixel!");
        }
    }

}

pub fn write_canvas(cfg: Arc<Config>, recv: Receiver<(usize,usize,Color)>) -> Canvas {
    let mut canvas = Canvas::new(cfg.width, cfg.height);

    let expected = cfg.width * cfg.height;

    for _ in 0 .. expected {
        let (x,y,color) = recv.recv().expect("Failed to read all pixels!");
        let pixel = canvas.get_mut(x,y).expect("Pixel out of bounds!");
        *pixel = color
    }

    canvas
}
