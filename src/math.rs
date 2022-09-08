use nalgebra::{Unit, Vector3};

/// Reflect `vec` through `normal`.
pub fn reflect(vec: &Unit<Vector3<f32>>, normal: &Unit<Vector3<f32>>) -> Unit<Vector3<f32>> {
    Unit::new_unchecked(vec.as_ref() - normal.as_ref() * 2. * vec.dot(normal))
}

/// Clamp a value to the range.
#[inline]
pub fn clamp(lo: f32, hi: f32, val: f32) -> f32 {
    val.max(lo).min(hi)
}

#[inline]
pub fn mix(x: f32, y: f32, a: f32) -> f32 {
    x * (1.0 - a) + y * a
}
