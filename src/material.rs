
use nalgebra::Vector3;

use crate::float::Float;

pub struct Material {
    pub emissive: Vector3<Float>,
    pub albedo: Vector3<Float>,
}
