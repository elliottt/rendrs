
use nalgebra::{Point3,Matrix4};

use crate::ray::Ray;

pub struct Camera {
    width_px: usize,
    height_px: usize,
    fov: f32,
    transform: Matrix4<f32>,
    inv_transform: Matrix4<f32>,
    half_width: f32,
    half_height: f32,
    pixel_size: f32,
}

impl Camera {
    pub fn new(width_px: usize, height_px: usize, fov: f32) -> Self {
        let mut camera = Camera{
            width_px,
            height_px,
            fov,
            transform: Matrix4::identity(),
            inv_transform: Matrix4::identity(),
            half_width: 0.0,
            half_height: 0.0,
            pixel_size: 0.0,
        };

        camera.update_cache();

        camera
    }

    fn update_cache(&mut self) {
        let half_view = f32::tan(self.fov / 2.0);
        let aspect = (self.width_px as f32) / (self.height_px as f32);
        if aspect >= 1.0 {
            self.half_width = half_view;
            self.half_height = half_view / aspect;
        } else {
            self.half_width = half_view * aspect;
            self.half_height = half_view;
        }
        self.pixel_size = (self.half_width * 2.0) / (self.height_px as f32);
    }

    /// Given a pixel position of the output, generate a ray. The convention for pixel coordinates
    /// is that top-left is (0,0).
    pub fn ray_for_pixel(&self, px: usize, py: usize) -> Ray {
        let xoff = (px as f32 + 0.5) * self.pixel_size;
        let yoff = (py as f32 + 0.5) * self.pixel_size;

        let world_x = xoff - self.half_width;
        let world_y = self.half_height - yoff;

        let pixel = self.inv_transform.transform_point(&Point3::new(world_x, world_y, 1.0));
        let origin = self.inv_transform.transform_point(&Point3::origin());
        let dir = (pixel - origin).normalize();

        Ray::new(origin, dir)
    }

    /// Set the view transformation.
    pub fn set_transform(&mut self, transform: Matrix4<f32>) {
        self.transform = transform;
        self.inv_transform = transform.try_inverse().expect("Unable to invert transform!");
    }
}

#[test]
fn test_ray_for_pixel() {
    use nalgebra::{Vector3};

    let mut camera = Camera::new(11, 11, std::f32::consts::PI / 2.0);
    let eye = Point3::new(0.0, 0.0, -1.0);
    camera.set_transform(Matrix4::look_at_lh(
        &eye,
        &Point3::origin(),
        &Vector3::new(0.0, 1.0, 0.0),
    ));

    {
        let ray = camera.ray_for_pixel(5,5);
        assert_eq!(ray.origin, eye);
        assert_eq!(ray.direction, Vector3::new(0.0, 0.0, 1.0));
    };

    {
        let ray = camera.ray_for_pixel(0,0);
        assert_eq!(ray.origin, eye);
        assert!(ray.direction.x < 0.0);
        assert!(ray.direction.y > 0.0);
    };

    {
        let ray = camera.ray_for_pixel(10,10);
        assert_eq!(ray.origin, eye);
        assert!(ray.direction.x > 0.0);
        assert!(ray.direction.y < 0.0);
    };
}
