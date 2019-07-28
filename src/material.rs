
use nalgebra::{Point3,Vector3};

use crate::ray::reflect;
use crate::canvas::Color;
use crate::pattern::{PatternId,Pattern};

pub struct Light {
    pub position: Point3<f32>,
    pub color: Color,
}

#[derive(Copy,Clone,Ord,PartialOrd,Eq,PartialEq,Debug)]
pub struct MaterialId(pub usize);

#[derive(Clone,Debug)]
pub struct Material {
    ambient: f32,
    diffuse: f32,
    specular: f32,
    shininess: f32,
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

    pub fn lighting<'a, Pats>(
        &'a self,
        light: &Light,
        patterns: Pats,
        pattern: &'a Pattern,
        object_space_point: &Point3<f32>,
        world_space_point: &Point3<f32>,
        dir: &Vector3<f32>,
        normal: &Vector3<f32>,
        visible: bool,
    ) -> Color
        where Pats: Fn(PatternId) -> &'a Pattern
    {
        let effectivec = pattern.color_at(patterns, object_space_point) * &light.color;
        let lightv = (light.position - world_space_point).normalize();
        let ambientc = &effectivec * self.ambient;
        let light_dot_normal = lightv.dot(normal);

        let diffuse_specular =
            if !visible || light_dot_normal < 0.0 {
                Color::black()
            } else {
                let specularc = {
                    let reflectv = reflect(& -lightv, normal);
                    let reflect_dot_eye = reflectv.dot(dir);
                    if reflect_dot_eye <= 0.0 {
                        Color::black()
                    } else {
                        let factor = reflect_dot_eye.powf(self.shininess);
                        &light.color * (self.specular * factor)
                    }
                };

                let diffusec = &effectivec * (self.diffuse * light_dot_normal);
                &diffusec + &specularc
            };

        &ambientc + &diffuse_specular
    }
}

#[test]
fn test_lighting() {
    use crate::pattern::Patterns;

    let pats = Patterns::new();
    let lookup = |patid| pats.get_pattern(patid);
    let white = Pattern::solid(Color::white());
    let m = Material::default();
    let pos = Point3::origin();

    {
        let eyev = Vector3::new(0.0, 0.0, -1.0);
        let normalv = Vector3::new(0.0, 0.0, -1.0);
        let light = Light{
            position: Point3::new(0.0, 0.0, -10.0),
            color: Color::new(1.0, 1.0, 1.0)
        };
        let res = m.lighting(&light, lookup, &white, &pos, &pos, &eyev, &normalv, true);
        assert_eq!(res.r(), 1.9);
        assert_eq!(res.g(), 1.9);
        assert_eq!(res.b(), 1.9);

        let res = m.lighting(&light, lookup, &white, &pos, &pos, &eyev, &normalv, false);
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
            color: Color::new(1.0, 1.0, 1.0)
        };
        let res = m.lighting(&light, lookup, &white, &pos, &pos, &eyev, &normalv, true);
        assert_eq!(res.r(), 1.0);
        assert_eq!(res.g(), 1.0);
        assert_eq!(res.b(), 1.0);
    }
}
