
extern crate nalgebra;

use nalgebra::{Point3,Vector3};

use rendrs::{canvas,ray::Ray,shapes::{Scene,Shape}};

pub fn main() {
    let mut c = canvas::Canvas::new(1000,1000);

    let mut scene = Scene::new();
    let sphere = scene.sphere();
    let a = scene.add(Shape::translation(&Vector3::new(-1.0, 0.0, 0.0), sphere.clone()));
    let d = scene.add(Shape::uniform_scaling(2.0, sphere));
    let b = scene.add(Shape::translation(&Vector3::new(1.0, 0.0, 0.0), d));
    let root = scene.add(Shape::union(vec![a, b]));

    let origin = Point3::new(0.0, 0.0, -5.0);

    // cast rays
    for y in 0 .. c.height {
        let oy = 10.0 * ((y as f32) / 1000.0) - 5.0;
        for x in 0 .. c.width {
            let ox = 10.0 * ((x as f32) / 1000.0) - 5.0;
            let target = Point3::new(ox, oy, 2.0);
            let direction = (target - origin).normalize();
            let pixel = c.get_mut(x,y).expect("Missing a pixel!");
            if let Some(res) = Ray::new(origin, direction).march(|pt| scene.sdf(&root, pt)) {
                let val = 1.0 - ((res.steps as f32) / (Ray::MAX_STEPS as f32)) * 10.0;
                pixel.set_r(val).set_g(val).set_b(val);
            } else {
                pixel.set_r(0.1).set_g(0.1).set_b(0.1);
            }
        }
    }

    c.save("test.png");
}
