use nalgebra::Vector3;

use crate::float::Float;
use crate::ray::Ray;

#[derive(Default)]
pub struct ShapeStorage {
    shapes: Vec<Shape>,
}

impl ShapeStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, shape: Shape) -> ShapeRef {
        let index = self.shapes.len();
        self.shapes.push(shape);
        ShapeRef { index }
    }

    pub fn get(&self, ShapeRef { index }: ShapeRef) -> &Shape {
        unsafe { self.shapes.get_unchecked(index) }
    }
}

pub struct ShapeRef {
    index: usize,
}

pub struct PrimShape {
    sdf: Box<dyn Fn(&Ray) -> Float>,
}

impl PrimShape {
    pub fn sphere(radius: Float) -> Self {
        let sdf = move |ray: &Ray| ray.pos.coords.magnitude() - radius;

        PrimShape { sdf: Box::new(sdf) }
    }

    pub fn sdf(&self, ray: &Ray) -> Float {
        (self.sdf)(ray)
    }
}

pub enum Shape {
    Prim { prim: PrimShape },
}

impl Shape {
    pub fn sdf(&self, ray: &Ray) -> Float {
        match self {
            Shape::Prim { prim } => (prim.sdf)(ray),
        }
    }
}
