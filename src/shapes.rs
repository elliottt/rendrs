
use nalgebra::{Vector3,Point3,Matrix4};

use crate::material::{MaterialId};
use crate::pattern::{PatternId};
use crate::ray::{SDFResult};
use crate::scene::{Scene};

#[derive(Copy,Clone,Ord,PartialOrd,Eq,PartialEq,Debug)]
pub struct ShapeId(usize);

#[derive(Debug)]
pub struct Shapes {
    shapes: Vec<Shape>,
}

impl Shapes {
    pub fn new() -> Self {
        Shapes { shapes: Vec::with_capacity(10) }
    }

    pub fn add_shape(&mut self, shape: Shape) -> ShapeId {
        self.shapes.push(shape);
        ShapeId(self.shapes.len() - 1)
    }

    pub fn get_shape(&self, sid: ShapeId) -> &Shape {
        unsafe { self.shapes.get_unchecked(sid.0) }
    }
}

#[derive(Debug,Clone)]
pub enum PrimShape {
    /// The unit sphere
    Sphere,

    /// X-Z plane
    XZPlane,
}

impl PrimShape {
    fn sdf(&self, point: &Point3<f32>) -> f32 {
        match self {
            PrimShape::Sphere => {
                let magnitude = Vector3::new(point.x, point.y, point.z).magnitude();
                magnitude - 1.0
            },

            PrimShape::XZPlane => {
                point.y
            }
        }
    }

}

#[derive(Debug,Clone)]
pub enum Shape {
    /// The unit sphere
    PrimShape{
        shape: PrimShape,
    },

    /// Union together a bunch of nodes
    Union{
        nodes: Vec<ShapeId>,
    },

    /// Subtract one node from another
    Subtract{
        first: ShapeId,
        second: ShapeId,
    },

    /// A transformation applied to a sub-graph
    Transform{
        matrix: Matrix4<f32>,
        node: ShapeId,
    },

    /// Scaling must be handled differently for an SDF
    UniformScale{
        amount: f32,
        node: ShapeId,
    },

    /// Apply this material to the sub-graph
    Material{
        pattern: PatternId,
        material: MaterialId,
        node: ShapeId,
    },
}

impl Shape {
    pub fn sdf<'a>(&self, scene: &'a Scene, point: &Point3<f32>) -> SDFResult<(PatternId,MaterialId)> {
        match self {
            Shape::PrimShape{ shape } => {
                SDFResult{
                    distance: shape.sdf(point),
                    object_space_point: point.clone(),
                    material: (scene.default_pattern,scene.default_material),
                }
            },

            Shape::Union{nodes} => {
                let mut res = nodes
                    .iter()
                    .map(|node| scene.sdf_from(*node, point))
                    .min_by(|a,b| a.distance.partial_cmp(&b.distance).expect("failed to compare"))
                    .expect("Missing nodes to union");

                // override the object space coordinate to be relative to the whole group, not the
                // individual where the hit occurred
                res.object_space_point = point.clone();
                res
            },

            Shape::Subtract{ first, second } => {
                let mut ra = scene.sdf_from(*first, point);
                let rb = scene.sdf_from(*second, point);
                ra.distance = f32::max(ra.distance, -rb.distance);
                ra
            },

            Shape::Transform{ matrix, node } => {
                let p = matrix.transform_point(point);
                scene.sdf_from(*node, &p)
            },

            Shape::UniformScale{ amount, node } => {
                let p = point / *amount;
                let mut res = scene.sdf_from(*node, &p);
                res.distance *= amount;
                res
            },

            Shape::Material{ pattern, material, node } => {
                let mut res = scene.sdf_from(*node, point);
                res.material = (*pattern,*material);
                res
            }
        }
    }

    pub fn transform(matrix: &Matrix4<f32>, node: ShapeId) -> Self {
        let inv = matrix.try_inverse().expect("Unable to invert transformation matrix");
        Shape::Transform{ matrix: inv, node }
    }

    pub fn rotation(axisangle: Vector3<f32>, node: ShapeId) -> Self {
        Self::transform(&Matrix4::new_rotation(axisangle), node)
    }

    /// Translate the sub-graph by the given vector.
    pub fn translation(vec: &Vector3<f32>, node: ShapeId) -> Self {
        Self::transform(&Matrix4::new_translation(vec), node)
    }

    /// Scale each dimension by a constant amount.
    pub fn uniform_scaling(amount: f32, node: ShapeId) -> Self {
        Shape::UniformScale{ amount, node }
    }

    pub fn union(nodes: Vec<ShapeId>) -> Self {
        Shape::Union{ nodes: nodes.into() }
    }

    pub fn subtract(first: ShapeId, second: ShapeId) -> Self {
        Shape::Subtract{ first, second }
    }

    pub fn material(pattern: PatternId, material: MaterialId, node: ShapeId) -> Self {
        Shape::Material{ pattern, material, node }
    }
}
