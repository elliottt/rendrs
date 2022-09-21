use nalgebra::{Matrix4, Point3, Unit, Vector3};

use crate::{math, transform::ApplyTransform};

#[derive(Debug, Clone)]
pub struct Ray {
    pub position: Point3<f32>,
    pub direction: Unit<Vector3<f32>>,

    /// Used when testing intersection with a bounding box.
    pub inv_direction: Point3<f32>,
}

impl Ray {
    /// Construct a new ray.
    pub fn new(position: Point3<f32>, direction: Unit<Vector3<f32>>) -> Ray {
        let inv_direction = Point3::new(
            if direction.x != 0.0 {
                1.0 / direction.x
            } else {
                std::f32::INFINITY
            },
            if direction.y != 0.0 {
                1.0 / direction.y
            } else {
                std::f32::INFINITY
            },
            if direction.z != 0.0 {
                1.0 / direction.z
            } else {
                std::f32::INFINITY
            },
        );
        Ray {
            position,
            direction,
            inv_direction,
        }
    }

    /// Move the position of the ray along `direction` by `amount`.
    pub fn step(&mut self, amount: f32) {
        self.position += self.direction.scale(amount);
    }

    /// Construct a new ray reflected through a normal.
    pub fn reflect(&self, normal: &Unit<Vector3<f32>>) -> Self {
        Self::new(self.position, math::reflect(&self.direction, normal))
    }
}

impl ApplyTransform for Ray {
    #[inline]
    fn transform(&self, m: &Matrix4<f32>) -> Self {
        Ray::new(self.position.transform(m), self.direction.transform(m))
    }
}
