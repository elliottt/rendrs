
extern crate nalgebra;

use nalgebra::{Point3,Vector3,Matrix4};

use rendrs::{
    camera::Camera,
    canvas::{Canvas,Color},
    shapes::{Light,Scene,Shape,Material},
};

pub fn main() {
    let mut c = Canvas::new(1000,1000);

    let mut scene = Scene::new();
    let blue = scene.add_material(Material::Phong{
        color: Color::new(0.0, 0.0, 1.0),
        ambient: 0.1,
        diffuse: 0.9,
        specular: 0.9,
        shininess: 200.0,
    });
    let red = scene.add_material(Material::Phong{
        color: Color::new(1.0, 0.0, 0.0),
        ambient: 0.1,
        diffuse: 0.9,
        specular: 0.9,
        shininess: 200.0,
    });

    let sphere = scene.sphere();
    let red_sphere = scene.add(Shape::material(red, sphere));
    let blue_sphere = scene.add(Shape::material(blue, sphere));
    let a = scene.add(Shape::translation(&Vector3::new(-1.0, 0.0, 0.0), red_sphere));
    let d = scene.add(Shape::uniform_scaling(2.0, blue_sphere));
    let b = scene.add(Shape::translation(&Vector3::new(1.0, 0.0, 0.0), d));
    let e = scene.add(Shape::translation(&Vector3::new(0.0, 1.0, -1.0), sphere));
    let s = scene.add(Shape::union(vec![a, b]));
    let root = scene.add(Shape::subtract(s, e));

    scene.add_light(Light{
        position: Point3::new(0.0, -5.0, -10.0),
        color: Color::white(),
    });

    let mut camera = Camera::new(1000, 1000, std::f32::consts::PI / 2.0);
    camera.set_transform(
        Matrix4::look_at_lh(
            &Point3::new(0.0, 0.0, -5.0),
            &Point3::origin(),
            &Vector3::new(0.0, 1.0, 0.0)
        )
    );

    // cast rays
    for y in 0 .. c.height {
        for x in 0 .. c.width {
            let ray = camera.ray_for_pixel(x, y);
            let pixel = c.get_mut(x,y).expect("Missing a pixel!");
            if let Some(res) = ray.march(|pt| scene.sdf(&root, pt)) {
                let mat = scene.get_material(res.material);
                let normal = res.normal(|pt| scene.sdf(&root, pt));

                for light in scene.iter_lights() {
                    // not correct, but what do you do
                    *pixel = mat.lighting(light, &res.point, &ray.direction, &normal);
                }
            } else {
                pixel.set_r(0.1).set_g(0.1).set_b(0.1);
            }
        }
    }

    c.save("test.png");
}
