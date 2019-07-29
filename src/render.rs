
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

#[derive(Debug)]
pub enum DebugMode {
    Normals,
    Steps,
}

pub struct ConfigBuilder {
    config: Config,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        ConfigBuilder {
            config: Config {
                width: 100,
                height: 100,
                max_steps: 100,
                jobs: 1,
                buffer_size: 1000,
                debug_mode: None,
            }
        }
    }
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

    pub fn set_max_steps(mut self, steps: usize) -> Self {
        self.config.max_steps = steps;
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

    pub fn set_debug_mode(mut self, mode: DebugMode) -> Self {
        self.config.debug_mode = Some(mode);
        self
    }

    pub fn build(self) -> Arc<Config> {
        Arc::new(self.config)
    }
} 

#[derive(Debug)]
pub struct Config {
    width: usize,
    height: usize,
    max_steps: usize,
    jobs: usize,
    buffer_size: usize,
    debug_mode: Option<DebugMode>,
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
            match cfg_copy.debug_mode {
                None =>
                    render_job(scene_copy, camera_copy, cfg_copy, i, send_copy),

                Some(DebugMode::Normals) =>
                    render_normals_job(scene_copy, camera_copy, cfg_copy, i, send_copy),

                Some(DebugMode::Steps) =>
                    render_steps_job(scene_copy, camera_copy, cfg_copy, i, send_copy),
            }
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
            if let Some(res) = ray.march(cfg.max_steps, 1.0, |pt| scene.sdf(pt)) {
                let pat = scene.get_pattern(res.material.0);
                let mat = scene.get_material(res.material.1);
                let normal = res.normal(|pt| scene.sdf(pt));

                let obj_color = pat.color_at(get_pattern, &res.object_space_point);

                // the direction towards the eye
                let eyev = -ray.direction;

                for light in scene.iter_lights() {
                    let point = res.world_space_point + normal * 0.01;
                    let light_dir = light.position - point;
                    let dist = light_dir.magnitude();

                    // check to see if the path to the light is obstructed
                    let light_visible = Ray::new(point, light_dir.normalize())
                        .march(cfg.max_steps, 1.0, |pt| scene.sdf(pt))
                        .map_or(true, |hit| hit.distance >= dist);

                    pixel += mat.lighting(
                        light, &obj_color, &res.world_space_point,
                        &eyev, &normal, light_visible,
                    ) * light_weight;

                }
            }

            send.send((x,y,pixel)).expect("Failed to send pixel!");
        }
    }
}

fn render_normals_job(
    scene: Arc<Scene>,
    camera: Arc<Camera>,
    cfg: Arc<Config>,
    idx: usize,
    send: SyncSender<(usize,usize,Color)>) {

    for x in 0 .. cfg.width {
        for y in (idx .. cfg.height).step_by(cfg.jobs) {
            let ray = camera.ray_for_pixel(x, y);

            let mut pixel = Color::black();
            if let Some(res) = ray.march(cfg.max_steps, 1.0, |pt| scene.sdf(pt)) {
                let normal = res.normal(|pt| scene.sdf(pt));
                pixel
                    .set_r(0.5 + normal.x / 2.0)
                    .set_g(0.5 + normal.y / 2.0)
                    .set_b(0.5 + normal.z / 2.0);
            }

            send.send((x,y,pixel)).expect("Failed to send pixel!");
        }
    }

}

fn render_steps_job(
    scene: Arc<Scene>,
    camera: Arc<Camera>,
    cfg: Arc<Config>,
    idx: usize,
    send: SyncSender<(usize,usize,Color)>) {

    let step_max = cfg.max_steps as f32;

    for x in 0 .. cfg.width {
        for y in (idx .. cfg.height).step_by(cfg.jobs) {
            let ray = camera.ray_for_pixel(x, y);

            let mut pixel = Color::black();
            if let Some(res) = ray.march(cfg.max_steps, 1.0, |pt| scene.sdf(pt)) {
                let step_val = 1.0 - (res.steps as f32) / step_max;
                pixel.set_r(step_val).set_b(step_val);
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
