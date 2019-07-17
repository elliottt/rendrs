
use nalgebra::{Vector3,Point3,Matrix4};
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

    /// Scaling must be handled differently for an SDF
    UniformScale{
        amount: f32,
        node: NodeId,
    },
}

impl Shape {
    fn sdf(&self, scene: &Scene, point: &Point3<f32>) -> f32 {
        match self {
            Shape::Sphere => {
                let magnitude = (point - Point3::origin()).magnitude();
                magnitude - 1.0
            },

            Shape::Transform{ matrix, node } => {
                let p = matrix.transform_point(point);
                scene.sdf(node, &p)
            },

            Shape::UniformScale{ amount, node } => {
                let p = point / *amount;
                scene.sdf(node, &p) * amount
            },
        }
    }

    fn transform(matrix: &Matrix4<f32>, node: NodeId) -> Self {
        let inv = matrix.try_inverse().expect("Unable to invert transformation matrix");
        Shape::Transform{ matrix: inv, node }
    }

    /// Translate the sub-graph by the given vector.
    pub fn translation(vec: &Vector3<f32>, node: NodeId) -> Self {
        Self::transform(&Matrix4::new_translation(vec), node)
    }

    /// Scale each dimension by a constant amount.
    pub fn uniform_scaling(amount: f32, node: NodeId) -> Self {
        Shape::UniformScale{ amount, node }
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

    pub fn sphere(&self) -> NodeId {
        NodeId(0)
    }

    pub fn sdf(&self, root: &NodeId, point: &Point3<f32>) -> f32 {
        self.members.get(root).expect("Missing node").sdf(self, point)
    }

}
