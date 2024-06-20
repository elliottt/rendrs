use approx::AbsDiffEq;
use nalgebra::{Point3, Unit, Vector2, Vector3};

use crate::{
    bvh::{BoundingBox, BVH},
    canvas::Color,
    math::Mix,
    ray::Ray,
    transform::{ApplyTransform, Transform},
};

#[derive(Debug, Default)]
pub struct Scene {
    pub nodes: Vec<(BoundingBox, Node)>,
    pub patterns: Vec<Pattern>,
    pub materials: Vec<Material>,
    pub lights: Vec<Light>,
}

// TODO: make a macro for deriving the id/vector pairs

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PatternId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MaterialId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LightId(u32);

/// Primitive shapes, centered at the origin.
#[derive(Debug)]
pub enum Prim {
    /// A plane with the given normal.
    Plane { normal: Unit<Vector3<f32>> },

    /// A sphere with the given radius.
    Sphere { radius: f32 },

    /// A box with the given dimensions.
    Box { width: f32, height: f32, depth: f32 },

    /// A torus with the given hole radius and ring radius.
    Torus { hole: f32, radius: f32 },

    /// A triangle with no depth.
    Triangle {
        a: Point3<f32>,
        b: Point3<f32>,
        c: Point3<f32>,
        n: Unit<Vector3<f32>>,
    },
}

/// Nodes in the scene graph.
#[derive(Debug)]
pub enum Node {
    /// Primitive shapes.
    Prim { prim: Prim },

    /// Invert an SDF.
    Invert { node: NodeId },

    /// A group of nodes.
    Group { union: bool, nodes: BVH<NodeId> },

    /// Subtracting one node from another.
    Subtract { left: NodeId, right: NodeId },

    /// A smooth union of two nodes.
    SmoothUnion { k: f32, left: NodeId, right: NodeId },

    /// The intersection of nodes.
    Intersect { nodes: Vec<NodeId> },

    /// Apply this Transform the node.
    Transform { transform: Transform, node: NodeId },

    /// Apply this material to the node.
    Material { material: MaterialId, node: NodeId },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Distance(pub f32);

#[derive(Debug, Clone)]
pub struct MarchConfig {
    pub max_steps: u32,
    pub min_dist: f32,
    pub max_dist: f32,
}

impl Default for MarchConfig {
    fn default() -> Self {
        Self {
            max_steps: 200,
            min_dist: 0.001,
            max_dist: 1000.,
        }
    }
}

#[derive(Debug)]
pub struct SDFResult {
    /// The closest object.
    pub id: NodeId,

    /// The point in object space.
    pub object: Point3<f32>,

    /// The normal in world space.
    pub normal: Unit<Vector3<f32>>,

    /// The distance between the world-space ray and this object.
    pub distance: Distance,

    /// The maierial for the object.
    pub material: Option<MaterialId>,
}

impl SDFResult {
    fn new(id: NodeId, object: Point3<f32>) -> Self {
        Self {
            id,
            object,
            normal: Unit::new_unchecked(Vector3::new(0., 0., 1.)),
            distance: Distance(std::f32::INFINITY),
            material: None,
        }
    }
}

#[derive(Debug)]
pub struct FastSDFResult {
    /// The distance between the world-space ray and this object.
    pub distance: Distance,

    /// The material for the object.
    pub material: Option<MaterialId>,
}

impl FastSDFResult {
    fn new() -> Self {
        Self {
            distance: Distance(std::f32::INFINITY),
            material: None,
        }
    }
}

impl Scene {
    #[inline]
    fn add_node(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        let bounds = node.bounding_box(self);
        self.nodes.push((bounds, node));
        id
    }

    /// Fetch a node from the scene.
    #[inline]
    pub fn node(&self, NodeId(id): NodeId) -> &Node {
        &self.nodes[id as usize].1
    }

    #[inline]
    pub fn bounding_box(&self, NodeId(id): NodeId) -> &BoundingBox {
        &self.nodes[id as usize].0
    }

    /// Construct a plane with the given normal in the scene.
    pub fn plane(&mut self, normal: Unit<Vector3<f32>>) -> NodeId {
        self.add_node(Node::Prim {
            prim: Prim::Plane { normal },
        })
    }

    /// Construct a sphere with the given radius in the scene.
    pub fn sphere(&mut self, radius: f32) -> NodeId {
        self.add_node(Node::Prim {
            prim: Prim::Sphere { radius },
        })
    }

    /// Construct a box with the given dimensions in the scene.
    pub fn rect(&mut self, width: f32, height: f32, depth: f32) -> NodeId {
        self.add_node(Node::Prim {
            prim: Prim::Box {
                width,
                height,
                depth,
            },
        })
    }

    /// Construct a torus with the given inner and outer radii.
    pub fn torus(&mut self, hole: f32, radius: f32) -> NodeId {
        self.add_node(Node::Prim {
            prim: Prim::Torus { hole, radius },
        })
    }

    /// Render a triangle in the scene, with no depth.
    pub fn triangle(
        &mut self,
        a: Point3<f32>,
        b: Point3<f32>,
        c: Point3<f32>,
        n: Unit<Vector3<f32>>,
    ) -> NodeId {
        self.add_node(Node::Prim {
            prim: Prim::Triangle { a, b, c, n },
        })
    }

    /// Invert the node.
    pub fn invert(&mut self, node: NodeId) -> NodeId {
        self.add_node(Node::Invert { node })
    }

    fn add_group(&mut self, union: bool, nodes: Vec<NodeId>) -> NodeId {
        assert!(!nodes.is_empty());
        let nodes = nodes
            .into_iter()
            .map(|id| (self.bounding_box(id).clone(), id))
            .collect();
        let nodes = BVH::from_nodes(nodes);
        self.add_node(Node::Group { union, nodes })
    }

    pub fn group(&mut self, nodes: Vec<NodeId>) -> NodeId {
        self.add_group(false, nodes)
    }

    pub fn union(&mut self, nodes: Vec<NodeId>) -> NodeId {
        self.add_group(true, nodes)
    }

    pub fn subtract(&mut self, left: NodeId, right: NodeId) -> NodeId {
        self.add_node(Node::Subtract { left, right })
    }

    pub fn smooth_union(&mut self, k: f32, nodes: &[NodeId]) -> NodeId {
        match nodes.len() {
            0 => panic!("no nodes given to `smooth_union`"),
            1 => nodes[0],
            len => {
                let (left, right) = nodes.split_at(len / 2);
                let left = self.smooth_union(k, left);
                let right = self.smooth_union(k, right);
                self.add_node(Node::SmoothUnion { k, left, right })
            }
        }
    }

    pub fn intersect(&mut self, nodes: Vec<NodeId>) -> NodeId {
        self.add_node(Node::Intersect { nodes })
    }

    pub fn transform(&mut self, transform: Transform, node: NodeId) -> NodeId {
        // as an optimization, compose transforms of transforms while building the scene.
        if let Node::Transform { transform: t, node } = self.node(node) {
            self.add_node(Node::Transform {
                transform: transform * t,
                node: *node,
            })
        } else {
            self.add_node(Node::Transform { transform, node })
        }
    }

    pub fn paint(&mut self, material: MaterialId, node: NodeId) -> NodeId {
        self.add_node(Node::Material { material, node })
    }

    #[inline]
    fn add_material(&mut self, material: Material) -> MaterialId {
        let id = MaterialId(self.materials.len() as u32);
        self.materials.push(material);
        id
    }

    #[inline]
    pub fn material(&self, MaterialId(id): MaterialId) -> &Material {
        &self.materials[id as usize]
    }

    pub fn phong(
        &mut self,
        pattern: PatternId,
        ambient: f32,
        diffuse: f32,
        specular: f32,
        shininess: f32,
        reflective: f32,
        transparent: f32,
        refractive_index: f32,
    ) -> MaterialId {
        self.add_material(Material::Phong {
            pattern,
            ambient,
            diffuse,
            specular,
            shininess,
            reflective,
            transparent,
            refractive_index,
        })
    }

    pub fn emissive(&mut self, pattern: PatternId) -> MaterialId {
        self.add_material(Material::Emissive { pattern })
    }

    #[inline]
    fn add_light(&mut self, light: Light) -> LightId {
        let id = LightId(self.lights.len() as u32);
        self.lights.push(light);
        id
    }

    pub fn point_light(&mut self, position: Point3<f32>, color: Color) -> LightId {
        self.add_light(Light::Point { position, color })
    }

    pub fn diffuse_light(&mut self, color: Color) -> LightId {
        self.add_light(Light::Diffuse { color })
    }

    #[inline]
    fn add_pattern(&mut self, pattern: Pattern) -> PatternId {
        let id = PatternId(self.patterns.len() as u32);
        self.patterns.push(pattern);
        id
    }

    #[inline]
    pub fn pattern(&self, PatternId(id): PatternId) -> &Pattern {
        &self.patterns[id as usize]
    }

    pub fn solid(&mut self, color: Color) -> PatternId {
        self.add_pattern(Pattern::Solid { color })
    }

    pub fn gradiant(&mut self, first: PatternId, second: PatternId) -> PatternId {
        self.add_pattern(Pattern::Gradiant { first, second })
    }

    pub fn stripes(&mut self, first: PatternId, second: PatternId) -> PatternId {
        self.add_pattern(Pattern::Stripes { first, second })
    }

    pub fn checkers(&mut self, first: PatternId, second: PatternId) -> PatternId {
        self.add_pattern(Pattern::Checkers { first, second })
    }

    pub fn shells(&mut self, first: PatternId, second: PatternId) -> PatternId {
        self.add_pattern(Pattern::Shells { first, second })
    }

    pub fn transform_pat(&mut self, transform: Transform, pattern: PatternId) -> PatternId {
        self.add_pattern(Pattern::Transform { transform, pattern })
    }
}

impl Prim {
    /// Determine the bounding box for this primitive.
    pub fn bounding_box(&self) -> BoundingBox {
        match self {
            Prim::Plane { .. } => BoundingBox::max(),

            &Prim::Sphere { radius } => BoundingBox::new(
                Point3::new(-radius, -radius, -radius),
                Point3::new(radius, radius, radius),
            ),

            &Prim::Box {
                width,
                height,
                depth,
            } => BoundingBox::new(
                Point3::new(-width, -height, -depth),
                Point3::new(width, height, depth),
            ),

            &Prim::Torus { hole, radius } => {
                let rad = hole + radius;
                BoundingBox::new(
                    Point3::new(-rad, -radius, -rad),
                    Point3::new(rad, radius, rad),
                )
            }

            &Prim::Triangle { a, b, c, .. } => BoundingBox::new(a, b).union_point(&c),
        }
    }

    /// Compute the distance from the current position of the ray to the primitive object. As
    /// primitives are all centered at the origin, there is no need to return more information than
    /// the distance.
    pub fn sdf(&self, p: &Point3<f32>) -> Distance {
        let pv = Vector3::new(p.x, p.y, p.z);
        match self {
            Prim::Plane { normal } => Distance(pv.dot(normal)),
            Prim::Sphere { radius } => Distance(pv.norm() - radius),
            Prim::Box {
                width,
                height,
                depth,
            } => {
                let p = pv.abs();
                let x = p.x - width;
                let y = p.y - height;
                let z = p.z - depth;
                let diff = x.max(y).max(z).min(0.0);
                Distance(Vector3::new(x.max(0.), y.max(0.), z.max(0.)).norm() + diff)
            }

            Prim::Torus { hole, radius } => {
                let q = Vector2::new(pv.xz().norm() - hole, pv.y);
                return Distance(q.norm() - radius);
            }

            Prim::Triangle { a, b, c, n } => {
                let ba = b - a;
                let cb = c - b;
                let ac = a - c;

                let pa = p - a;
                let pb = p - b;
                let pc = p - c;

                let v = if ba.cross(&n).dot(&pa).signum()
                    + cb.cross(&n).dot(&pb).signum()
                    + ac.cross(&n).dot(&pc).signum()
                    < 2.0
                {
                    let x = ba * f32::clamp(ba.dot(&pa) / ba.dot(&ba), 0.0, 1.0) - pa;
                    let y = cb * f32::clamp(cb.dot(&pb) / cb.dot(&cb), 0.0, 0.0) - pb;
                    let z = ac * f32::clamp(ac.dot(&pc) / ac.dot(&ac), 0.0, 0.0) - pc;
                    x.dot(&x).min(y.dot(&y)).min(z.dot(&z))
                } else {
                    n.dot(&pa).powi(2)/n.dot(&n)
                };

                Distance(f32::sqrt(v))
            }
        }
    }

    /// Compute the normal for the primitive when possible.
    pub fn normal(&self, p: &Point3<f32>) -> Option<Unit<Vector3<f32>>> {
        match self {
            // The plane knows its normal already
            Prim::Plane { normal } => Some(normal.clone()),

            // The sphere is always centered at the origin.
            Prim::Sphere { .. } => Some(Unit::new_normalize(Vector3::new(p.x, p.y, p.z))),

            Prim::Triangle { n, .. } => Some(n.clone()),

            _ => None,
        }
    }
}

/// Returns the difference between the right and left distances, `h` which is the linear
/// interpolation value between the two distances, and the composite distance.
fn smooth_union_parts(k: f32, left: Distance, right: Distance) -> (f32, f32, Distance) {
    let diff = right.0 - left.0;

    let h = (0.5 + 0.5 * diff / k).clamp(0., 1.);
    let factor = k * h * (1.0 - h);

    (diff, h, Distance(f32::mix(right.0, left.0, h) - factor))
}

impl Node {
    pub fn bounding_box(&self, scene: &Scene) -> BoundingBox {
        match self {
            Node::Prim { prim } => prim.bounding_box(),

            Node::Invert { .. } => BoundingBox::Max,

            Node::Group { nodes, .. } => nodes.bounding_box(),

            Node::Subtract { left, .. } => scene.bounding_box(*left).clone(),

            Node::SmoothUnion { left, right, .. } => {
                scene.bounding_box(*left).union(scene.bounding_box(*right))
            }

            Node::Intersect { nodes } => {
                nodes.iter().copied().fold(BoundingBox::max(), |acc, id| {
                    acc.intersect(scene.bounding_box(id))
                })
            }

            Node::Transform { transform, node } => scene.bounding_box(*node).apply(transform),

            Node::Material { node, .. } => scene.bounding_box(*node).clone(),
        }
    }

    pub fn sdf(&self, scene: &Scene, id: NodeId, ray: &Ray) -> SDFResult {
        match self {
            Node::Prim { prim } => {
                let distance = prim.sdf(&ray.position);
                SDFResult {
                    id,
                    material: None,
                    object: ray.position,
                    normal: prim
                        .normal(&ray.position)
                        .unwrap_or_else(|| self.normal_sdf(scene, ray.clone(), distance)),
                    distance,
                }
            }

            Node::Invert { node } => {
                let mut res = scene.node(*node).sdf(scene, *node, ray);

                res.distance.0 = -res.distance.0;
                res.normal = -res.normal;

                res
            }

            Node::Group { union, nodes } => {
                let mut res =
                    nodes.fold_intersections(ray, SDFResult::new(id, ray.position), |acc, &id| {
                        let res = scene.node(id).sdf(scene, id, ray);
                        if res.distance < acc.distance {
                            res
                        } else {
                            acc
                        }
                    });

                if *union {
                    res.id = id;
                    res.object = ray.position;
                }

                res
            }

            Node::Subtract { left, right } => {
                let mut left = scene.node(*left).sdf(scene, *left, ray);
                let mut right = scene.node(*right).sdf(scene, *right, ray);

                right.distance.0 = -right.distance.0;

                if left.distance < right.distance {
                    right.object = ray.position;
                    right.normal = -right.normal;
                    right.material = right.material.or_else(|| left.material);
                    right
                } else {
                    left.object = ray.position;
                    left
                }
            }

            Node::SmoothUnion { k, left, right } => {
                let mut left = scene.node(*left).sdf(scene, *left, ray);
                let right = scene.node(*right).sdf(scene, *right, ray);

                let (diff, h, dist) = smooth_union_parts(*k, left.distance, right.distance);

                if diff < 0. {
                    left.material = right.material;
                }

                left.distance = dist;

                // Try to preserve the normals of the left and right objects, but fall back on
                // re-computing the normal for the points where the two are blending. This isn't
                // necessarily an optimization if the unioned models are simple, as it's more
                // costly to run `Node::sdf` than `Node::fast_sdf`.
                if h < 1. {
                    if h == 0. {
                        left.normal = right.normal;
                    } else {
                        left.normal = right
                            .normal
                            .try_slerp(&left.normal, h, f32::default_epsilon())
                            .unwrap_or_else(|| self.normal_sdf(scene, ray.clone(), left.distance));
                    }
                }

                left.object = ray.position;

                left
            }

            Node::Intersect { nodes } => {
                let mut res = nodes
                    .iter()
                    .copied()
                    .map(|id| scene.node(id).sdf(scene, id, ray))
                    .max_by_key(|res| res.distance)
                    .unwrap();

                res.object = ray.position;

                res
            }

            Node::Transform { transform, node } => {
                let mut res = scene.node(*node).sdf(scene, *node, &ray.invert(transform));
                res.normal = res.normal.apply(transform);
                res.distance.0 *= transform.scale_factor();
                res
            }

            Node::Material { material, node } => {
                let mut res = scene.node(*node).sdf(scene, *node, ray);
                res.material = Some(*material);
                res
            }
        }
    }

    /// Compute the normal by using the SDF. Useful as an intermediate for combination nodes that
    /// don't have a closed form normal computation.
    fn normal_sdf(&self, scene: &Scene, mut ray: Ray, dist: Distance) -> Unit<Vector3<f32>> {
        let p = ray.position;
        let offset = Vector3::new(0.00001, 0.0, 0.0);

        ray.position = p - offset.xyy();
        let px = self.fast_sdf(scene, &ray).distance;

        ray.position = p - offset.yxy();
        let py = self.fast_sdf(scene, &ray).distance;

        ray.position = p - offset.yyx();
        let pz = self.fast_sdf(scene, &ray).distance;
        Unit::new_normalize(Vector3::new(dist.0 - px.0, dist.0 - py.0, dist.0 - pz.0))
    }

    // A version of `sdf` that only computes the distance and material information. Useful for
    // things like lighting calculations.
    pub fn fast_sdf(&self, scene: &Scene, ray: &Ray) -> FastSDFResult {
        match self {
            Node::Prim { prim } => FastSDFResult {
                distance: prim.sdf(&ray.position),
                material: None,
            },

            Node::Invert { node } => {
                let mut res = scene.node(*node).fast_sdf(scene, ray);
                res.distance.0 = -res.distance.0;
                res
            }

            Node::Group { nodes, .. } => {
                nodes.fold_intersections(ray, FastSDFResult::new(), |acc, &id| {
                    let res = scene.node(id).fast_sdf(scene, ray);
                    if res.distance < acc.distance {
                        res
                    } else {
                        acc
                    }
                })
            }

            Node::Subtract { left, right } => {
                let left = scene.node(*left).fast_sdf(scene, ray);
                let mut right = scene.node(*right).fast_sdf(scene, ray);

                right.distance.0 = -right.distance.0;
                if left.distance < right.distance {
                    right
                } else {
                    left
                }
            }

            Node::SmoothUnion { k, left, right } => {
                let mut left = scene.node(*left).fast_sdf(scene, ray);
                let right = scene.node(*right).fast_sdf(scene, ray);

                let (diff, _, dist) = smooth_union_parts(*k, left.distance, right.distance);

                if diff < 0. {
                    left.material = right.material;
                }

                left.distance = dist;

                left
            }

            Node::Intersect { nodes } => nodes
                .iter()
                .copied()
                .map(|id| scene.node(id).fast_sdf(scene, ray))
                .max_by_key(|res| res.distance)
                .unwrap(),

            Node::Transform { transform, node } => {
                let mut res = scene.node(*node).fast_sdf(scene, &ray.invert(transform));
                res.distance.0 *= transform.scale_factor();
                res
            }

            Node::Material { node, .. } => scene.node(*node).fast_sdf(scene, ray),
        }
    }
}

impl Mix for Distance {
    type Output = Distance;

    #[inline]
    fn mix(self, b: Distance, t: f32) -> Self::Output {
        Distance(f32::mix(self.0, b.0, t))
    }
}

impl PartialEq for Distance {
    fn eq(&self, other: &Distance) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for Distance {}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Distance) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Distance {
    fn cmp(&self, other: &Distance) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap_or_else(|| {
            if self.0.is_nan() {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        })
    }
}

#[derive(Debug)]
pub enum Light {
    /// A diffuse light, for rays that escape the scene.
    Diffuse { color: Color },

    /// A point light, positioned according to the given transform.
    Point { position: Point3<f32>, color: Color },
}

impl Light {
    /// The light contribution for rays that escape the scene.
    pub fn light_escape(&self) -> Color {
        match self {
            Light::Diffuse { color } => color.clone(),
            Light::Point { .. } => Color::black(),
        }
    }

    pub fn intensity(&self) -> &Color {
        match self {
            Light::Diffuse { color } => color,
            Light::Point { color, .. } => color,
        }
    }

    pub fn position(&self) -> Option<Point3<f32>> {
        match self {
            Light::Diffuse { .. } => None,
            Light::Point { position, .. } => Some(position.clone()),
        }
    }
}

/// Materials using the Phong reflection model.
#[derive(Debug)]
pub enum Material {
    Phong {
        /// The pattern of the surface.
        pattern: PatternId,

        /// The ambient reflection of this surface.
        ambient: f32,

        /// The diffuse reflection of this surface.
        diffuse: f32,

        /// The specular reflection of this surface.
        specular: f32,

        /// The shininess of the surface.
        shininess: f32,

        /// How reflective the surface is.
        reflective: f32,

        /// How transparent the object is.
        transparent: f32,

        /// The refractive index of the object.
        refractive_index: f32,
    },

    Emissive {
        /// The emissive pattern.
        pattern: PatternId,
    },
}

/// Patterns for texturing a surface with.
#[derive(Debug)]
pub enum Pattern {
    /// Just a solid color.
    Solid { color: Color },

    /// A gradient based on the object's x value.
    Gradiant { first: PatternId, second: PatternId },

    /// Stripes of two different patterns.
    Stripes { first: PatternId, second: PatternId },

    /// Checkers of two different patterns.
    Checkers { first: PatternId, second: PatternId },

    /// Shells of two different patterns.
    Shells { first: PatternId, second: PatternId },

    /// Transform the point before rendering the pattern.
    Transform {
        transform: Transform,
        pattern: PatternId,
    },
}

impl Pattern {
    /// Generate the color for a point in object space, along with its world normal.
    pub fn color_at(
        &self,
        scene: &Scene,
        point: &Point3<f32>,
        normal: &Unit<Vector3<f32>>,
    ) -> Color {
        match self {
            Pattern::Solid { color } => color.clone(),

            Pattern::Gradiant { first, second } => {
                if point.x < 0. {
                    scene.pattern(*first).color_at(scene, point, normal)
                } else if point.x > 1. {
                    scene.pattern(*second).color_at(scene, point, normal)
                } else {
                    let first = scene.pattern(*first).color_at(scene, point, normal);
                    let second = scene.pattern(*second).color_at(scene, point, normal);
                    first.mix(&second, point.x)
                }
            }

            Pattern::Stripes { first, second } => {
                if point.x.floor() % 2. == 0. {
                    scene.pattern(*first).color_at(scene, point, normal)
                } else {
                    scene.pattern(*second).color_at(scene, point, normal)
                }
            }

            Pattern::Checkers { first, second } => {
                let val = point.x.floor() + point.y.floor() + point.z.floor();
                if val % 2. == 0. {
                    scene.pattern(*first).color_at(scene, point, normal)
                } else {
                    scene.pattern(*second).color_at(scene, point, normal)
                }
            }

            Pattern::Shells { first, second } => {
                let val = Vector3::new(point.x, point.y, point.z).norm().floor();
                if val % 2. == 0. {
                    scene.pattern(*first).color_at(scene, point, normal)
                } else {
                    scene.pattern(*second).color_at(scene, point, normal)
                }
            }

            Pattern::Transform { transform, pattern } => {
                let point = point.invert(transform);
                scene.pattern(*pattern).color_at(scene, &point, normal)
            }
        }
    }
}
