use nalgebra::{Point3, Unit, Vector3};

use crate::{
    canvas::Color,
    math,
    scene::{Light, Material, Scene},
};

pub fn phong(
    scene: &Scene,
    material: &Material,
    light: &Light,
    object_point: &Point3<f32>,
    point: &Point3<f32>,
    eyev: &Unit<Vector3<f32>>,
    normal: &Unit<Vector3<f32>>,
    in_shadow: bool,
) -> Color {
    let effective_color = scene
        .pattern(material.pattern)
        .color_at(scene, object_point, normal)
        * light.intensity();
    let ambient = material.ambient * &effective_color;

    // When the point is out of view of this light, we only integrate the ambient component of the
    // light.
    if in_shadow {
        return ambient;
    }

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
