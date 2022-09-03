use nalgebra::{Point3, Unit, Vector3};

use crate::{camera::Camera, canvas::Canvas, ray::Ray, scene::{Distance, MarchConfig, NodeId, Scene}};

pub trait Integrator {
    fn render(&mut self, scene: &Scene);
}

pub struct Whitted<C: Camera> {
    pub canvas: Canvas,
    pub camera: C,
}

impl<C: Camera> Whitted<C> {
    pub fn new(canvas: Canvas, camera: C) -> Self {
        Self { canvas, camera }
    }
}

impl<C: Camera> Integrator for Whitted<C> {
    fn render(&mut self, scene: &Scene) {
        for y in 0..self.canvas.height() {
            let y = y as f32 + 0.5;
            for x in 0..self.canvas.width() {
                let x = x as f32 + 0.5;
            }
        }
    }
}

/// Information about a ray hit with scene geometry.
pub struct Hit {
    /// The closest node in the scene.
    pub node: NodeId,

    /// The intersection point in object space.
    pub object: Point3<f32>,

    /// The intersection point in world space.
    pub world: Point3<f32>,

    /// The normal computed at the hit.
    pub normal: Unit<Vector3<f32>>,

    /// The distance traveled to get to this point.
    pub distance: Distance,

    /// The number of steps taken.
    pub steps: u32,
}

impl Hit {

    pub fn march(config: &MarchConfig, scene: &Scene, root: NodeId, mut ray: Ray) -> Option<Self> {
        let mut total_dist = Distance::default();

        let node = scene.node(root);

        for i in 0..config.max_steps {
            let result = node.sdf(scene, root, &ray);
            let radius = result.distance.0;

            if radius < config.min_dist {
                return Some(Self {
                    node: result.id,
                    object: ray.position.clone(),
                    normal: Unit::new_unchecked(Vector3::new(1.,0.,0.)),
                    world: ray.position,
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
