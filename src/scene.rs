use nalgebra::{Point3, Unit, Vector2, Vector3};

use crate::transform::{ApplyTransform, Transform};

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
    Group { nodes: Vec<NodeId> },

    /// Apply this Transform the node.
    Transform { transform: Transform, node: NodeId },
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

    /// The distance between the world-space ray and this object.
    pub distance: Distance,
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
        self.add_node(Node::Group { nodes })
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

    /// Fetch a node from the scene.
    pub fn node(&self, NodeId(id): NodeId) -> &Node {
        &self.nodes[id as usize]
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
                let x = p.x.abs() - *width;
                let y = p.y.abs() - *height;
                let z = p.z.abs() - *depth;
                let diff = x.max(y.max(x)).min(0.0);
                Distance(Vector3::new(x.max(0.), y.max(0.), z.max(0.)).norm() + diff)
            }

            Prim::Torus { hole, radius } => {
                let q = Vector2::new(p.xz().norm() - hole, p.y);
                return Distance(q.norm() - radius);
            }
        }
    }
}

impl Node {
    pub fn sdf(&self, scene: &Scene, id: NodeId, point: &Point3<f32>) -> SDFResult {
        match self {
            Node::Prim { prim } => SDFResult {
                id,
                object: point.clone(),
                distance: prim.sdf(point),
            },

            Node::Group { nodes } => nodes
                .iter()
                .copied()
                .map(|id| scene.node(id).sdf(scene, id, point))
                .min_by_key(|result| result.distance)
                .unwrap(),

            Node::Transform { transform, node } => {
                scene
                    .node(*node)
                    .sdf(scene, *node, &point.invert(transform))
            }
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
