use nalgebra::Matrix4;

use crate::float::Float;
use crate::ray::Ray;
use crate::shape::Shape;

#[derive(Debug, Clone, Copy)]
pub struct NodeRef {
    index: usize,
}

#[derive(Default)]
pub struct Scene {
    nodes: Vec<Node>,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, shape: Node) -> NodeRef {
        let index = self.nodes.len();
        self.nodes.push(shape);
        NodeRef { index }
    }

    pub fn get(&self, NodeRef { index }: NodeRef) -> &Node {
        unsafe { self.nodes.get_unchecked(index) }
    }

    /// Add an object to the scene.
    pub fn object(&mut self, shape: Shape) -> NodeRef {
        self.add(Node::Object { shape })
    }

    /// Add a transformation node to the scene.
    pub fn transform(
        &mut self,
        matrix: Matrix4<Float>,
        scale_factor: Float,
        child_ref: NodeRef,
    ) -> NodeRef {
        let inverse = matrix.try_inverse().expect("Unable to invert matrix");
        self.add(Node::Transform {
            matrix,
            inverse,
            scale_factor,
            child_ref,
        })
    }
}

impl Scene {
    pub fn sdf(&self, ray: &Ray, from: NodeRef) -> Float {
        match *self.get(from) {
            Node::Object { ref shape } => shape.sdf(ray),

            Node::Transform {
                ref inverse,
                scale_factor,
                child_ref,
                ..
            } => {
                let ray = ray.transform(inverse);
                let dist = self.sdf(&ray, child_ref);
                dist * scale_factor
            }
        }
    }
}

pub enum Node {
    Object {
        shape: Shape,
    },

    Transform {
        matrix: Matrix4<Float>,
        inverse: Matrix4<Float>,
        scale_factor: Float,
        child_ref: NodeRef,
    },
}
