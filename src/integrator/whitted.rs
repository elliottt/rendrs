use nalgebra::Unit;
use std::borrow::Cow;

use crate::{
    camera::{Camera, Sample},
    canvas::Color,
    integrator::{Containers, Hit, Integrator, IntegratorBuilder},
    math,
    ray::Ray,
    scene::{Light, MarchConfig, Material, NodeId, Scene},
};

pub struct WhittedBuilder<C> {
    camera: C,
    config: MarchConfig,
    max_reflections: u32,
}

impl<C> WhittedBuilder<C> {
    pub fn new(camera: C, config: MarchConfig, max_reflections: u32) -> Self {
        Self {
            camera,
            config,
            max_reflections,
        }
    }
}

impl<C: Camera + Clone + 'static> IntegratorBuilder for WhittedBuilder<C> {
    fn build(&self) -> Box<dyn Integrator> {
        Box::new(Whitted::new(
            self.camera.clone(),
            self.config.clone(),
            self.max_reflections,
        ))
    }
}

pub struct Whitted<C> {
    camera: C,
    config: MarchConfig,
    max_reflections: u32,
}

impl<C> Whitted<C> {
    pub fn new(camera: C, config: MarchConfig, max_reflections: u32) -> Self {
        Self {
            camera,
            config,
            max_reflections,
        }
    }

    /// Determine the color that would result from a ray intersection with the scene.
    fn color_for_ray<'a>(
        &mut self,
        scene: &Scene,
        root: NodeId,
        containers: Cow<'a, Containers>,
        ray: Ray,
        reflection: u32,
    ) -> Color {
        let mut color = Color::black();

        if reflection >= self.max_reflections {
            return color;
        }

        let Some(mut hit) = Hit::march(&self.config, scene, root, ray, !containers.is_empty())
        else {
            for light in scene.lights.iter() {
                color += light.light_escape();
            }
            return color;
        };

        // return unlit magenta if there's no material for this object
        let Some(material) = hit.material else {
            return Color::hex(0xff00ff);
        };

        match scene.material(material) {
            &Material::Phong {
                pattern,
                ambient,
                diffuse,
                specular,
                shininess,
                reflective,
                transparent,
                refractive_index,
            } => {
                let eyev = -hit.ray.direction;

                let base_color = scene
                    .pattern(pattern)
                    .color_at(scene, &hit.object, &hit.normal);

                let mut surface = Color::black();

                for light in scene.lights.iter() {
                    let effective_color = &base_color * light.intensity();
                    surface += ambient * &effective_color;

                    // When the point is out of view of this light, we only integrate the ambient component of the
                    // light.
                    if light.position().map_or(false, |light| {
                        hit.in_shadow(&self.config, scene, root, &light)
                    }) {
                        continue;
                    }

                    let diffuse_specular = match light {
                        Light::Diffuse { .. } => Color::black(),
                        Light::Point { position, color } => {
                            // direction to the light
                            let lightv = Unit::new_normalize(position - &hit.ray.position);

                            let light_dot_normal = lightv.dot(&hit.normal);

                            if light_dot_normal < 0. {
                                Color::black()
                            } else {
                                let diffuse = effective_color * diffuse * light_dot_normal;

                                // direction to the eye
                                if specular > 0. {
                                    let reflectv = math::reflect(&(-lightv), &hit.normal);
                                    let reflect_dot_eye = reflectv.dot(&eyev);
                                    let specular = if reflect_dot_eye <= 0. {
                                        Color::black()
                                    } else {
                                        let factor = reflect_dot_eye.powf(shininess);
                                        color * specular * factor
                                    };
                                    diffuse + specular
                                } else {
                                    diffuse
                                }
                            }
                        }
                    };

                    surface += diffuse_specular;
                }

                // If we're exiting a transparent object on this hit, we need to invert the normal.
                if containers.contains(hit.node) {
                    hit.normal = -hit.normal;
                }

                let reflected = self.reflected_color(
                    scene,
                    root,
                    containers.clone(),
                    reflection,
                    &hit,
                    reflective,
                );

                let (refracted, reflectance) = self.refracted_color(
                    scene,
                    root,
                    containers,
                    reflection,
                    &hit,
                    reflective > 0.0,
                    transparent,
                    refractive_index,
                );

                surface
                    + if reflective > 0.0 && transparent > 0.0 {
                        reflected * reflectance + refracted * (1.0 - reflectance)
                    } else {
                        reflected + refracted
                    }
            }

            Material::Emissive { pattern } => {
                scene
                    .pattern(*pattern)
                    .color_at(scene, &hit.object, &hit.normal)
            }
        }
    }

    fn reflected_color<'a>(
        &mut self,
        scene: &Scene,
        root: NodeId,
        containers: Cow<'a, Containers>,
        reflection: u32,
        hit: &Hit,
        reflective: f32,
    ) -> Color {
        if reflective <= 0.0 {
            return Color::black();
        }

        let mut reflect_ray = hit.ray.reflect(&hit.normal);
        reflect_ray.step(self.config.min_dist);
        reflective * self.color_for_ray(scene, root, containers, reflect_ray, reflection + 1)
    }

    fn refracted_color<'a>(
        &mut self,
        scene: &Scene,
        root: NodeId,
        mut containers: Cow<'a, Containers>,
        reflection: u32,
        hit: &Hit,
        reflective: bool,
        transparent: f32,
        refractive_index: f32,
    ) -> (Color, f32) {
        if transparent <= 0.0 {
            return (Color::black(), 1.0);
        }

        let (n1, n2) = containers
            .to_mut()
            .refractive_indices(hit.node, refractive_index);

        let n_ratio = n1 / n2;
        let cos_i = hit.ray.direction.dot(&hit.normal);
        let sin2_t = n_ratio.powi(2) * (1.0 - cos_i.powi(2));

        // Check for total internal reflection
        if sin2_t > 1.0 {
            return (Color::black(), 1.0);
        }

        let cos_t = f32::sqrt(1.0 - sin2_t);

        // Step 2x min distance along the negated normal to ensure that we step into the object,
        // and are far enough away to not trigger a hit immediately.
        let start = hit.ray.position - hit.normal.scale(self.config.min_dist * 2.0);

        let direction = Unit::new_unchecked(
            hit.normal.scale(n_ratio * cos_i - cos_t) - hit.ray.direction.scale(n_ratio),
        );

        let refract_ray = Ray::new(start, direction);
        let color =
            transparent * self.color_for_ray(scene, root, containers, refract_ray, reflection + 1);

        let schlick = if reflective {
            // TODO: it's not clear why cos_t is what should always be used here.
            let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
            r0 + (1.0 - r0) * (1.0 - cos_t).powi(5)
        } else {
            0.0
        };

        (color, schlick)
    }
}

impl<C: Camera> Integrator for Whitted<C> {
    fn luminance(&mut self, scene: &Scene, root: NodeId, sample: &Sample) -> Color {
        self.color_for_ray(
            scene,
            root,
            Cow::Owned(Containers::default()),
            self.camera.generate_ray(sample),
            0,
        )
    }
}
