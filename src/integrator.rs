use crossbeam::{channel, thread};
use nalgebra::{Point2, Point3, Unit, Vector3};

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

pub fn render<I: Integrator, S: Sampler>(
    info: CanvasInfo,
    scene: &Scene,
    root: NodeId,
    sampler: &S,
    integrator: &I,
    num_threads: usize,
) -> Canvas {
    let mut canvas = info.new_canvas();

    let (input, tiles): (_, channel::Receiver<Tile>) = channel::unbounded();
    let (results, chunks) = channel::unbounded();

    thread::scope(|s| {
        for _ in 0..num_threads {
            s.spawn(|_| {
                let mut sampler = sampler.clone();
                let inv_num_samples = 1. / (sampler.samples_per_pixel() as f32);
                let results = results.clone();
                for tile in tiles.clone() {
                    let mut chunk = Canvas::new(tile.width, tile.height);

                    for ((col, row), pixel) in chunk.coords().zip(chunk.pixels_mut()) {
                        for sample in sampler.pixel(&Point2::new(
                            col as f32 + tile.offset_x,
                            row as f32 + tile.offset_y,
                        )) {
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

pub trait Integrator: std::marker::Send + std::marker::Sync {
    fn luminance(&self, scene: &Scene, root: NodeId, sample: &Sample) -> Color;
}

impl<C> Integrator for Box<C>
where
    C: Integrator + ?Sized,
{
    fn luminance(&self, scene: &Scene, root: NodeId, sample: &Sample) -> Color {
        self.as_ref().luminance(scene, root, sample)
    }
}

#[derive(Clone)]
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
    pub fn color_for_ray(&self, scene: &Scene, root: NodeId, ray: Ray, reflection: u32) -> Color {
        let mut color = Color::black();

        if reflection >= self.max_reflections {
            return color;
        }

        let hit = Hit::march(&self.config, scene, root, ray);

        if hit.is_none() {
            for light in scene.lights.iter() {
                color += light.light_escape();
            }
            return color;
        }

        let hit = hit.unwrap();

        // return unlit magenta if there's no material for this object
        if hit.material.is_none() {
            return Color::hex(0xff00ff);
        }

        let material = scene.material(hit.material.unwrap());
        let color = match material {
            &Material::Phong {
                pattern,
                ambient,
                diffuse,
                specular,
                shininess,
                reflective,
            } => {
                let eyev = -hit.ray.direction;

                let base_color = scene
                    .pattern(pattern)
                    .color_at(scene, &hit.object, &hit.normal);

                let mut total = Color::black();
                for light in scene.lights.iter() {
                    let effective_color = &base_color * light.intensity();
                    total += ambient * &effective_color;

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

                    total += diffuse_specular;
                }

                if reflective > 0. {
                    let mut reflect_ray = hit.ray.reflect(&hit.normal);
                    reflect_ray.step(self.config.min_dist);
                    total +=
                        reflective * self.color_for_ray(scene, root, reflect_ray, reflection + 1);
                }

                total
            }

            Material::Emissive { pattern } => {
                scene
                    .pattern(*pattern)
                    .color_at(scene, &hit.object, &hit.normal)
            }
        };

        // TODO: compute refraction contribution

        color
    }
}

impl<C: Camera + std::marker::Send + std::marker::Sync> Integrator for Whitted<C> {
    fn luminance(&self, scene: &Scene, root: NodeId, sample: &Sample) -> Color {
        self.color_for_ray(scene, root, self.camera.generate_ray(sample), 0)
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
    pub fn march(config: &MarchConfig, scene: &Scene, root: NodeId, mut ray: Ray) -> Option<Self> {
        let mut total_dist = Distance::default();

        let node = scene.node(root);

        for i in 0..config.max_steps {
            let result = node.sdf(scene, root, &ray);
            let radius = result.distance.0;

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
