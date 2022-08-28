use nalgebra::{Point2, Point3, Unit, Vector3};

use crate::ray::Ray;
use crate::transform::{ApplyTransform, Transform};

#[derive(Debug, Clone)]
pub struct ProjectiveCamera {
    camera_to_world: Transform,
    camera_to_screen: Transform,
    raster_to_camera: Transform,
    screen_to_raster: Transform,
    raster_to_screen: Transform,
}

impl ProjectiveCamera {
    pub fn new(
        width: u32,
        height: u32,
        camera_to_world: Transform,
        camera_to_screen: Transform,
    ) -> Self {
        let scale = &Vector3::new(1. / (width as f32), 1. / (height as f32), 1.);
        let screen_to_raster = Transform::new().scale(&scale);

        let raster_to_screen = screen_to_raster.inverse();
        let raster_to_camera = camera_to_screen.inverse() * &raster_to_screen;

        Self {
            camera_to_world,
            camera_to_screen,
            raster_to_camera,
            screen_to_raster,
            raster_to_screen,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PinholeCamera {
    camera: ProjectiveCamera,
}

impl PinholeCamera {
    pub fn new(width: u32, height: u32, camera_to_world: Transform, fov: f32) -> Self {
        let aspect = width as f32 / height as f32;
        let camera_to_screen = Transform::perspective(aspect, fov, 1e-2, 1000.);
        Self {
            camera: ProjectiveCamera::new(width, height, camera_to_world, camera_to_screen),
        }
    }
}

pub struct Sample {
    /// The point on the film where the ray originates.
    pub film: Point2<f32>,
}

impl Sample {
    pub fn new(fx: f32, fy: f32) -> Self {
        Self {
            film: Point2::new(fx, fy),
        }
    }
}

pub trait Camera {
    /// Given a [`CameraSample`], generate a ray.
    fn generate_ray(&self, sample: Sample) -> Ray;
}

impl Camera for PinholeCamera {
    fn generate_ray(&self, sample: Sample) -> Ray {
        let canvas = Vector3::new(sample.film.x, sample.film.y, 0.);
        let camera = Unit::new_normalize(canvas.apply(&self.camera.raster_to_camera));

        Ray::new(Point3::origin(), camera).apply(&self.camera.camera_to_world)
    }
}
