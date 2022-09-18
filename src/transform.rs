use nalgebra::{Matrix4, Point3, Unit, Vector3};
use std::ops::Neg;

#[derive(Debug, Clone)]
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

    /// Construct the lhs look-at transform.
    pub fn look_at(eye: &Point3<f32>, target: &Point3<f32>, up: &Vector3<f32>) -> Self {
        let matrix = Matrix4::look_at_rh(eye, target, up);
        let inverse = matrix.try_inverse().unwrap();
        Self {
            matrix,
            inverse,
            scale_factor: 1.0,
        }
    }

    /// Construct a perspective transform.
    pub fn perspective(aspect: f32, fov: f32, znear: f32, zfar: f32) -> Self {
        let matrix = Matrix4::new_perspective(aspect, fov, znear, zfar);
        let inverse = matrix.try_inverse().unwrap();
        Self {
            matrix,
            inverse,
            scale_factor: 1.0,
        }
    }

    pub fn inverse(&self) -> Self {
        Self {
            matrix: self.inverse,
            inverse: self.matrix,
            scale_factor: 1.0 / self.scale_factor,
        }
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// Compose a translation with this transform.
    pub fn translate(mut self, vec: &Vector3<f32>) -> Self {
        self.matrix.prepend_translation_mut(vec);
        self.inverse.append_translation_mut(&vec.neg());
        self
    }

    /// Compose a uniform scaling with this transform.
    pub fn uniform_scale(mut self, amount: f32) -> Self {
        self.matrix.prepend_scaling_mut(amount);
        self.inverse.append_scaling_mut(1.0 / amount);
        self.scale_factor *= amount;
        self
    }

    /// Compose a non-uniform scaling with this transform.
    pub fn scale(mut self, vec: &Vector3<f32>) -> Self {
        self.matrix.prepend_nonuniform_scaling_mut(vec);

        let inv = Vector3::new(1. / vec.x, 1. / vec.y, 1. / vec.z);
        self.inverse.append_nonuniform_scaling_mut(&inv);

        // hmmm
        self.scale_factor *= vec.x.max(vec.y).max(vec.z);
        self
    }

    /// Compose an axis-angle rotation to the transform.
    pub fn rotate(mut self, axisangle: &Vector3<f32>) -> Self {
        self.matrix = self.matrix * Matrix4::new_rotation(axisangle.clone());
        self.inverse = Matrix4::new_rotation(axisangle.neg()) * self.inverse;
        self
    }
}

impl std::ops::Mul for &Transform {
    type Output = Transform;

    fn mul(self, other: Self) -> Self::Output {
        Self::Output {
            matrix: self.matrix * other.matrix,
            inverse: other.inverse * self.inverse,
            scale_factor: self.scale_factor * other.scale_factor,
        }
    }
}

impl std::ops::Mul<&Transform> for Transform {
    type Output = Transform;

    fn mul(self, other: &Self) -> Self::Output {
        Self::Output {
            matrix: self.matrix * other.matrix,
            inverse: other.inverse * self.inverse,
            scale_factor: self.scale_factor * other.scale_factor,
        }
    }
}

pub trait ApplyTransform: Sized {
    fn transform(&self, m: &Matrix4<f32>) -> Self;

    #[inline]
    fn apply(&self, t: &Transform) -> Self {
        self.transform(&t.matrix)
    }

    #[inline]
    fn invert(&self, t: &Transform) -> Self {
        self.transform(&t.inverse)
    }
}

impl ApplyTransform for Point3<f32> {
    #[inline]
    fn transform(&self, m: &Matrix4<f32>) -> Self {
        m.transform_point(self)
    }
}

impl ApplyTransform for Vector3<f32> {
    #[inline]
    fn transform(&self, m: &Matrix4<f32>) -> Self {
        m.transform_vector(self)
    }
}

impl<T: ApplyTransform> ApplyTransform for Unit<T> {
    #[inline]
    fn transform(&self, m: &Matrix4<f32>) -> Self {
        Unit::new_unchecked(self.as_ref().transform(m))
    }
}

#[test]
fn test_translate() {
    let t = Transform::new().translate(&Vector3::new(1., 0., 0.));
    let p = Point3::new(1., 0., 0.);
    assert_eq!(Point3::new(2., 0., 0.), p.apply(&t));
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
