
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
    pattern::{Pattern},
    ray::{reflect,Ray,MarchResult},
    scene::Scene,
    shapes::ShapeId,
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
                max_reflections: 10,
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

    let containers = Containers::new();

    render_body(idx, camera, cfg, send, |ray| {
        find_hit(&world, &containers, &ray).map_or_else(
            || Color::black(),
            |hit| shade_hit(&world, &containers, &hit, 0))
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

/// Objects that a ray is within during refraction processing.
#[derive(Debug,Clone)]
struct Container {
    object: ShapeId,
    refractive_index: f32,
}

#[derive(Debug,Clone)]
struct Containers {
    containers: Vec<Container>,
}

impl Containers {
    fn new() -> Self {
        Containers{ containers: Vec::new() }
    }

    fn refractive_index(&self) -> f32 {
        self.containers.last().map_or_else(
            || 1.0,
            |container| container.refractive_index)
    }

    /// Returns `-1.0` when the ray originated from within another object, or 1.0 when it is
    /// outside.
    fn determine_sign(&self) -> f32 {
        if self.containers.is_empty() {
            1.0
        } else {
            -1.0
        }
    }

    /// Determine the n1/n2 refractive indices for a hit involving a transparent object, and return
    /// a boolean that indicates if the ray is leaving the object.
    fn process_hit(&mut self, object: ShapeId, refractive_index: f32) -> (bool,f32,f32) {
        let n1 = self.refractive_index();

        let mut leaving = false;

        // if the object is already in the set, find its index.
        let existing_ix = self.containers
            .iter()
            .enumerate()
            .find_map( |arg| {
                if arg.1.object == object {
                    leaving = true;
                    Some(arg.0)
                } else {
                    None
                }
            });

        match existing_ix {
            Some(ix) => {
                // the object exists, so remove it.
                self.containers.remove(ix);
            },
            None =>
                self.containers.push(Container{ object, refractive_index }),
        }

        let n2 = self.refractive_index();

        return (leaving,n1,n2)
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
    n1: f32,
    n2: f32,
    leaving: bool,
    containers: Containers,
}

impl<'a> Hit<'a> {
    fn new<'b>(
        world: &'b World,
        containers: &Containers,
        ray: &Ray,
        res: MarchResult,
    ) -> Hit<'b> {

        let material = world.scene.get_material(res.material);

        let mut containers = containers.clone();

        let (leaving,n1,n2) = if material.transparent > 0.0 {
            containers.process_hit(res.object_id, material.refractive_index)
        } else {
            let val = containers.refractive_index();
            (false,val,val)
        };

        let normal = res.normal(|pt| world.scene.sdf(pt));

        Hit{
            object_space_point: res.object_space_point,
            world_space_point: res.world_space_point,
            normal,

            // this doesn't need to be computed here
            reflectv: reflect(&ray.direction, &normal),

            // the direction towards the eye
            eyev: -ray.direction,

            material: material,
            pattern: world.scene.get_pattern(res.pattern),

            // refraction information
            n1,
            n2,

            leaving,

            containers,
        }
    }

    fn reflection_ray(&self, containers: &Containers) -> Ray {
        // start the origin along the ray a bit
        let origin = self.world_space_point + self.reflectv * 0.01;
        Ray::new(origin, self.reflectv.clone(), containers.determine_sign())
    }

    fn refraction_ray(&self, containers: &Containers) -> Option<Ray> {
        // the normal must point inside when the refraction ray originated within the object
        let refrac_normal =
            if self.leaving {
                -self.normal
            } else {
                self.normal
            };

        let n_ratio = self.n1 / self.n2;
        let cos_i = self.eyev.dot(&refrac_normal);
        let sin2_t = n_ratio.powi(2) * (1.0 - cos_i.powi(2));

        if sin2_t > 1.0 {
            None
        } else {
            let cos_t = (1.0 - sin2_t).sqrt();
            let direction = (refrac_normal * (n_ratio * cos_i - cos_t) - self.eyev * n_ratio).normalize();

            let origin = self.world_space_point + direction * 0.01;
            Some(Ray::new(origin, direction, containers.determine_sign()))
        }
    }

    fn schlick(&self) -> f32 {
        schlick(&self.eyev, &self.normal, self.n1, self.n2)
    }
}

fn schlick(eyev: &Vector3<f32>, normal: &Vector3<f32>, n1: f32, n2: f32) -> f32 {
    let mut cos = eyev.dot(&normal);

    // total internal reflection
    if n1 > n2 {
        let n = n1 / n2;
        let sin2_t = n.powi(2) * (1.0 - cos.powi(2));
        if sin2_t > 1.0 {
            return 1.0;
        }

        cos = (1.0 - sin2_t).sqrt();
    }

    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    (r0 + (1.0 - r0) * (1.0 - cos).powi(5)).min(1.0)
}

#[test]
fn test_schlick() {
    let reflectance = schlick(&Vector3::new(0.0, 0.0, -1.0), &Vector3::new(0.0, 0.0, -1.0), 1.0, 1.5);
    assert!(reflectance - 0.04 < 0.001, format!("{} != {}", reflectance, 0.04));

    let reflectance = schlick(&Vector3::new(0.0, 0.0, -1.0), &Vector3::new(1.0, 0.0, 0.0), 1.0, 1.5);
    assert!(reflectance - 1.0 < 0.001, format!("{} != {}", reflectance, 1.0));
}

/// March a ray until it hits something. If it runs out of steps, or exceeds the max bound, returns
/// `None`.
fn find_hit<'a>(
    world: &'a World,
    containers: &Containers,
    ray: &Ray
) -> Option<Hit<'a>> {
    ray.march(world.cfg.max_steps, |pt| world.scene.sdf(pt))
        .map(|res| Hit::new(world, containers, ray, res))
}

/// Shade the color according to the material properties, and light.
fn shade_hit(
    world: &World,
    containers: &Containers,
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

    if reflection_count < world.cfg.max_reflections {
        let refl = reflected_color(world, containers, hit, reflection_count);
        let refrac = refracted_color(world, &hit.containers, hit, reflection_count);

        if hit.material.reflective > 0.0 && hit.material.transparent > 0.0 {
            let reflectance = hit.schlick();
            output + refl * reflectance + refrac * (1.0 - reflectance)
        } else {
            output + refl + refrac
        }
    } else {
        output
    }
}

/// Compute the color from a reflection.
fn reflected_color(
    world: &World,
    containers: &Containers,
    hit: &Hit,
    reflection_count: usize,
) -> Color {
    let reflective = hit.material.reflective;
    if reflective <= 0.0 {
        Color::black()
    } else {
        let ray = hit.reflection_ray(containers);
        find_hit(world, &containers, &ray).map_or_else(
            || Color::black(),
            |refl_hit| shade_hit(world, &containers, &refl_hit, reflection_count + 1) * reflective)
    }
}

/// Compute the additional color due to refraction.
fn refracted_color(
    world: &World,
    containers: &Containers,
    hit: &Hit,
    reflection_count: usize
) -> Color {
    let transparent = hit.material.transparent;
    if transparent <= 0.0 {
        Color::black()
    } else {
        hit.refraction_ray(containers)
            .and_then(|ray| find_hit(world, &containers, &ray))
            .map_or_else(
                || Color::black(),
                |refr_hit| shade_hit(world, &containers, &refr_hit, reflection_count+1) * transparent)
    }
}


/// A predicate that tests whether or not a light is visible from a hit in the scene.
///
/// TODO: this currently considers transparent objects to be opaque
fn light_visible(world: &World, hit: &Hit, light: &Light) -> bool {
    // move slightly away from the surface that was contacted
    let point = hit.world_space_point + hit.normal * 0.01;
    let light_dir = light.position - point;
    let dist = light_dir.magnitude();

    // check to see if the path to the light is obstructed
    Ray::new(point, light_dir.normalize(), 1.0)
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
