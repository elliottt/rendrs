use crate::float::Float;
use crate::ray::Ray;

pub struct Shape {
    sdf: Box<dyn Fn(&Ray) -> Float>,
}

impl Shape {
    pub fn sphere(radius: Float) -> Self {
        let sdf = move |ray: &Ray| ray.pos.coords.magnitude() - radius;
        Shape { sdf: Box::new(sdf) }
    }

    pub fn sdf(&self, ray: &Ray) -> Float {
        (self.sdf)(ray)
    }
}
