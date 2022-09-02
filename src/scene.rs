use nalgebra::{Unit, Vector3};

use crate::ray::Ray;

#[derive(Debug, Default)]
pub struct Scene {
    nodes: Vec<Node>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeId(u32);

/// Primitive shapes, centered at the origin.
#[derive(Debug)]
pub enum Prim {
    /// A plane with the given normal.
    Plane { normal: Unit<Vector3<f32>> },

    /// A sphere with the given radius.
    Sphere { radius: f32 },
}

/// Nodes in the scene graph.
#[derive(Debug)]
pub enum Node {
    /// Primitive shapes.
    Prim { prim: Prim },

    /// A group of nodes.
    Group { nodes: Vec<NodeId> },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Distance(pub f32);

#[derive(Debug)]
pub struct MarchResult {
    pub node: NodeId,
    pub distance: Distance,
    pub steps: u32,
}

#[derive(Debug)]
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
    id: NodeId,
    distance: Distance,
}

impl Scene {
    fn add_node(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
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

    pub fn group(&mut self, nodes: Vec<NodeId>) -> NodeId {
        self.add_node(Node::Group { nodes })
    }

    /// Fetch a node from the scene.
    pub fn node(&self, NodeId(id): NodeId) -> &Node {
        &self.nodes[id as usize]
    }

    /// March this ray through the scene rooted at `root`.
    pub fn march(
        &self,
        config: &MarchConfig,
        root: NodeId,
        mut ray: Ray,
    ) -> Option<MarchResult> {
        let mut total_dist = Distance::default();

        let node = self.node(root);

        for i in 0..config.max_steps {
            let result = node.sdf(self, root, &ray);
            let radius = result.distance.0;

            if radius < config.min_dist {
                return Some(MarchResult {
                    node: result.id,
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
}

impl Prim {
    /// Compute the distance from the current position of the ray to the primitive object.
    pub fn sdf(&self, ray: &Ray) -> Distance {
        let vec = ray.position_vector();
        match self {
            Prim::Plane { normal } => Distance(vec.dot(normal)),
            Prim::Sphere { radius } => Distance(vec.norm() - radius),
        }
    }
}

impl Node {
    pub fn sdf(&self, scene: &Scene, id: NodeId, ray: &Ray) -> SDFResult {
        match self {
            Node::Prim { prim } => SDFResult {
                id,
                distance: prim.sdf(ray),
            },

            Node::Group { nodes } => nodes
                .iter()
                .copied()
                .map(|id| scene.node(id).sdf(scene, id, ray))
                .min_by_key(|result| result.distance)
                .unwrap(),
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
