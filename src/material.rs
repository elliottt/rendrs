
use nalgebra::{Point3,Vector3};

use crate::ray::reflect;
use crate::canvas::Color;

pub struct Light {
    pub position: Point3<f32>,
    pub intensity: Color,
}

#[derive(Copy,Clone,Ord,PartialOrd,Eq,PartialEq,Debug)]
pub struct MaterialId(usize);

#[derive(Debug)]
pub struct Materials {
    materials: Vec::<Material>,
}

impl Materials {
    pub fn new() -> Self {
        Materials{ materials: Vec::with_capacity(10) }
    }

    pub fn add_material(&mut self, mat: Material) -> MaterialId {
        self.materials.push(mat);
        MaterialId(self.materials.len() - 1)
    }

    pub fn get_material(&self, mid: MaterialId) -> &Material {
        unsafe { self.materials.get_unchecked(mid.0) }
    }
}

#[derive(Clone,Debug)]
pub struct Material {
    pub ambient: f32,
    pub diffuse: f32,
    pub specular: f32,
    pub shininess: f32,
}

impl Default for Material {
    fn default() -> Self {
        Material{
            ambient: 0.1,
            diffuse: 0.9,
            specular: 0.9,
            shininess: 200.0,
        }
    }
}

impl Material {
    pub fn new(
        ambient: f32,
        diffuse: f32,
        specular: f32,
        shininess: f32,
    ) -> Self {
        Material{ ambient, diffuse, specular, shininess }
    }

    pub fn set_shininess(mut self, shininess: f32) -> Self {
        self.shininess = shininess;
        self
    }

    pub fn lighting(
        &self,
        light: &Light,
        obj_color: &Color,
        world_space_point: &Point3<f32>,
        eyev: &Vector3<f32>,
        normal: &Vector3<f32>,
        light_visible: bool,
    ) -> Color
    {
        let effectivec = obj_color * &light.intensity;

        let mut color = &effectivec * self.ambient;

        if light_visible {
            // the direction to the light
            let lightv = (light.position - world_space_point).normalize();
            let light_dot_normal = lightv.dot(normal);

            if light_dot_normal > 0.0 {
                // add in the diffuse part
                color += &effectivec * (self.diffuse * light_dot_normal);

                let reflectv = reflect(& -lightv, normal);
                let reflect_dot_eye = reflectv.dot(eyev);

                if reflect_dot_eye > 0.0 {
                    let factor = reflect_dot_eye.powf(self.shininess);
                    color += &light.intensity * (self.specular * factor);
                }
            }
        }

        color
    }
}

#[test]
fn test_lighting() {
    let white = Color::white();
    let m = Material::default();
    let pos = Point3::origin();

    {
        let eyev = Vector3::new(0.0, 0.0, -1.0);
        let normalv = Vector3::new(0.0, 0.0, -1.0);
        let light = Light{
            position: Point3::new(0.0, 0.0, -10.0),
            intensity: Color::new(1.0, 1.0, 1.0)
        };
        let res = m.lighting(&light, &white, &pos, &eyev, &normalv, true);
        assert_eq!(res.r(), 1.9);
        assert_eq!(res.g(), 1.9);
        assert_eq!(res.b(), 1.9);

        let res = m.lighting(&light, &white, &pos, &eyev, &normalv, false);
        assert_eq!(res.r(), 0.1);
        assert_eq!(res.g(), 0.1);
        assert_eq!(res.b(), 0.1);
    }

    {
        let s2d2 = f32::sqrt(2.0) / 2.0;
        let eyev = Vector3::new(0.0, s2d2, -s2d2);
        let normalv = Vector3::new(0.0, 0.0, -1.0);
        let light = Light{
            position: Point3::new(0.0, 0.0, -10.0),
            intensity: Color::new(1.0, 1.0, 1.0)
        };
        let mut res = m.lighting(&light, &white, &pos, &eyev, &normalv, true);
        assert_eq!(res.r(), 1.0);
        assert_eq!(res.g(), 1.0);
        assert_eq!(res.b(), 1.0);

        let eyev2 = Vector3::new(0.0, -s2d2, -s2d2);
        let light2 = Light{
            position: Point3::new(0.0, 10.0, -10.0),
            intensity: Color::new(1.0, 1.0, 1.0),
        };
        res = m.lighting(&light2, &white, &pos, &eyev2, &normalv, true);
        assert_eq!(res.r(), 1.6363853);
        assert_eq!(res.g(), 1.6363853);
        assert_eq!(res.b(), 1.6363853);
    }
}
