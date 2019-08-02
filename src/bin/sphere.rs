
extern crate nalgebra;

use std::sync::Arc;

use nalgebra::{Point3,Vector3,Matrix4};

use rendrs::{
    camera::Camera,
    canvas::{Color},
    shapes::{Shape,PrimShape},
    scene::{Scene},
    pattern::{Pattern},
    material::{Light,Material},
    render::{render,write_canvas,ConfigBuilder},
};

pub fn main() {
    let mut scene = Scene::new();

    let mat = scene.add_material(Material::default());

    let blue = scene.add_pattern(Pattern::solid(Color::new(0.0, 0.0, 1.0)));
    let red = scene.add_pattern(Pattern::solid(Color::new(1.0, 0.0, 0.0)));

    let sphere = scene.add(Shape::PrimShape{ shape: PrimShape::Sphere });
    let xz_plane = scene.add(Shape::PrimShape{ shape: PrimShape::XZPlane });

    let white = scene.add_pattern(Pattern::solid(Color::white()));
    let black = scene.add_pattern(Pattern::solid(Color::black()));
    let striped = scene.add_pattern(Pattern::stripe(black, white));

    {
        let red_sphere = scene.add(Shape::material(red, mat, sphere));
        let blue_sphere = scene.add(Shape::material(blue, mat, sphere));
        let a = scene.add(Shape::translation(&Vector3::new(-1.0, 0.0, 0.0), red_sphere));
        let d = scene.add(Shape::uniform_scaling(2.0, blue_sphere));
        let b = scene.add(Shape::translation(&Vector3::new(1.0, 0.0, 0.0), d));
        let e = scene.add(Shape::translation(&Vector3::new(0.0, 1.0, -1.0), sphere));
        let s = scene.add(Shape::union(vec![a, b]));
        let root = scene.add(Shape::subtract(s, e));
        scene.add_root(root);
    }

    {
        let ground = scene.add(Shape::translation(&Vector3::new(0.0, -2.0, 0.0), xz_plane));
        let striped_ground = scene.add(Shape::material(striped, mat, ground));
        scene.add_root(striped_ground);
    }

    {
        let angle = 3.0 * (std::f32::consts::PI / 2.0);
        let axis = Vector3::new(1.0, 0.0, 0.0);
        let trans = Matrix4::new_rotation(axis * angle)
            .append_translation(&Vector3::new(0.0, 0.0, 10.0));
        let wall = scene.add(Shape::transform(&trans, xz_plane));
        let blue_wall = scene.add(Shape::material(striped, mat, wall));
        scene.add_root(blue_wall);
    }

    scene.add_light(Light{
        position: Point3::new(5.0, 10.0, -10.0),
        intensity: Color::white(),
    });

    // scene.add_light(Light{
    //     position: Point3::new(-3.0, 2.0, 0.0),
    //     intensity: Color::white(),
    // });

    let mut camera = Camera::new(1000, 1000, std::f32::consts::PI / 2.0);
    camera.set_transform(
        Matrix4::look_at_lh(
            &Point3::new(0.0, 0.0, -5.0),
            &Point3::origin(),
            &Vector3::new(0.0, 1.0, 0.0)
        )
    );

    let cfg = ConfigBuilder::default()
        .set_width(1000)
        .set_height(1000)
        .set_jobs(8)
        .set_max_steps(1000)
        .build();

    let recv = render(Arc::new(scene), Arc::new(camera), cfg.clone());
    write_canvas(cfg, recv).save("test.png");
}
