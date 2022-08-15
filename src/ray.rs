use nalgebra::{Unit, Vector3};

#[derive(Debug)]
pub struct Ray {
    pub position: Vector3<f32>,
    pub direction: Unit<Vector3<f32>>,
}

impl Ray {
    /// Construct a new ray.
    pub fn new(position: Vector3<f32>, direction: Unit<Vector3<f32>>) -> Ray {
        Ray {
            position,
            direction,
        }
    }

    /// Move the position of the ray along `direction` by `amount`.
    pub fn step(&mut self, amount: f32) {
        self.position += self.direction.scale(amount);
    }
}
