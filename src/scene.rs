use nalgebra::Matrix4;

use crate::float::Float;
use crate::material::Material;
use crate::ray::Ray;
use crate::shape::Shape;

#[derive(Debug, Clone, Copy)]
pub struct NodeRef {
    index: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialRef {
    index: usize,
}

#[derive(Default)]
pub struct Scene {
    nodes: Vec<Node>,
    materials: Vec<Material>,
    root: Option<NodeRef>,
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

    pub fn add_material(&mut self, material: Material) -> MaterialRef {
        let index = self.materials.len();
        self.materials.push(material);
        MaterialRef { index }
    }

    pub fn get_material(&self, MaterialRef { index }: MaterialRef) -> &Material {
        unsafe { self.materials.get_unchecked(index) }
    }

    pub fn set_root(&mut self, node: NodeRef) {
        self.root = Some(node)
    }

    pub fn get_root(&self) -> Option<NodeRef> {
        self.root
    }

    /// Add an object to the scene.
    pub fn object(&mut self, shape: Shape, material: MaterialRef) -> NodeRef {
        self.add(Node::Object { shape, material })
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

#[derive(Debug)]
pub struct SDFResult {
    pub distance: Float,
    pub material: MaterialRef,
}

impl Default for SDFResult {
    fn default() -> Self {
        SDFResult {
            distance: 0.,
            material: MaterialRef { index: 0 },
        }
    }
}

impl SDFResult {
    pub fn reset(&mut self) {
        self.distance = 0.;
        self.material = MaterialRef { index: 0 };
    }
}

impl Scene {
    pub fn sdf(&self, result: &mut SDFResult, ray: &Ray, from: NodeRef) {
        match self.get(from) {
            Node::Object { shape, material } => {
                result.distance = shape.sdf(ray);
                result.material = *material;
            }

            Node::Transform {
                ref inverse,
                scale_factor,
                child_ref,
                ..
            } => {
                let ray = ray.transform(inverse);
                self.sdf(result, &ray, *child_ref);
                result.distance *= scale_factor;
            }
        }
    }
}

pub enum Node {
    Object {
        shape: Shape,
        material: MaterialRef,
    },

    Transform {
        matrix: Matrix4<Float>,
        inverse: Matrix4<Float>,
        scale_factor: Float,
        child_ref: NodeRef,
    },
}
