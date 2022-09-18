use nalgebra::{Point3, Unit, Vector3};
use rayon::prelude::*;

use crate::{
    camera::{Camera, CanvasInfo, Sample},
    canvas::{Canvas, Color},
    lighting,
    ray::Ray,
    scene::{Distance, MarchConfig, MaterialId, NodeId, Scene},
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

pub fn render<I: Integrator>(
    info: CanvasInfo,
    scene: &Scene,
    root: NodeId,
    integrator: &I,
) -> Canvas {
    Tiles::new(info.width, info.height)
        .par_bridge()
        .fold(
            || info.new_canvas(),
            |mut canvas, tile| {
                let mut chunk = Canvas::new(tile.width, tile.height);

                for ((col, row), pixel) in chunk.coords().zip(chunk.pixels_mut()) {
                    let y = row as f32 + tile.offset_y + 0.5;
                    let x = col as f32 + tile.offset_x + 0.5;

                    let sample = Sample::new(x, y);
                    *pixel = integrator.luminance(scene, root, &sample);
                }

                canvas.blit(tile.offset_x as u32, tile.offset_y as u32, &chunk);

                canvas
            },
        )
        .reduce(|| info.new_canvas(), Canvas::merge)
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
}

impl<C> Whitted<C> {
    pub fn new(camera: C, config: MarchConfig) -> Self {
        Self { camera, config }
    }
}

impl<C: Camera + std::marker::Send + std::marker::Sync> Integrator for Whitted<C> {
    fn luminance(&self, scene: &Scene, root: NodeId, sample: &Sample) -> Color {
        let hit = Hit::march(&self.config, scene, root, self.camera.generate_ray(sample));

        if hit.is_none() {
            let mut color = Color::black();
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

        let eye = -hit.ray.direction;

        // TODO: compute emitted light for emissive objects

        let mut color = Color::black();
        for light in scene.lights.iter() {
            // if this light has a position in the scene, check to see if it's visible from the
            // intersection point.
            let in_shadow = light.position().map_or(false, |light| {
                hit.in_shadow(&self.config, scene, root, &light)
            });
            color += lighting::phong(
                scene,
                material,
                light,
                &hit.object,
                &hit.ray.position,
                &eye,
                &hit.normal,
                in_shadow,
            );
        }

        // TODO: compute reflection contribution
        // TODO: compute refraction contribution

        color
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
