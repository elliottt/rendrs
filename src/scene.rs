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
pub struct Distance(f32);

#[derive(Debug)]
pub struct MarchResult {
    pub node: NodeId,
    pub distance: Distance,
    pub steps: usize,
}

#[derive(Debug)]
pub struct SDFResult {
    id: NodeId,
    distance: Distance,
}

impl Scene {
    /// Construct a plane with the given normal in the scene.
    pub fn plane(&mut self, normal: Unit<Vector3<f32>>) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(Node::Prim {
            prim: Prim::Plane { normal },
        });
        id
    }

    /// Construct a sphere with the given normal in the scene.
    pub fn sphere(&mut self, radius: f32) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(Node::Prim {
            prim: Prim::Sphere { radius },
        });
        id
    }

    pub fn group(&mut self, nodes: Vec<NodeId>) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(Node::Group {
            nodes,
        });
        id
    }

    /// Fetch a node from the scene.
    pub fn node(&self, NodeId(id): NodeId) -> &Node {
        &self.nodes[id as usize]
    }

    /// March this ray through the scene rooted at `root`.
    pub fn march(
        &self,
        min_dist: f32,
        max_dist: f32,
        max_steps: usize,
        root: NodeId,
        mut ray: Ray,
    ) -> Option<MarchResult> {
        let mut total_dist = Distance::default();

        let node = self.node(root);

        for i in 0..max_steps {
            let result = node.sdf(self, root, &ray);
            let radius = result.distance.0;

            if radius < min_dist {
                return Some(MarchResult {
                    node: result.id,
                    distance: total_dist,
                    steps: i,
                });
            }

            total_dist.0 += radius;

            if total_dist.0 > max_dist {
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
        match self {
            Prim::Plane { normal } => Distance(ray.position.dot(normal)),
            Prim::Sphere { radius } => Distance(ray.position.norm() - radius),
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
