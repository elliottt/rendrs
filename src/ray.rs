use nalgebra::{Matrix4, Point3, Vector3};

use crate::float::Float;

#[derive(Debug, Clone)]
pub struct Ray {
    pub pos: Point3<Float>,
    pub dir: Vector3<Float>,
}

impl Ray {
    pub fn new(pos: Point3<Float>, dir: Vector3<Float>) -> Self {
        Ray { pos, dir }
    }

    /// Update the position of the ray by advancing `dist` along `self.dir`.
    pub fn move_by_mut(&mut self, dist: Float) {
        self.pos += self.dir * dist;
    }

    /// Generate a new ray whose position is advanced by `dist` along `self.dir`.
    pub fn move_by(&self, dist: Float) -> Self {
        Ray {
            pos: self.pos + self.dir * dist,
            dir: self.dir,
        }
    }

    pub fn transform(&self, matrix: &Matrix4<Float>) -> Self {
        Self::new(
            matrix.transform_point(&self.pos),
            matrix.transform_vector(&self.dir),
        )
    }
}
