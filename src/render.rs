
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
    material::{Light,MaterialId,Material},
    pattern::{PatternId,Pattern},
    ray::{reflect,Ray,MarchResult},
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

    pub fn set_max_reflections(mut self, max_reflections: isize) -> Self {
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
    max_reflections: isize,
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
    refractive_index: f32,
    material: &'a Material,
    pattern: &'a Pattern,
    sign: f32,
}

impl<'a> Hit<'a> {
    fn new<'b>(
        world: &'b World,
        ray: &Ray,
        refractive_index: f32,
        res: MarchResult<(PatternId,MaterialId)>,
    ) -> Hit<'b> {
        let normal = ray.sign * res.normal(|pt| world.scene.sdf(pt));
        Hit{
            object_space_point: res.object_space_point,
            world_space_point: res.world_space_point,
            normal,

            reflectv: reflect(&ray.direction, &normal),

            // the direction towards the eye
            eyev: -ray.direction,

            // the refractive index of the source medium
            refractive_index,

            material: world.scene.get_material(res.material.1),
            pattern: world.scene.get_pattern(res.material.0),

            sign: ray.sign,
        }
    }

    fn reflection_ray(&self) -> Ray {
        // start the origin along the ray a bit
        let origin = self.world_space_point + self.reflectv * 0.01;
        Ray::new(origin, self.reflectv.clone(), self.sign)
    }

    fn refraction_ray(&self) -> Option<Ray> {
        let n_ratio = self.refractive_index / self.material.refractive_index;
        let cos_i = self.eyev.dot(&self.normal);
        let sin2_t = n_ratio.powi(2) * (1.0 - cos_i.powi(2));

        if sin2_t > 1.0 {
            None
        } else {
            let cos_t = (1.0 - sin2_t).sqrt();
            let direction = self.normal * (n_ratio * cos_i - cos_t) - self.eyev * n_ratio;

            let origin = self.world_space_point + direction * 0.01;
            Some(Ray::new(origin, direction, -self.sign))
        }
    }
}

/// March a ray until it hits something. If it runs out of steps, or exceeds the max bound, returns
/// `None`.
fn find_hit<'a>(world: &'a World, ray: &Ray, refractive_index: f32) -> Option<Hit<'a>> {
    ray.march(world.cfg.max_steps, |pt| world.scene.sdf(pt))
        .map(|res| Hit::new(world, ray, refractive_index, res))
}

/// Shade the color according to the material properties, and light.
fn shade_hit(
    world: &World,
    hit: &Hit,
    reflection_count: isize,
) -> Color {
    let color = hit.pattern.color_at(&|pid| world.scene.get_pattern(pid), &hit.object_space_point);

    let mut output = Color::black();

    for light in world.scene.iter_lights() {
        output += hit.material.lighting(
                light, &color, &hit.world_space_point,
                &hit.eyev, &hit.normal, light_visible(&world, &hit, light)
            ) * world.light_weight;
    }

    let additional =
        if reflection_count < world.cfg.max_reflections {
            let refl = reflected_color(world, hit, reflection_count);
            let refrac = refracted_color(world, hit, reflection_count);
            refl + refrac
        } else {
            Color::black()
        };

    output + additional
}

/// Compute the color from a reflection.
fn reflected_color(world: &World, hit: &Hit, reflection_count: isize) -> Color {
    let reflective = hit.material.reflective;
    if reflective <= 0.0 {
        Color::black()
    } else {
        let ray = hit.reflection_ray();
        find_hit(world, &ray, hit.refractive_index).map_or_else(
            || Color::black(),
            |refl_hit| shade_hit(world, &refl_hit, reflection_count + 1) * reflective)
    }
}

/// Compute the additional color due to refraction.
fn refracted_color(world: &World, hit: &Hit, reflection_count: isize) -> Color {
    let transparent = hit.material.transparent;
    if transparent <= 0.0 {
        Color::black()
    } else {
        hit.refraction_ray()
            .and_then(|internal_ray|
                find_hit(world, &internal_ray, hit.material.refractive_index))
            .and_then(|mut internal_hit| {
                // TODO: big hack, what if we aren't just exiting this object?
                internal_hit.material = hit.material;
                internal_hit
                    .refraction_ray()
                    .and_then(|external_ray|
                        find_hit(world, &external_ray, internal_hit.material.refractive_index))
            })
            .map_or_else(
                || Color::black(),
                |external_hit| shade_hit(world, &external_hit, reflection_count+1) * transparent)
    }
}


/// A predicate that tests whether or not a light is visible from a hit in the scene.
fn light_visible(world: &World, hit: &Hit, light: &Light) -> bool {
    // move slightly away from the surface that was contacted
    let point = hit.world_space_point + hit.normal * 0.01;
    let light_dir = light.position - point;
    let dist = light_dir.magnitude();

    // check to see if the path to the light is obstructed
    Ray::new(point, light_dir.normalize(), hit.sign)
        .march(world.cfg.max_steps, |pt| world.scene.sdf(pt))
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
        if let Some(res) = ray.march(cfg.max_steps, |pt| scene.sdf(pt)) {
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
        if let Some(res) = ray.march(cfg.max_steps, |pt| scene.sdf(pt)) {
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
