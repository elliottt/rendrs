use std::ops::Neg;

use nalgebra::{Matrix4, Point3, Vector3};

pub struct Transform {
    matrix: Matrix4<f32>,
    inverse: Matrix4<f32>,
    scale_factor: f32,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            matrix: Matrix4::identity(),
            inverse: Matrix4::identity(),
            scale_factor: 1.0,
        }
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Append a translation to this transform.
    pub fn translate(mut self, vec: &Vector3<f32>) -> Self {
        self.matrix.append_translation_mut(vec);
        self.inverse.prepend_translation_mut(&vec.neg());
        self
    }

    /// Append a uniform scaling to this transform.
    pub fn uniform_scale(mut self, amount: f32) -> Self {
        self.matrix.append_scaling_mut(amount);
        self.inverse.prepend_scaling_mut(1.0 / amount);
        self.scale_factor *= amount;
        self
    }

    /// Append an axis-angle rotation to the transform.
    pub fn rotate(mut self, axisangle: &Vector3<f32>) -> Self {
        self.matrix = self.matrix * Matrix4::new_rotation(axisangle.clone());
        self.inverse = Matrix4::new_rotation(axisangle.neg()) * self.inverse;
        self
    }
}

trait ApplyTransform {
    fn apply(&self, transform: &Transform) -> Self;
    fn invert(&self, transform: &Transform) -> Self;
}

impl ApplyTransform for Point3<f32> {
    #[inline]
    fn apply(&self, transform: &Transform) -> Self {
        transform.matrix.transform_point(self)
    }

    #[inline]
    fn invert(&self, transform: &Transform) -> Self {
        transform.inverse.transform_point(self)
    }
}

impl ApplyTransform for Vector3<f32> {
    #[inline]
    fn apply(&self, transform: &Transform) -> Self {
        transform.matrix.transform_vector(self)
    }

    #[inline]
    fn invert(&self, transform: &Transform) -> Self {
        transform.inverse.transform_vector(self)
    }
}

#[test]
fn test_translate() {
    let t = Transform::new().translate(&Vector3::new(1., 0., 0.));
    let p = Point3::new(1., 0., 0.);
    assert_eq!(p, p.apply(&t).invert(&t));
}

#[test]
fn test_scaling() {
    let t = Transform::new().uniform_scale(10.0);
    let p = Point3::new(1., 0., 0.);
    assert_eq!(p, p.apply(&t).invert(&t));
}

#[test]
fn test_rotation() {
    let t = Transform::new().rotate(&Vector3::new(std::f32::consts::PI, 0., 0.));
    let p = Point3::new(0., 1., 0.);
    assert_eq!(p, p.apply(&t).invert(&t));
}

#[test]
fn test_composition() {
    let t = Transform::new()
        .uniform_scale(10.0)
        .translate(&Vector3::new(1., 0., 0.));
    let p = Point3::new(1., 0., 1.);
    assert_eq!(p, p.apply(&t).invert(&t));
}
