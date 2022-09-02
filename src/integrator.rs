use nalgebra::{Point3, Vector3};

use crate::{camera::Camera, canvas::Canvas, ray::Ray, scene::{MarchConfig, NodeId, Scene}};

pub trait Integrator {
    fn render(&mut self, scene: &Scene);
}

pub struct Whitted<C: Camera> {
    pub canvas: Canvas,
    pub camera: C,
}

impl<C: Camera> Whitted<C> {
    pub fn new(canvas: Canvas, camera: C) -> Self {
        Self { canvas, camera }
    }
}

impl<C: Camera> Integrator for Whitted<C> {
    fn render(&mut self, scene: &Scene) {
        for y in 0..self.canvas.height() {
            let y = y as f32 + 0.5;
            for x in 0..self.canvas.width() {
                let x = x as f32 + 0.5;
            }
        }
    }
}

pub struct Hit {
    pub world: Point3<f32>,
    pub normal: Vector3<f32>,
}

impl Hit {

    pub fn try_new(config: &MarchConfig, scene: &Scene, root: NodeId, ray: Ray) -> Option<Self> {
        scene.march(config, root, ray);
        None
    }

}
