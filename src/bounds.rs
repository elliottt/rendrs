use nalgebra::{Point2, Point3};
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct Bounds2<T: Debug + Copy + PartialEq + PartialOrd + 'static> {
    pub min: Point2<T>,
    pub max: Point2<T>,
}

impl<T: Debug + Copy + PartialEq + PartialOrd> From<[T; 4]> for Bounds2<T> {
    fn from(ts: [T; 4]) -> Self {
        Bounds2 {
            min: Point2::new(ts[0], ts[1]),
            max: Point2::new(ts[2], ts[3]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bounds3<T: Debug + Copy + PartialEq + PartialOrd + 'static> {
    pub min: Point3<T>,
    pub max: Point3<T>,
}

impl<T: Debug + Copy + PartialEq + PartialOrd> From<[T; 6]> for Bounds3<T> {
    fn from(ts: [T; 6]) -> Self {
        Bounds3 {
            min: Point3::new(ts[0], ts[1], ts[2]),
            max: Point3::new(ts[3], ts[4], ts[5]),
        }
    }
}

pub trait Bounds {
    type Point;

    fn contains(&self, p: &Self::Point) -> bool;
}

impl<T: Debug + Copy + PartialEq + PartialOrd> Bounds for Bounds2<T> {
    type Point = Point2<T>;

    fn contains(&self, p: &Self::Point) -> bool {
        self.min.ge(p) && self.max.le(p)
    }
}

impl<T: Debug + Copy + PartialEq + PartialOrd> Bounds for Bounds3<T> {
    type Point = Point3<T>;

    fn contains(&self, p: &Self::Point) -> bool {
        self.min.ge(p) && self.max.le(p)
    }
}
