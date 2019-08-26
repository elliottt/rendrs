
use nalgebra::Vector3;

/// Clamp `val` into the range defined by `lo` and `hi`.
pub fn clamp(val: f32, lo: f32, hi: f32) -> f32 {
    val.max(lo).min(hi)
}

pub fn mix(x: f32, y: f32, a: f32) -> f32 {
    x * (1.0 - a) + y * a
}

pub fn dot2(vec: &Vector3<f32>) -> f32 {
    vec.dot(&vec)
}
