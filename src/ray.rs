use nalgebra::{Matrix4, Point3, Vector3};

use crate::float::Float;

pub struct Ray {
    pub pos: Point3<Float>,
    pub dir: Vector3<Float>,
}

impl Ray {
    pub fn new(pos: Point3<Float>, dir: Vector3<Float>) -> Self {
        Ray { pos, dir }
    }

    /// Update the position of the ray by advancing `dist` along `self.dir`.
    pub fn move_by(&mut self, dist: Float) {
        self.pos += self.dir * dist;
    }

    pub fn transform(&self, matrix: &Matrix4<Float>) -> Self {
        Self::new(
            matrix.transform_point(&self.pos),
            matrix.transform_vector(&self.dir),
        )
    }
}
