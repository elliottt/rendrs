use nalgebra::{Point3, Unit, Vector3};

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

    /// Translate the position to a vector.
    pub fn position_vector(&self) -> Vector3<f32> {
        Vector3::new(self.position.x, self.position.y, self.position.z)
    }
}
