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

pub trait Mix {
    type Output;

    fn mix(self, b: Self, t: f32) -> Self::Output;
}

impl Mix for f32 {
    type Output = f32;

    #[inline]
    fn mix(self, y: f32, t: f32) -> f32 {
        self * (1.0 - t) + y * t
    }
}

#[inline]
pub fn deg_to_rad(deg: f32) -> f32 {
    (deg / 180.) * std::f32::consts::PI
}

#[test]
fn test_deg_to_rad() {
    assert_eq!(std::f32::consts::PI, deg_to_rad(180.));
}
