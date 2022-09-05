use nalgebra::{Point3, Unit, Vector3};

use crate::{
    canvas::Color,
    math,
    scene::{Light, Material},
};

pub fn phong(
    material: &Material,
    light: &Light,
    point: &Point3<f32>,
    eyev: &Unit<Vector3<f32>>,
    normal: &Unit<Vector3<f32>>,
) -> Color {
    let effective_color = &material.pattern * light.intensity();
    let ambient = material.ambient * &effective_color;

    let diffuse_specular = match light {
        Light::Diffuse { .. } => Color::black(),
        Light::Point { position, color } => {
            // direction to the light
            let lightv = Unit::new_normalize(position - point);

            let light_dot_normal = lightv.dot(normal);

            if light_dot_normal < 0. {
                Color::black()
            } else {
                let diffuse = effective_color * material.diffuse * light_dot_normal;

                // direction to the eye
                if material.specular > 0. {
                    let reflectv = math::reflect(&(-lightv), normal);
                    let reflect_dot_eye = reflectv.dot(eyev);
                    let specular = if reflect_dot_eye <= 0. {
                        Color::black()
                    } else {
                        let factor = reflect_dot_eye.powf(material.shininess);
                        color * material.specular * factor
                    };
                    diffuse + specular
                } else {
                    diffuse
                }
            }
        }
    };

    ambient + diffuse_specular
}
