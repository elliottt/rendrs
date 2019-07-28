
use nalgebra::{Vector3,Point3,Matrix4};

use crate::canvas::Color;
use crate::ray::{SDFResult};
use crate::pattern::{PatternId,Pattern,Patterns};
use crate::material::{Light,MaterialId,Material};

#[derive(Copy,Clone,Ord,PartialOrd,Eq,PartialEq,Debug)]
pub struct NodeId(usize);

#[derive(Clone)]
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

#[derive(Clone)]
pub enum Shape {
    /// The unit sphere
    PrimShape{
        shape: PrimShape,
    },

    /// Union together a bunch of nodes
    Union{
        nodes: Vec<NodeId>,
    },

    /// Subtract one node from another
    Subtract{
        first: NodeId,
        second: NodeId,
    },

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

    /// Apply this material to the sub-graph
    Material{
        pattern: PatternId,
        material: MaterialId,
        node: NodeId,
    },
}

impl Shape {
    fn sdf<'a>(&self, scene: &'a Scene, point: &Point3<f32>) -> SDFResult<(PatternId,MaterialId)> {
        match self {
            Shape::PrimShape{ shape } => {
                SDFResult{
                    distance: shape.sdf(point),
                    object_space_point: point.clone(),
                    material: (scene.default_pattern(),scene.default_material()),
                }
            },

            Shape::Union{nodes} => {
                nodes
                    .iter()
                    .map(|node| scene.sdf_from(node, point))
                    .min_by(|a,b| a.distance.partial_cmp(&b.distance).expect("failed to compare"))
                    .expect("Missing nodes to union")
            },

            Shape::Subtract{ first, second } => {
                let mut ra = scene.sdf_from(first, point);
                let rb = scene.sdf_from(second, point);
                ra.distance = f32::max(ra.distance, -rb.distance);
                ra
            },

            Shape::Transform{ matrix, node } => {
                let p = matrix.transform_point(point);
                scene.sdf_from(node, &p)
            },

            Shape::UniformScale{ amount, node } => {
                let p = point / *amount;
                let mut res = scene.sdf_from(node, &p);
                res.distance *= amount;
                res
            },

            Shape::Material{ pattern, material, node } => {
                let mut res = scene.sdf_from(node, point);
                res.material = (*pattern,*material);
                res
            }
        }
    }

    pub fn transform(matrix: &Matrix4<f32>, node: NodeId) -> Self {
        let inv = matrix.try_inverse().expect("Unable to invert transformation matrix");
        Shape::Transform{ matrix: inv, node }
    }

    pub fn rotation(axisangle: Vector3<f32>, node: NodeId) -> Self {
        Self::transform(&Matrix4::new_rotation(axisangle), node)
    }

    /// Translate the sub-graph by the given vector.
    pub fn translation(vec: &Vector3<f32>, node: NodeId) -> Self {
        Self::transform(&Matrix4::new_translation(vec), node)
    }

    /// Scale each dimension by a constant amount.
    pub fn uniform_scaling(amount: f32, node: NodeId) -> Self {
        Shape::UniformScale{ amount, node }
    }

    pub fn union(nodes: Vec<NodeId>) -> Self {
        Shape::Union{ nodes: nodes.into() }
    }

    pub fn subtract(first: NodeId, second: NodeId) -> Self {
        Shape::Subtract{ first, second }
    }

    pub fn material(pattern: PatternId, material: MaterialId, node: NodeId) -> Self {
        Shape::Material{ pattern, material, node }
    }
}

pub struct Scene {
    members: Vec<Shape>,
    lights: Vec<Light>,
    materials: Vec<Material>,
    patterns: Patterns,

    // the roots of the world
    world: Vec<NodeId>,
}

impl Scene {

    pub fn new() -> Self {
        let mut scene = Scene{
            members: Vec::new(),
            lights: Vec::new(),
            materials: Vec::new(),
            patterns: Patterns::new(),
            world: Vec::new(),
        };

        // record primitives
        scene.add(Shape::PrimShape{ shape: PrimShape::Sphere });
        scene.add(Shape::PrimShape{ shape: PrimShape::XZPlane });
        scene.add_pattern(Pattern::solid(Color::white()));
        scene.add_material(Default::default());

        scene
    }

    pub fn add(&mut self, shape: Shape) -> NodeId {
        self.members.push(shape);
        NodeId(self.members.len() - 1)
    }

    pub fn add_root(&mut self, node: NodeId) {
        self.world.push(node);
    }

    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    }

    pub fn num_lights(&self) -> usize {
        self.lights.len()
    }

    pub fn iter_lights(&self) -> impl Iterator<Item=&Light> {
        self.lights.iter()
    }

    pub fn add_pattern(&mut self, pattern: Pattern) -> PatternId {
        self.patterns.add_pattern(pattern)
    }

    pub fn get_pattern(&self, pattern: PatternId) -> &'_ Pattern {
        self.patterns.get_pattern(pattern)
    }

    pub fn add_material(&mut self, material: Material) -> MaterialId {
        self.materials.push(material);
        MaterialId(self.materials.len() - 1)
    }

    pub fn get_material(&self, material: MaterialId) -> &'_ Material {
        // you can't remove a material, so this will always be valid, as you can't construct
        // arbitrary MaterialId values.
        unsafe { self.materials.get_unchecked(material.0) }
    }

    pub fn sphere(&self) -> NodeId {
        NodeId(0)
    }

    pub fn xz_plane(&self) -> NodeId {
        NodeId(1)
    }

    pub fn default_pattern(&self) -> PatternId {
        PatternId(0)
    }

    pub fn default_material(&self) -> MaterialId {
        MaterialId(0)
    }

    pub fn sdf(&self, point: &Point3<f32>) -> SDFResult<(PatternId,MaterialId)> {
        self.world
            .iter()
            .map(|root| self.sdf_from(root, point))
            .min_by(|a,b| a.distance.partial_cmp(&b.distance).expect("failed to compare"))
            .expect("Empty world")
    }

    pub fn sdf_from(&self, root: &NodeId, point: &Point3<f32>) -> SDFResult<(PatternId,MaterialId)> {
        unsafe { self.members.get_unchecked(root.0).sdf(self, point) }
    }

}
