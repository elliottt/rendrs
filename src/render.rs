
use std::{
    sync::{
        Arc,
        mpsc::{channel,Sender,Receiver},
    },
    thread,
};

use nalgebra::{Point3,Vector3};

use crate::{
    camera::Camera,
    canvas::{Canvas,Color},
    material::{Light,Material},
    pattern::Pattern,
    ray::{reflect,Ray},
    scene::Scene,
};

#[derive(Debug,Clone)]
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
                max_steps: 200,
                jobs: 1,
                debug_mode: None,
                max_reflections: 1,
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

    pub fn set_debug_mode(mut self, mode: DebugMode) -> Self {
        self.config.debug_mode = Some(mode);
        self
    }

    pub fn set_max_reflections(mut self, max_reflections: usize) -> Self {
        self.config.max_reflections = max_reflections;
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
    debug_mode: Option<DebugMode>,
    max_reflections: usize,
}

pub struct RenderedRow {
    y: usize,
    row: Vec<Color>,
}

pub fn render(scene: Arc<Scene>, camera: Arc<Camera>, cfg: Arc<Config>) -> Receiver<RenderedRow> {

    let (send,recv) = channel();

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

fn render_body<Body>(
    idx: usize,
    camera: Arc<Camera>,
    cfg: Arc<Config>,
    send: Sender<RenderedRow>,
    mut body: Body
) where Body: FnMut(Ray) -> Color,
{
    for y in (idx .. cfg.height).step_by(cfg.jobs) {
        let mut row = Vec::with_capacity(cfg.width);
        for x in 0 .. cfg.width {
            row.push(body(camera.ray_for_pixel(x,y)));
        }
        send.send(RenderedRow{ y, row }).expect("Failed to send row!");
    }

}

fn render_job(
    scene: Arc<Scene>,
    camera: Arc<Camera>,
    cfg: Arc<Config>,
    idx: usize,
    send: Sender<RenderedRow>) {

    let world = World::new(cfg.clone(), scene);

    render_body(idx, camera, cfg, send, |ray| {
        if let Some(hit) = find_hit(&world, &ray, 1.0) {
            shade_hit(&world, &hit, 0)
        } else {
            Color::black()
        }
    });
}

struct World {
    cfg: Arc<Config>,
    scene: Arc<Scene>,
    light_weight: f32,
}

impl World {
    fn new(cfg: Arc<Config>, scene: Arc<Scene>) -> Self {
        let light_weight = 1.0 / (scene.num_lights() as f32);
        World{ cfg, scene, light_weight }
    }
}

#[derive(Debug)]
struct Hit<'a> {
    object_space_point: Point3<f32>,
    world_space_point: Point3<f32>,
    normal: Vector3<f32>,
    reflectv: Vector3<f32>,
    eyev: Vector3<f32>,
    material: &'a Material,
    pattern: &'a Pattern,
    sign: f32,
}

impl<'a> Hit<'a> {
    fn reflection(&self) -> Ray {
        // start the origin along the ray a bit
        let origin = self.world_space_point + self.reflectv * 0.01;
        Ray::new(origin, self.reflectv.clone())
    }
}

/// March a ray until it hits something. If it runs out of steps, or exceeds the max bound, returns
/// `None`.
fn find_hit<'a>(world: &'a World, ray: &Ray, sign: f32) -> Option<Hit<'a>> {
    ray.march(world.cfg.max_steps, sign, |pt| world.scene.sdf(pt)).map(|res| {
        let normal = res.normal(|pt| world.scene.sdf(pt));
        Hit{
            object_space_point: res.object_space_point,
            world_space_point: res.world_space_point,
            normal,

            reflectv: reflect(&ray.direction, &normal),

            // the direction towards the eye
            eyev: -ray.direction,

            material: world.scene.get_material(res.material.1),
            pattern: world.scene.get_pattern(res.material.0),

            sign: sign,
        }
    })
}

/// Shade the color according to the material properties, and light.
fn shade_hit(
    world: &World,
    hit: &Hit,
    reflection_count: usize,
) -> Color {
    let color = hit.pattern.color_at(&|pid| world.scene.get_pattern(pid), &hit.object_space_point);

    let mut output = Color::black();

    for light in world.scene.iter_lights() {
        output += hit.material.lighting(
                light, &color, &hit.world_space_point,
                &hit.eyev, &hit.normal, light_visible(&world, &hit, light)
            ) * world.light_weight;
    }

    let reflected =
        if reflection_count < world.cfg.max_reflections {
            reflected_color(world, hit, reflection_count)
        } else {
            Color::black()
        };

    output + reflected
}

/// Compute the color from a reflection.
fn reflected_color(world: &World, hit: &Hit, reflection_count: usize) -> Color {
    let reflective = hit.material.reflective;
    if reflective <= 0.0 {
        Color::black()
    } else {
        let ray = hit.reflection();
        find_hit(world, &ray, hit.sign).map_or_else(
            || Color::black(),
            |refl_hit| shade_hit(world, &refl_hit, reflection_count + 1) * reflective)
    }
}

/// A predicate that tests whether or not a light is visible from a hit in the scene.
fn light_visible(world: &World, hit: &Hit, light: &Light) -> bool {
    // move slightly away from the surface that was contacted
    let point = hit.world_space_point + hit.normal * 0.01;
    let light_dir = light.position - point;
    let dist = light_dir.magnitude();

    // check to see if the path to the light is obstructed
    Ray::new(point, light_dir.normalize())
        .march(world.cfg.max_steps, 1.0, |pt| world.scene.sdf(pt))
        .map_or_else(|| true, |res| res.distance >= dist)
}

fn render_normals_job(
    scene: Arc<Scene>,
    camera: Arc<Camera>,
    cfg: Arc<Config>,
    idx: usize,
    send: Sender<RenderedRow>) {

    render_body(idx, camera, cfg.clone(), send, |ray| {
        let mut pixel = Color::black();
        if let Some(res) = ray.march(cfg.max_steps, 1.0, |pt| scene.sdf(pt)) {
            let normal = res.normal(|pt| scene.sdf(pt));
            pixel
                .set_r(0.5 + normal.x / 2.0)
                .set_g(0.5 + normal.y / 2.0)
                .set_b(0.5 + normal.z / 2.0);
        }
        pixel
    });

}

fn render_steps_job(
    scene: Arc<Scene>,
    camera: Arc<Camera>,
    cfg: Arc<Config>,
    idx: usize,
    send: Sender<RenderedRow>) {

    let step_max = cfg.max_steps as f32;

    render_body(idx, camera, cfg.clone(), send, |ray| {
        let mut pixel = Color::black();
        if let Some(res) = ray.march(cfg.max_steps, 1.0, |pt| scene.sdf(pt)) {
            let step_val = 1.0 - (res.steps as f32) / step_max;
            pixel.set_r(step_val).set_b(step_val);
        }

        pixel
    });

}

pub fn write_canvas(cfg: Arc<Config>, recv: Receiver<RenderedRow>) -> Canvas {
    let mut canvas = Canvas::new(cfg.width, cfg.height);

    let expected = cfg.height;

    for _ in 0 .. expected {
        let row = recv.recv().expect("Failed to read all rows!");
        canvas.blit_row(row.y, row.row);
    }

    canvas
}
