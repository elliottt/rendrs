use nalgebra::{Point3, Unit, Vector3};

use crate::math;

#[derive(Debug, Clone)]
pub struct Ray {
    pub position: Point3<f32>,
    pub direction: Unit<Vector3<f32>>,
}

impl Ray {
    /// Construct a new ray.
    pub fn new(position: Point3<f32>, direction: Unit<Vector3<f32>>) -> Ray {
        Ray {
            position,
            direction,
        }
    }

    /// Move the position of the ray along `direction` by `amount`.
    pub fn step(&mut self, amount: f32) {
        self.position += self.direction.scale(amount);
    }

    pub fn reflect(&self, normal: &Unit<Vector3<f32>>) -> Ray {
        Ray {
            position: self.position.clone(),
            direction: math::reflect(&self.direction, normal),
        }
    }
}

#[test]
fn test_reflect() {
    let ray = Ray::new(
        Point3::origin(),
        Unit::new_unchecked(Vector3::new(0., 0., 1.)),
    );
    let next = ray.reflect(&Unit::new_unchecked(Vector3::new(0., 0., -1.)));
    assert_eq!(
        Unit::new_unchecked(Vector3::new(0., 0., -1.)),
        next.direction
    );
}
