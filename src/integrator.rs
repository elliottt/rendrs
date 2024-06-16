use crossbeam::{channel, thread};
use nalgebra::{Point2, Point3, Unit, Vector3};
use smallvec::SmallVec;
use std::borrow::Cow;

use crate::{
    camera::{Camera, CanvasInfo, Sample},
    canvas::{Canvas, Color},
    math,
    ray::Ray,
    sampler::Sampler,
    scene::{Distance, Light, MarchConfig, Material, MaterialId, NodeId, Scene},
};

/// An individual tile in the rendering target.
#[derive(Debug)]
struct Tile {
    offset_x: f32,
    offset_y: f32,
    width: u32,
    height: u32,
}

/// An iterator for tiles in a rendering target.
#[derive(Debug)]
struct Tiles {
    width: u32,
    height: u32,
    chunks_x: u32,
    chunks_y: u32,
    x: u32,
    y: u32,
}

impl Tiles {
    fn new(width: u32, height: u32) -> Self {
        let chunks_x = (width + 15) / 16;
        let chunks_y = (height + 15) / 16;

        Self {
            width,
            height,
            chunks_x,
            chunks_y,
            x: 0,
            y: 0,
        }
    }

    fn total(&self) -> u32 {
        self.chunks_x * self.chunks_y
    }
}

impl Iterator for Tiles {
    type Item = Tile;

    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= self.chunks_x {
            self.x = 0;
            self.y += 1;
        }

        if self.y >= self.chunks_y {
            return None;
        }

        let offset_x = self.x * 16;
        let offset_y = self.y * 16;
        let width = (self.width - offset_x).min(16);
        let height = (self.height - offset_y).min(16);

        self.x += 1;

        Some(Tile {
            offset_x: offset_x as f32,
            offset_y: offset_y as f32,
            width,
            height,
        })
    }
}

pub fn render(
    info: CanvasInfo,
    scene: &Scene,
    root: NodeId,
    sampler: impl Sampler,
    builder: impl IntegratorBuilder,
    num_threads: usize,
) -> Canvas {
    let mut canvas = info.new_canvas();

    let (input, tiles): (_, channel::Receiver<Tile>) = channel::unbounded();
    let (results, chunks) = channel::unbounded();

    thread::scope(|s| {
        for _ in 0..num_threads {
            let mut sampler = sampler.clone_sampler();
            let results = results.clone();
            let mut integrator = builder.build();
            let tiles = tiles.clone();
            s.spawn(move |_| {
                let mut samples = Vec::with_capacity(sampler.samples_per_pixel());
                let inv_num_samples = 1. / (sampler.samples_per_pixel() as f32);
                for tile in tiles.clone() {
                    let mut chunk = Canvas::new(tile.width, tile.height);

                    for ((col, row), pixel) in chunk.coords().zip(chunk.pixels_mut()) {
                        samples.clear();
                        sampler.pixel_samples(
                            &mut samples,
                            &Point2::new(col as f32 + tile.offset_x, row as f32 + tile.offset_y),
                        );
                        for sample in &samples {
                            let sample = Sample::new(sample.x, sample.y);
                            *pixel += integrator.luminance(scene, root, &sample);
                        }

                        *pixel *= inv_num_samples;
                    }

                    results
                        .send((tile.offset_x as u32, tile.offset_y as u32, chunk))
                        .unwrap();
                }
            });
        }

        let tiles = Tiles::new(info.width, info.height);
        let expecting = tiles.total() as usize;

        s.spawn(move |_| {
            for tile in tiles {
                input.send(tile).unwrap();
            }
        });

        for (offset_x, offset_y, chunk) in chunks.into_iter().take(expecting) {
            canvas.blit(offset_x, offset_y, &chunk)
        }
    })
    .unwrap();

    canvas
}

pub trait IntegratorBuilder {
    fn build(&self) -> Box<dyn Integrator>;
}

impl<C: IntegratorBuilder + ?Sized> IntegratorBuilder for Box<C> {
    fn build(&self) -> Box<dyn Integrator> {
        self.as_ref().build()
    }
}

pub trait Integrator: Send {
    fn luminance(&mut self, scene: &Scene, root: NodeId, sample: &Sample) -> Color;
}

impl<C> Integrator for Box<C>
where
    C: Integrator + ?Sized,
{
    fn luminance(&mut self, scene: &Scene, root: NodeId, sample: &Sample) -> Color {
        self.as_mut().luminance(scene, root, sample)
    }
}

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

        let mut hit =
            if let Some(hit) = Hit::march(&self.config, scene, root, ray, !containers.is_empty()) {
                hit
            } else {
                for light in scene.lights.iter() {
                    color += light.light_escape();
                }
                return color;
            };

        // return unlit magenta if there's no material for this object
        let material = if let Some(material) = hit.material {
            material
        } else {
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

/// A record of transparent objects that a ray is traversing.
#[derive(Clone, Debug, Default)]
pub struct Containers(SmallVec<[(NodeId, f32); 4]>);

impl Containers {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn contains(&self, node: NodeId) -> bool {
        self.0.iter().any(|(n, _)| *n == node)
    }

    /// For an intersection with object `node` with `refractive_index`, return the indices of
    /// refraction on either side of the intersection.
    fn refractive_indices(&mut self, node: NodeId, refractive_index: f32) -> (f32, f32) {
        let n1 = self.0.last().map(|(_, ri)| *ri).unwrap_or(1.0);

        // Determine if we're entering or leaving `node`
        if let Some(idx) = self
            .0
            .iter()
            .enumerate()
            .find(|(_, (n, _))| *n == node)
            .map(|(idx, _)| idx)
        {
            self.0.remove(idx);
        } else {
            self.0.push((node, refractive_index));
        }

        let n2 = self.0.last().map(|(_, ri)| *ri).unwrap_or(1.0);
        (n1, n2)
    }
}

/// Information about a ray hit with scene geometry.
pub struct Hit {
    /// The closest node in the scene.
    pub node: NodeId,

    /// The intersection point in object space.
    pub object: Point3<f32>,

    /// The normal of the object at the hit, in world space.
    pub normal: Unit<Vector3<f32>>,

    /// The material for the object.
    pub material: Option<MaterialId>,

    /// The ray that caused the intersection.
    pub ray: Ray,

    /// The distance traveled to get to this point.
    pub distance: Distance,

    /// The number of steps taken.
    pub steps: u32,
}

impl Hit {
    /// March the ray until it hits something in the geometry or runs out of fuel.
    pub fn march(
        config: &MarchConfig,
        scene: &Scene,
        root: NodeId,
        mut ray: Ray,
        inside: bool,
    ) -> Option<Self> {
        let mut total_dist = Distance::default();

        let node = scene.node(root);

        let sign = if inside { -1.0 } else { 1.0 };

        for i in 0..config.max_steps {
            let result = node.sdf(scene, root, &ray);
            let radius = result.distance.0 * sign;

            if radius < config.min_dist {
                return Some(Self {
                    node: result.id,
                    object: result.object,
                    normal: result.normal,
                    material: result.material,
                    ray,
                    distance: total_dist,
                    steps: i,
                });
            }

            total_dist.0 += radius;

            if total_dist.0 > config.max_dist {
                break;
            }

            ray.step(radius);
        }

        None
    }

    /// March the ray until it hits something, but return only the distance.
    pub fn march_dist(
        config: &MarchConfig,
        scene: &Scene,
        root: NodeId,
        mut ray: Ray,
    ) -> Option<Distance> {
        let mut total_dist = Distance::default();

        let node = scene.node(root);

        for _ in 0..config.max_steps {
            let result = node.fast_sdf(scene, &ray);
            let radius = result.distance.0;

            if radius < config.min_dist {
                return Some(total_dist);
            }

            total_dist.0 += radius;

            if total_dist.0 > config.max_dist {
                break;
            }

            ray.step(radius);
        }

        None
    }

    /// Returns `true` when there is an object between the hit and the light at the point provided.
    pub fn in_shadow(
        &self,
        config: &MarchConfig,
        scene: &Scene,
        root: NodeId,
        light: &Point3<f32>,
    ) -> bool {
        // Move the point away from the hit by min_dist so that we ensure that there won't be an
        // immediate intersection with the object.
        let start = &self.ray.position + config.min_dist * self.normal.as_ref();

        let dir = light - start;
        let dist_to_light = dir.norm();
        let ray = Ray::new(start, Unit::new_normalize(dir));
        Hit::march_dist(config, scene, root, ray)
            .map_or(false, |hit_dist| hit_dist.0 < dist_to_light)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refraction_sphere_direct() {
        let mut scene = Scene::default();

        let white = scene.solid(Color::white());
        let vacuum = scene.phong(white, 0.1, 0.9, 0.9, 200.0, 0.0, 1.0, 1.0);
        let sphere = scene.sphere(1.0);
        let root = scene.paint(vacuum, sphere);

        // Check normal construction from an external hit
        let res = Hit::march(
            &MarchConfig::default(),
            &scene,
            root,
            Ray::new(
                Point3::new(0., 0., -2.),
                Unit::new_unchecked(Vector3::new(0., 0., 1.)),
            ),
            false,
        )
        .expect("intersection");

        assert_eq!(res.normal.x, 0.);
        assert_eq!(res.normal.y, 0.);
        assert_eq!(res.normal.z, -1.);

        // Check normal construction from an internal hit, asserting that the normal computed is
        // always pointing out of the object.
        let res = Hit::march(
            &MarchConfig::default(),
            &scene,
            root,
            Ray::new(
                Point3::new(0., 0., 0.),
                Unit::new_unchecked(Vector3::new(0., 0., 1.)),
            ),
            true,
        )
        .expect("intersection");

        assert_eq!(res.normal.x, 0.);
        assert_eq!(res.normal.y, 0.);
        assert_eq!(res.normal.z, 1.);
    }

    #[test]
    fn test_refraction_indices() {
        let mut containers = Containers::default();

        let mut scene = Scene::default();

        // These aren't used for actual intersections, as we're mocking the intersection order in
        // the asserts below.
        let a = scene.sphere(1.);
        let b = scene.sphere(1.);
        let c = scene.sphere(1.);

        assert_eq!((1.0, 1.5, false), containers.refractive_indices(a, 1.5));
        assert_eq!((1.5, 2.0, false), containers.refractive_indices(b, 2.0));
        assert_eq!((2.0, 2.5, false), containers.refractive_indices(c, 2.5));
        assert_eq!((2.5, 2.5, true), containers.refractive_indices(b, 2.0));
        assert_eq!((2.5, 1.5, true), containers.refractive_indices(c, 2.5));
        assert_eq!((1.5, 1.0, true), containers.refractive_indices(a, 1.5));
    }
}
