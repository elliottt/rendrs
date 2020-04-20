use nalgebra::{Point2, Vector2};

use crate::float::Float;

pub trait Filter {
    fn evaluate(&self, p: Point2<Float>) -> Float;
}

pub fn box_() -> Box<dyn Filter> {
    Box::new(BoxFilter)
}

struct BoxFilter;

impl Filter for BoxFilter {
    fn evaluate(&self, _: Point2<Float>) -> Float {
        1.0
    }
}

pub fn triangle(radius: Vector2<Float>) -> Box<dyn Filter> {
    Box::new(TriangleFilter { radius })
}

struct TriangleFilter {
    radius: Vector2<Float>,
}

impl Filter for TriangleFilter {
    fn evaluate(&self, p: Point2<Float>) -> Float {
        Float::max(0.0, self.radius.x - p.x.abs()) * Float::max(0.0, self.radius.y - p.y.abs())
    }
}
