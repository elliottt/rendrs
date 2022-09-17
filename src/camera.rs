use std::sync::Arc;

use nalgebra::{Point2, Point3, Unit, Vector3};

use crate::canvas::Canvas;
use crate::ray::Ray;
use crate::transform::{ApplyTransform, Transform};

#[derive(Debug, Clone)]
pub struct CanvasInfo {
    /// The width in pixels of the canvas.
    pub width: u32,

    /// The height in pixels of the canvas.
    pub height: u32,
}

impl CanvasInfo {
    /// Create a new [`CanvasInfo`].
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn new_canvas(&self) -> Canvas {
        Canvas::new(self.width as u32, self.height as u32)
    }

    /// Compute the aspect ratio.
    pub fn aspect_ratio(&self) -> f32 {
        self.width_f32() / self.height_f32()
    }

    pub fn width_f32(&self) -> f32 {
        self.width as f32
    }

    pub fn height_f32(&self) -> f32 {
        self.height as f32
    }
}

#[derive(Debug, Clone)]
pub struct ProjectiveCamera {
    camera_to_world: Transform,
    camera_to_screen: Transform,
    raster_to_camera: Transform,
    screen_to_raster: Transform,
    raster_to_screen: Transform,
}

impl ProjectiveCamera {
    // TODO: support targeting pixels that aren't square, like ascii characters
    pub fn new(info: &CanvasInfo, camera_to_world: Transform, camera_to_screen: Transform) -> Self {
        let screen_to_raster = Transform::new()
            .scale(&Vector3::new(info.width_f32(), info.height_f32(), 1.))
            .scale(&Vector3::new(0.5, 0.5, 1.))
            .translate(&Vector3::new(1., 1., 0.));

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
    pub fn new(info: &CanvasInfo, camera_to_world: Transform, fov: f32) -> Self {
        let camera_to_screen = Transform::perspective(info.aspect_ratio(), fov, 1., 1000.);
        Self {
            camera: ProjectiveCamera::new(info, camera_to_world, camera_to_screen),
        }
    }
}

#[derive(Debug, Clone)]
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

pub trait Camera: std::marker::Send + std::marker::Sync {
    /// Given a [`Sample`], generate a ray.
    fn generate_ray(&self, sample: &Sample) -> Ray;
}

impl<C> Camera for Arc<C> where C: Camera + ?Sized {
    fn generate_ray(&self, sample: &Sample) -> Ray {
        self.as_ref().generate_ray(sample)
    }
}

impl Camera for PinholeCamera {
    fn generate_ray(&self, sample: &Sample) -> Ray {
        let canvas =
            Point3::new(sample.film.x, sample.film.y, 0.).apply(&self.camera.raster_to_camera);
        let camera = Unit::new_normalize(canvas - Point3::origin());

        let ray = Ray::new(Point3::origin(), camera);

        ray.invert(&self.camera.camera_to_world)
    }
}

#[test]
fn test_projective_camera() {
    let info = CanvasInfo::new(10., 10.);
    let camera = ProjectiveCamera::new(&info, Transform::new(), Transform::new());

    assert_eq!(
        Point3::origin(),
        Point3::new(5., 5., 0.).apply(&camera.raster_to_camera)
    );

    assert_eq!(
        Point3::new(-1., -1., 0.),
        Point3::new(0., 0., 0.).apply(&camera.raster_to_camera)
    );

    // (10, 10) is out of bounds for screen space, but represents the upper-right corner of the
    // (9,9) pixel.
    assert_eq!(
        Point3::new(1., 1., 0.),
        Point3::new(10., 10., 0.).apply(&camera.raster_to_camera)
    );
}

#[test]
fn test_pinhole_camera() {
    let t = Transform::new();
    let fov = std::f32::consts::FRAC_PI_2;
    let info = CanvasInfo::new(10., 10.);
    let camera = PinholeCamera::new(&info, t, fov);

    let ray = camera.generate_ray(&Sample::new(5., 5.));

    assert_eq!(Point3::new(0., 0., 0.), ray.position);
    assert_eq!(
        Unit::new_normalize(Vector3::new(0., 0., -1.)),
        ray.direction
    );
}
