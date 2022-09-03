use nalgebra::{Point3, Unit, Vector3};

use crate::camera::Camera;

mod camera;
mod canvas;
mod integrator;
mod ray;
mod scene;
mod transform;

fn main() {
    let mut scene = scene::Scene::default();
    let sphere = scene.sphere(1.);
    let plane = scene.plane(Unit::new_normalize(Vector3::new(0., 1., 0.)));
    let root = scene.group(vec![plane, sphere]);
    // let root = scene.group(vec![sphere]);

    let info = camera::CanvasInfo::new(80., 24.).with_pixel_aspect_ratio(0.7);

    let camera = camera::PinholeCamera::new(
        &info,
        // transform::Transform::new().translate(&Vector3::new(0., 0.1, 1.5)),
        transform::Transform::look_at(
            &Point3::new(0., 2., 2.),
            &Point3::new(0., 0., 0.),
            &Vector3::new(0., 1., 0.),
        ),
        std::f32::consts::FRAC_PI_3,
    );

    let mut c = canvas::Canvas::new(80, 24);

    let config = scene::MarchConfig::default();
    for row in 0..c.height() {
        for col in 0..c.width() {
            let cx = col as f32 + 0.5;
            let rx = row as f32 + 0.5;
            let ray = camera.generate_ray(camera::Sample::new(cx, rx));

            let pixel = c.get_mut(col as usize, row as usize);
            if let Some(res) = integrator::Hit::march(&config, &scene, root, ray.clone()) {
                let val = (res.distance.0 / 20.0).min(1.0);
                pixel.r = val;
                pixel.g = val;
                pixel.b = val;
            } else {
                pixel.r = 1.;
                pixel.g = 1.;
                pixel.b = 1.;
            }
        }
    }

    // image::save_buffer("test.png", &c.data(), c.width(), c.height(), image::ColorType::Rgb8).unwrap();
    println!("{}", c.to_ascii());

    integrator::Whitted::new(c, camera);
}
