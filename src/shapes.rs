
use nalgebra::{Vector2,Vector3,Point3,Matrix4};

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

    /// The unit cylinder
    Cylinder,

    /// The unit cube
    Cube,

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

            PrimShape::Cylinder => {
                let xz_mag = Vector2::new(point.x, point.z).magnitude();
                (xz_mag - 1.0).max(point.y.abs() - 1.0)
            },

            PrimShape::Cube => {
                let x = point.x.abs() - 1.0;
                let y = point.y.abs() - 1.0;
                let z = point.z.abs() - 1.0;
                let diff = x.max(y.max(z)).min(0.0);
                Vector3::new(x.max(0.0), y.max(0.0), z.max(0.0)).magnitude() + diff
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

    /// Union together two nodes, with a smoothing factor
    SmoothUnion{
        k: f32,
        first: ShapeId,
        second: ShapeId,
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

    /// Onion the object
    Onion{
        thickness: f32,
        node: ShapeId,
    }
}

impl Shape {
    pub fn sdf(&self, scene: &Scene, point: &Point3<f32>, result: &mut SDFResult) {
        match self {
            Shape::PrimShape{ shape } => {
                result.distance = shape.sdf(point);
                result.object_space_point = point.clone();
            },

            Shape::Union{nodes} => {
                let mut tmp = result.clone();

                for node in nodes {
                    scene.get_shape(*node).sdf(scene, point, &mut tmp);
                    if tmp.distance < result.distance {
                        result.distance = tmp.distance;
                        result.material = tmp.material;
                        result.pattern = tmp.pattern;
                    }
                }

                // Make texturing relative to the union, not the individual object
                result.object_space_point = point.clone();
            },

            Shape::SmoothUnion{ k, first, second } => {
                use crate::utils::{mix,clamp};

                let mut tmp = result.clone();

                scene.get_shape(*first).sdf(scene, point, result);
                scene.get_shape(*second).sdf(scene, point, &mut tmp);

                let diff = tmp.distance - result.distance;

                if diff < 0.0 {
                    result.material = tmp.material;
                    result.pattern = tmp.pattern;
                }

                let h = clamp(0.5 + 0.5*diff / k, 0.0, 1.0);
                result.distance = mix(tmp.distance, result.distance, h) - k * h * (1.0 - h);

                // Make texturing relative to the union, not the individual object
                result.object_space_point = point.clone();
            },

            Shape::Subtract{ first, second } => {
                scene.get_shape(*first).sdf(scene, point, result);

                // figure out the distance for the part being subtracted
                let mut tmp = result.clone();
                scene.get_shape(*second).sdf(scene, point, &mut tmp);
                let sub = -tmp.distance;

                if result.distance <= sub {
                    result.distance = sub;
                    result.material = tmp.material;
                    result.pattern = tmp.pattern;
                }
            },

            Shape::Transform{ matrix, node } => {
                let p = matrix.transform_point(point);
                scene.get_shape(*node).sdf(scene, &p, result);
            },

            Shape::UniformScale{ amount, node } => {
                let p = point / *amount;
                scene.get_shape(*node).sdf(scene, &p, result);
                result.distance *= amount;
            },

            Shape::Material{ pattern, material, node } => {
                scene.get_shape(*node).sdf(scene, point, result);
                result.material = *material;
                result.pattern = *pattern;
            },

            Shape::Onion{ thickness, node } => {
                scene.get_shape(*node).sdf(scene, point, result);
                result.distance = result.distance.abs() - thickness;
            },
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
        Shape::Union{ nodes }
    }

    pub fn smooth_union(k: f32, first: ShapeId, second: ShapeId) -> Self {
        Shape::SmoothUnion{ k, first, second }
    }

    pub fn subtract(first: ShapeId, second: ShapeId) -> Self {
        Shape::Subtract{ first, second }
    }

    pub fn material(pattern: PatternId, material: MaterialId, node: ShapeId) -> Self {
        Shape::Material{ pattern, material, node }
    }

    pub fn onion(thickness: f32, node: ShapeId) -> Self {
        Shape::Onion{ thickness, node }
    }
}

#[test]
fn test_cube() {
    use crate::assert_eq_f32;

    let shape = PrimShape::Cube;
    {
        let point = Point3::new(1.0, 0.0, 0.0);
        assert_eq_f32!(shape.sdf(&point), 0.0);
    }
    {
        let point = Point3::new(0.5, 0.0, 0.0);
        assert_eq_f32!(shape.sdf(&point), -0.5);
    }
    {
        let point = Point3::new(0.0, 0.0, 0.0);
        assert_eq_f32!(shape.sdf(&point), -1.0);
    }
}
