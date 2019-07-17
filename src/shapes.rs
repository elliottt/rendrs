
use nalgebra::{Point3,Matrix4};
use std::collections::{BTreeMap};

#[derive(Clone,Ord,PartialOrd,Eq,PartialEq)]
pub struct NodeId(usize);

#[derive(Clone)]
pub enum Shape {
    /// The unit sphere
    Sphere,

    /// A transformation applied to a sub-graph
    Transform{
        matrix: Matrix4<f32>,
        node: NodeId,
    },
}

impl Shape {
    fn sdf(&self, scene: &Scene, point: &Point3<f32>) -> f32 {
        match self {
            Shape::Sphere => {
                point.to_homogeneous().magnitude() - 1.0
            },

            Shape::Transform{ matrix, node } => {
                let p = matrix.transform_point(point);
                scene.sdf(node, &p)
            },
        }
    }
}

pub struct Scene {
    members: BTreeMap<NodeId,Shape>,
    next: usize,
}

impl Scene {

    pub fn new() -> Self {
        let mut scene = Scene { members: BTreeMap::new(), next: 0 };

        // record primitives
        scene.add(Shape::Sphere);

        scene
    }

    pub fn add(&mut self, shape: Shape) -> NodeId
    {
        let id = NodeId(self.next);
        self.next += 1;
        self.members.insert(id.clone(), shape);
        id
    }

    pub fn sphere(&mut self) -> NodeId {
        NodeId(0)
    }

    pub fn sdf(&self, root: &NodeId, point: &Point3<f32>) -> f32 {
        self.members.get(root).expect("Missing node").sdf(self, point)
    }

}
