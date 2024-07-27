use crossbeam::{channel, thread};
use nalgebra::{Point2, Point3, Unit, Vector3};
use smallvec::SmallVec;

use crate::{
    camera::{CanvasInfo, Sample},
    canvas::{Canvas, Color},
    ray::Ray,
    sampler::Sampler,
    scene::{Distance, MarchConfig, MaterialId, NodeId, Scene},
};

mod whitted;

pub use whitted::WhittedBuilder;

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

        assert_eq!((1.0, 1.5), containers.refractive_indices(a, 1.5));
        assert!(containers.contains(a));
        assert_eq!((1.5, 2.0), containers.refractive_indices(b, 2.0));
        assert!(containers.contains(b));
        assert_eq!((2.0, 2.5), containers.refractive_indices(c, 2.5));
        assert!(containers.contains(c));
        assert_eq!((2.5, 2.5), containers.refractive_indices(b, 2.0));
        assert!(!containers.contains(b));
        assert_eq!((2.5, 1.5), containers.refractive_indices(c, 2.5));
        assert!(!containers.contains(c));
        assert_eq!((1.5, 1.0), containers.refractive_indices(a, 1.5));
        assert!(!containers.contains(a));
    }
}
