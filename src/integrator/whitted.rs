use crate::{
    canvas::Color,
    integrator::Integrator,
    material::{Light, Material},
    pattern::Pattern,
    ray::{self, MarchResult, Ray},
    render::Config,
    scene::Scene,
    shapes::ShapeId,
    utils::clamp,
};
use nalgebra::{Point3, Vector3};
use std::borrow::Cow;

pub struct Whitted;

impl Whitted {
    pub fn new() -> Self {
        Whitted
    }
}

impl Integrator for Whitted {
    fn render(&self, cfg: &Config, scene: &Scene, ray: &Ray) -> Color {
        let containers = Containers::new();
        find_hit(cfg, scene, &containers, ray).map_or_else(Color::black, |hit| {
            shade_hit(cfg, scene, &containers, &hit, 0)
        })
    }
}

/// March a ray until it hits something. If it runs out of steps, or exceeds the max bound, returns
/// `None`.
fn find_hit<'scene, 'cont>(
    cfg: &Config,
    scene: &'scene Scene,
    containers: &'cont Containers,
    ray: &Ray,
) -> Option<Hit<'scene, 'cont>> {
    ray.march(cfg.max_steps, |pt| scene.sdf(pt))
        .map(|res| Hit::new(scene, containers, ray, res))
}

/// Shade the color according to the material properties, and light.
fn shade_hit(
    cfg: &Config,
    scene: &Scene,
    containers: &Containers,
    hit: &Hit,
    reflection_count: usize,
) -> Color {
    let color = hit
        .pattern
        .color_at(&|pid| scene.get_pattern(pid), &hit.object_space_point);

    let mut output = Color::black();

    for light in scene.iter_lights() {
        output += hit.material.lighting(
            light,
            &color,
            &hit.world_space_point,
            &hit.eyev,
            &hit.normal,
            light_visible(cfg, scene, &hit, light),
        ) * scene.get_light_weight();
    }

    if reflection_count < cfg.max_reflections {
        let refl = reflected_color(cfg, scene, containers, hit, reflection_count);
        let refrac = refracted_color(cfg, scene, &hit.containers, hit, reflection_count);

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
    cfg: &Config,
    scene: &Scene,
    containers: &Containers,
    hit: &Hit,
    reflection_count: usize,
) -> Color {
    let reflective = hit.material.reflective;
    if reflective <= 0.0 {
        Color::black()
    } else {
        let ray = hit.reflection_ray(containers);
        find_hit(cfg, scene, containers, &ray).map_or_else(Color::black, |refl_hit| {
            shade_hit(cfg, scene, containers, &refl_hit, reflection_count + 1) * reflective
        })
    }
}

/// Compute the additional color due to refraction.
fn refracted_color(
    cfg: &Config,
    scene: &Scene,
    containers: &Containers,
    hit: &Hit,
    reflection_count: usize,
) -> Color {
    let transparent = hit.material.transparent;
    if transparent <= 0.0 {
        Color::black()
    } else {
        hit.refraction_ray(containers)
            .and_then(|ray| find_hit(cfg, scene, &containers, &ray))
            .map_or_else(Color::black, |refr_hit| {
                shade_hit(cfg, scene, &containers, &refr_hit, reflection_count + 1) * transparent
            })
    }
}

/// Returns a value between 0 and 1 that indicates how visible the light is. 0 indicates that the
/// light is completely obscured, while 1 indicates that it's completely visible.
fn light_visible(cfg: &Config, scene: &Scene, hit: &Hit, light: &Light) -> f32 {
    // move slightly away from the surface that was contacted
    let point = hit.world_space_point + hit.normal * 0.01;
    let light_dir = light.position - point;
    let light_dist = light_dir.magnitude();

    // manually march the ray, so that we can detect how close it comes to an object.
    let mut ray = Ray::new(point, light_dir.normalize(), 1.0);
    let mut visible: f32 = 1.0;
    let mut total_dist: f32 = 0.0;
    let k: f32 = 32.0;

    for _ in 0 .. cfg.max_steps {
        let res = scene.sdf(&ray);
        let signed_radius = ray.sign * res.distance;

        if total_dist >= light_dist {
            break;
        }

        if signed_radius < Ray::MIN_DIST {
            visible = 0.0;
            break;
        }

        visible = visible.min((k * signed_radius) / total_dist);
        ray.advance(signed_radius);

        total_dist += signed_radius;
        if total_dist > Ray::MAX_DIST {
            break;
        }
    }

    clamp(visible, 0.0, 1.0)
}

/// Objects that a ray is within during refraction processing.
#[derive(Debug, Clone)]
struct Container {
    object: ShapeId,
    refractive_index: f32,
}

#[derive(Debug, Clone)]
struct Containers {
    containers: Vec<Container>,
}

impl Containers {
    fn new() -> Self {
        Containers {
            containers: Vec::new(),
        }
    }

    fn refractive_index(&self) -> f32 {
        self.containers
            .last()
            .map_or_else(|| 1.0, |container| container.refractive_index)
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
    fn process_hit(&mut self, object: ShapeId, refractive_index: f32) -> (bool, f32, f32) {
        let n1 = self.refractive_index();

        let mut leaving = false;

        // if the object is already in the set, find its index.
        let existing_ix = self.containers.iter().enumerate().find_map(|arg| {
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
            }
            None => self.containers.push(Container {
                object,
                refractive_index,
            }),
        }

        let n2 = self.refractive_index();

        (leaving, n1, n2)
    }
}

#[derive(Debug)]
struct Hit<'scene, 'cont> {
    object_space_point: Point3<f32>,
    world_space_point: Point3<f32>,
    normal: Vector3<f32>,
    reflectv: Vector3<f32>,
    eyev: Vector3<f32>,
    material: &'scene Material,
    pattern: &'scene Pattern,
    n1: f32,
    n2: f32,
    leaving: bool,
    containers: Cow<'cont, Containers>,
}

impl<'scene, 'cont> Hit<'scene, 'cont> {
    fn new<'a, 'b>(
        scene: &'a Scene,
        containers: &'b Containers,
        ray: &Ray,
        res: MarchResult,
    ) -> Hit<'a, 'b> {
        let material = scene.get_material(res.material);

        let mut containers = Cow::Borrowed(containers);

        let (leaving, n1, n2) = if material.transparent > 0.0 {
            containers
                .to_mut()
                .process_hit(res.object_id, material.refractive_index)
        } else {
            let val = containers.refractive_index();
            (false, val, val)
        };

        let normal = res.normal(|pt| scene.sdf(pt));

        Hit {
            object_space_point: res.object_space_point,
            world_space_point: res.final_ray.origin,
            normal,

            // this doesn't need to be computed here
            reflectv: ray::reflect(&ray.direction, &normal),

            // the direction towards the eye
            eyev: -ray.direction,

            material,
            pattern: scene.get_pattern(res.pattern),

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
        Ray::new(origin, self.reflectv, containers.determine_sign())
    }

    fn refraction_ray(&self, containers: &Containers) -> Option<Ray> {
        // the normal must point inside when the refraction ray originated within the object
        let refrac_normal = if self.leaving {
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
            let direction =
                (refrac_normal * (n_ratio * cos_i - cos_t) - self.eyev * n_ratio).normalize();

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
    let reflectance = schlick(
        &Vector3::new(0.0, 0.0, -1.0),
        &Vector3::new(0.0, 0.0, -1.0),
        1.0,
        1.5,
    );
    assert!(
        reflectance - 0.04 < 0.001,
        format!("{} != {}", reflectance, 0.04)
    );

    let reflectance = schlick(
        &Vector3::new(0.0, 0.0, -1.0),
        &Vector3::new(1.0, 0.0, 0.0),
        1.0,
        1.5,
    );
    assert!(
        reflectance - 1.0 < 0.001,
        format!("{} != {}", reflectance, 1.0)
    );
}
