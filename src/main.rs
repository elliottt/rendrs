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
    let torus = scene.torus(0.5, 0.1);
    let plane = scene.plane(Unit::new_normalize(Vector3::new(0., 1., 0.)));
    // let root = scene.group(vec![plane, sphere]);
    // let root = scene.group(vec![plane, torus]);
    // let root = scene.group(vec![sphere]);
    // let root = scene.group(vec![plane]);
    let root = scene.group(vec![torus]);
    // let root = scene.group(vec![torus, plane]);

    let info = camera::CanvasInfo::new(80., 24.);
    // let info = camera::CanvasInfo::new(512., 512.);

    let camera = camera::PinholeCamera::new(
        &info,
        // transform::Transform::new().translate(&Vector3::new(0., 0.1, -3.)),
        transform::Transform::look_at(
            &Point3::new(0., 0.3, 1.),
            &Point3::new(0., 0., 0.),
            &Vector3::new(0., 1., 0.),
        ),
        std::f32::consts::FRAC_PI_3,
    );

    let mut c = info.new_canvas();
    let mut whitted = integrator::Whitted::new(camera, scene::MarchConfig::default(), 10);
    integrator::render(&mut c, &scene, root, &mut whitted);
    println!("{}", c.to_ascii());

    image::save_buffer(
        "test.jpg",
        &c.data(),
        c.width(),
        c.height(),
        image::ColorType::Rgb8,
    )
    .unwrap();
}
