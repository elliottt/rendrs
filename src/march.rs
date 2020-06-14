use nalgebra::Vector3;

use crate::float::Float;
use crate::ray::Ray;
use crate::scene::{MaterialRef, SDFResult, Scene};

pub struct MarchConfig {
    pub max_steps: usize,
    pub min_dist: Float,
    pub max_dist: Float,
}

impl Default for MarchConfig {
    fn default() -> Self {
        MarchConfig {
            max_steps: 100,
            min_dist: 0.01,
            max_dist: 100.0,
        }
    }
}

pub struct MarchResult {
    pub sign: Float,
    pub final_ray: Ray,
    pub distance: Float,
    pub material: MaterialRef,
    normal: Option<Vector3<Float>>,
}

impl MarchResult {
    /// Fetch the normal vector at the point of the intersection.
    pub fn normal(&mut self, scene: &Scene) -> &Vector3<Float> {
        if let Some(ref normal) = self.normal {
            normal
        } else {
            let mut ray = self.final_ray.clone();
            let mut result = SDFResult::default();
            let root = scene.get_root().expect("No root present in the scene");

            scene.sdf(&mut result, &ray, root);
            let dist = result.distance;
            let offset = Vector3::new(0.0001, 0.0, 0.0);

            let mut step = |off| {
                ray.pos = self.final_ray.pos - off;
                result.reset();
                scene.sdf(&mut result, &ray, root);
                dist - result.distance
            };

            let dx = step(offset.xyy());
            let dy = step(offset.yxy());
            let dz = step(offset.yyx());

            self.normal = Some(Vector3::new(dx, dy, dz).normalize());
            self.normal.as_ref().unwrap()
        }
    }
}

pub fn march(
    config: &MarchConfig,
    scene: &Scene,
    sign: Float,
    origin: &Ray,
) -> Option<MarchResult> {
    if let Some(root) = scene.get_root() {
        let mut result = SDFResult::default();
        let mut distance = 0.;
        let mut pos = origin.clone();
        for _ in 0..config.max_steps {
            result.reset();
            scene.sdf(&mut result, &pos, root);
            result.distance *= sign;

            pos = origin.move_by(distance);
            distance += result.distance;

            if result.distance <= config.min_dist {
                return Some(MarchResult {
                    sign,
                    final_ray: pos,
                    distance,
                    material: result.material,
                    normal: None,
                });
            }

            if distance >= config.max_dist {
                break;
            }
        }
    }

    None
}
