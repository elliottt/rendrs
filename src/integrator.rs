use nalgebra::{Point3, Unit, Vector3};

use crate::{
    camera::{Camera, Sample},
    canvas::{Canvas, Color},
    lighting,
    ray::Ray,
    scene::{Distance, MarchConfig, MaterialId, NodeId, Scene},
};

pub fn render<I: Integrator>(canvas: &mut Canvas, scene: &Scene, root: NodeId, integrator: &mut I) {
    // TODO: pass in a sampling strategy
    for row in 0..canvas.height() {
        let y = row as f32 + 0.5;
        for col in 0..canvas.width() {
            let x = col as f32 + 0.5;

            let sample = Sample::new(x, y);
            *canvas.get_mut(col as usize, row as usize) =
                integrator.luminance(scene, root, &sample);
        }
    }
}

pub trait Integrator {
    fn luminance(&mut self, scene: &Scene, root: NodeId, sample: &Sample) -> Color;
}

pub struct Whitted<C: Camera> {
    camera: C,
    config: MarchConfig,
    max_reflections: usize,
}

impl<C: Camera> Whitted<C> {
    pub fn new(camera: C, config: MarchConfig, max_reflections: usize) -> Self {
        Self {
            camera,
            config,
            max_reflections,
        }
    }
}

impl<C: Camera> Integrator for Whitted<C> {
    fn luminance(&mut self, scene: &Scene, root: NodeId, sample: &Sample) -> Color {
        let hit = Hit::march(&self.config, scene, root, self.camera.generate_ray(sample));

        if hit.is_none() {
            let mut color = Color::black();
            for light in scene.lights.iter() {
                color += light.light_escape();
            }
            return color;
        }

        let hit = hit.unwrap();

        // return unlit magenta if there's no material for this object
        if hit.material.is_none() {
            return Color::hex(0xff00ff);
        }

        let material = scene.material(hit.material.unwrap());

        let eye = -hit.ray.direction;

        // TODO: compute emitted light for emissive objects

        let mut color = Color::black();
        for light in scene.lights.iter() {
            // TODO: check to see if the light is visible before computing the lighting
            color += lighting::phong(material, light, &hit.ray.position, &eye, &hit.normal);
        }

        // TODO: compute reflection contribution
        // TODO: compute refraction contribution

        color
    }
}

/// Information about a ray hit with scene geometry.
pub struct Hit {
    /// The closest node in the scene.
    pub node: NodeId,

    /// The intersection point in object space.
    pub object: Point3<f32>,

    /// The normal of the object at the hit, in world space.
    pub normal: Unit<Vector3<f32>>,

    /// The material for the object.
    pub material: Option<MaterialId>,

    /// The ray that caused the intersection.
    pub ray: Ray,

    /// The distance traveled to get to this point.
    pub distance: Distance,

    /// The number of steps taken.
    pub steps: u32,
}

impl Hit {
    /// March the ray until it hits something in the geometry or runs out of fuel.
    pub fn march(config: &MarchConfig, scene: &Scene, root: NodeId, mut ray: Ray) -> Option<Self> {
        let mut total_dist = Distance::default();

        let node = scene.node(root);

        for i in 0..config.max_steps {
            let result = node.sdf(scene, root, &ray.position);
            let radius = result.distance.0;

            if radius < config.min_dist {
                return Some(Self {
                    node: result.id,
                    object: result.object,
                    normal: result.normal,
                    material: result.material,
                    ray,
                    distance: total_dist,
                    steps: i,
                });
            }

            total_dist.0 += radius;

            if total_dist.0 > config.max_dist {
                break;
            }

            ray.step(radius);
        }

        None
    }
}
