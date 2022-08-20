use nalgebra::{Unit, Vector3};

mod canvas;
mod ray;
mod scene;

fn main() {
    let mut scene = scene::Scene::default();
    let plane = scene.plane(Unit::new_normalize(Vector3::new(0., 1., 0.)));
    let sphere = scene.sphere(2.);
    let root = scene.group(vec![plane, sphere]);

    let ray = ray::Ray::new(
        Vector3::new(0., 5., 5.),
        Unit::new_normalize(Vector3::new(0., -1., 1.)),
    );
    if let Some(res) = scene.march(0.01, 100., 200, root, ray) {
        // println!("hit! steps: {}", res.steps);
    } else {
        // println!("no hit :(");
    }

    let mut c = canvas::Canvas::new(2, 2);

    c.get_mut(0, 0).r = 1.;

    println!("{}", c.ppm());
}
