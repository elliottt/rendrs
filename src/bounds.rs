use nalgebra::Point2;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Bounds2<T: Debug + Copy + PartialEq + 'static> {
    pub min: Point2<T>,
    pub max: Point2<T>,
}

impl<T: Debug + Copy + PartialEq> From<[T; 4]> for Bounds2<T> {
    fn from(ts: [T; 4]) -> Self {
        Bounds2 {
            min: Point2::new(ts[0], ts[1]),
            max: Point2::new(ts[2], ts[3]),
        }
    }
}
