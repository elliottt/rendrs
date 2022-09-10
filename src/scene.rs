use nalgebra::{Point3, Unit, Vector2, Vector3};

use crate::{
    canvas::Color,
    math,
    transform::{ApplyTransform, Transform},
};

#[derive(Debug, Default)]
pub struct Scene {
    pub nodes: Vec<Node>,
    pub materials: Vec<Material>,
    pub lights: Vec<Light>,
}

// TODO: make a macro for deriving the id/vector pairs

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(u32);

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
}

/// Nodes in the scene graph.
#[derive(Debug)]
pub enum Node {
    /// Primitive shapes.
    Prim { prim: Prim },

    /// A group of nodes.
    Group { union: bool, nodes: Vec<NodeId> },

    /// A smooth union of two nodes.
    SmoothUnion { k: f32, left: NodeId, right: NodeId },

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
            min_dist: 0.01,
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

    /// The material for the object.
    pub material: Option<MaterialId>,
}

#[derive(Debug)]
pub struct FastSDFResult {
    /// The distance between the world-space ray and this object.
    pub distance: Distance,

    /// The material for the object.
    pub material: Option<MaterialId>,
}

impl Scene {
    fn add_node(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }

    /// Fetch a node from the scene.
    pub fn node(&self, NodeId(id): NodeId) -> &Node {
        &self.nodes[id as usize]
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

    pub fn group(&mut self, nodes: Vec<NodeId>) -> NodeId {
        self.add_node(Node::Group {
            union: false,
            nodes,
        })
    }

    pub fn union(&mut self, nodes: Vec<NodeId>) -> NodeId {
        self.add_node(Node::Group { union: true, nodes })
    }

    pub fn smooth_union(&mut self, k: f32, left: NodeId, right: NodeId) -> NodeId {
        self.add_node(Node::SmoothUnion { k, left, right })
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

    fn add_material(&mut self, material: Material) -> MaterialId {
        let id = MaterialId(self.materials.len() as u32);
        self.materials.push(material);
        id
    }

    pub fn material(&self, MaterialId(id): MaterialId) -> &Material {
        &self.materials[id as usize]
    }

    pub fn phong(
        &mut self,
        pattern: Color,
        ambient: f32,
        diffuse: f32,
        specular: f32,
        shininess: f32,
    ) -> MaterialId {
        self.add_material(Material {
            pattern,
            ambient,
            diffuse,
            specular,
            shininess,
        })
    }

    fn add_light(&mut self, light: Light) -> LightId {
        let id = LightId(self.lights.len() as u32);
        self.lights.push(light);
        id
    }

    pub fn light(&self, LightId(id): LightId) -> &Light {
        &self.lights[id as usize]
    }

    pub fn point_light(&mut self, position: Point3<f32>, color: Color) -> LightId {
        self.add_light(Light::Point { position, color })
    }

    pub fn diffuse_light(&mut self, color: Color) -> LightId {
        self.add_light(Light::Diffuse { color })
    }
}

impl Prim {
    /// Compute the distance from the current position of the ray to the primitive object. As
    /// primitives are all centered at the origin, there is no need to return more information than
    /// the distance.
    pub fn sdf(&self, p: &Point3<f32>) -> Distance {
        let p = Vector3::new(p.x, p.y, p.z);
        match self {
            Prim::Plane { normal } => Distance(p.dot(normal)),
            Prim::Sphere { radius } => Distance(p.norm() - radius),
            Prim::Box {
                width,
                height,
                depth,
            } => {
                let x = p.x.abs() - width;
                let y = p.y.abs() - height;
                let z = p.z.abs() - depth;
                let diff = x.max(y).max(z).min(0.0);
                Distance(Vector3::new(x.max(0.), y.max(0.), z.max(0.)).norm() + diff)
            }

            Prim::Torus { hole, radius } => {
                let q = Vector2::new(p.xz().norm() - hole, p.y);
                return Distance(q.norm() - radius);
            }
        }
    }

    /// Compute the normal for the primitive, relative to the point.
    pub fn normal(&self, p: &Point3<f32>) -> Unit<Vector3<f32>> {
        match self {
            // The plane knows its normal already
            Prim::Plane { normal } => normal.clone(),

            // The sphere is always centered at the origin.
            Prim::Sphere { .. } => Unit::new_normalize(Vector3::new(p.x, p.y, p.z)),

            // For the other cases, we just fall back on multiple uses of the SDF.
            _ => {
                let offset = Vector3::new(0.00001, 0.0, 0.0);
                let dist = self.sdf(p);
                let px = self.sdf(&(p - offset.xyy()));
                let py = self.sdf(&(p - offset.yxy()));
                let pz = self.sdf(&(p - offset.yyx()));
                Unit::new_normalize(Vector3::new(dist.0 - px.0, dist.0 - py.0, dist.0 - pz.0))
            }
        }
    }
}

/// Returns the difference between the right and left distances, `h` which is the linear
/// interpolation value between the two distances, and the composite distance.
fn smooth_union_parts(k: f32, left: Distance, right: Distance) -> (f32, f32, Distance) {
    let diff = right.0 - left.0;

    let h = math::clamp(0.0, 1.0, 0.5 + 0.5 * diff / k);
    let factor = k * h * (1.0 - h);

    (diff, h, Distance(math::mix(right.0, left.0, h) - factor))
}

impl Node {
    pub fn sdf(&self, scene: &Scene, id: NodeId, point: &Point3<f32>) -> SDFResult {
        match self {
            Node::Prim { prim } => SDFResult {
                id,
                material: None,
                object: point.clone(),
                normal: prim.normal(point),
                distance: prim.sdf(point),
            },

            Node::Group { union, nodes } => {
                let mut res = nodes
                    .iter()
                    .copied()
                    .map(|id| scene.node(id).sdf(scene, id, point))
                    .min_by_key(|result| result.distance)
                    .unwrap();

                if *union {
                    res.id = id;
                    res.object = *point;
                }

                res
            }

            Node::SmoothUnion { k, left, right } => {
                let mut left = scene.node(*left).sdf(scene, *left, point);
                let right = scene.node(*right).sdf(scene, *right, point);

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
                        left.normal = self.normal_sdf(scene, id, point, left.distance);
                    }
                }

                left
            }

            Node::Transform { transform, node } => {
                let mut res = scene
                    .node(*node)
                    .sdf(scene, *node, &point.invert(transform));
                res.normal = res.normal.apply(transform);
                res
            }

            Node::Material { material, node } => {
                let mut res = scene.node(*node).sdf(scene, *node, point);
                res.material = Some(*material);
                res
            }
        }
    }

    /// Compute the normal by using the SDF. Useful as an intermediate for combination nodes that
    /// don't have a closed form normal computation.
    fn normal_sdf(
        &self,
        scene: &Scene,
        id: NodeId,
        p: &Point3<f32>,
        dist: Distance,
    ) -> Unit<Vector3<f32>> {
        let offset = Vector3::new(0.00001, 0.0, 0.0);
        let px = self.fast_sdf(scene, id, &(p - offset.xyy())).distance;
        let py = self.fast_sdf(scene, id, &(p - offset.yxy())).distance;
        let pz = self.fast_sdf(scene, id, &(p - offset.yyx())).distance;
        Unit::new_normalize(Vector3::new(dist.0 - px.0, dist.0 - py.0, dist.0 - pz.0))
    }

    // A version of `sdf` that only computes the distance and material information. Useful for
    // things like lighting calculations.
    pub fn fast_sdf(&self, scene: &Scene, id: NodeId, point: &Point3<f32>) -> FastSDFResult {
        match self {
            Node::Prim { prim } => FastSDFResult {
                distance: prim.sdf(point),
                material: None,
            },

            Node::Group { nodes, .. } => nodes
                .iter()
                .copied()
                .map(|id| scene.node(id).fast_sdf(scene, id, point))
                .min_by_key(|res| res.distance)
                .unwrap(),

            Node::SmoothUnion { k, left, right } => {
                let mut left = scene.node(*left).fast_sdf(scene, *left, point);
                let right = scene.node(*right).fast_sdf(scene, *right, point);

                let (diff, _, dist) = smooth_union_parts(*k, left.distance, right.distance);

                if diff < 0. {
                    left.material = right.material;
                }

                left.distance = dist;

                left
            }

            Node::Transform { transform, node } => {
                scene
                    .node(*node)
                    .fast_sdf(scene, *node, &point.invert(transform))
            }

            Node::Material { node, .. } => scene.node(*node).fast_sdf(scene, *node, point),
        }
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
pub struct Material {
    /// For now, the pattern of a surface is just a color.
    pub pattern: Color,

    /// The ambient reflection of this surface.
    pub ambient: f32,

    /// The diffuse reflection of this surface.
    pub diffuse: f32,

    /// The specular reflection of this surface.
    pub specular: f32,

    /// The shininess of the surface.
    pub shininess: f32,
}
