
use nalgebra::{Vector2,Vector3,Point3,Matrix4};

use crate::bounding_volume::{AABB};
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
    Sphere,

    Cylinder{
        radius: f32,
        length: f32,
    },

    RectangularPrism{
        width: f32,
        height: f32,
        depth: f32,
    },

    Triangle{
        a: Point3<f32>,
        b: Point3<f32>,
        c: Point3<f32>,
        ba: Vector3<f32>,
        cb: Vector3<f32>,
        ac: Vector3<f32>,
        normal: Vector3<f32>,
    },

    XZPlane,
}

impl PrimShape {
    fn sdf(&self, point: &Point3<f32>) -> f32 {
        match self {
            PrimShape::Sphere => {
                let magnitude = Vector3::new(point.x, point.y, point.z).magnitude();
                magnitude - 1.0
            },

            PrimShape::Cylinder{ radius, length } => {
                let xz_mag = Vector2::new(point.x, point.z).magnitude();
                (xz_mag - radius).max(point.y.abs() - length)
            },

            PrimShape::RectangularPrism{ width, height, depth } => {
                let x = point.x.abs() - width;
                let y = point.y.abs() - height;
                let z = point.z.abs() - depth;
                let diff = x.max(y.max(z)).min(0.0);
                Vector3::new(x.max(0.0), y.max(0.0), z.max(0.0)).magnitude() + diff
            },

            PrimShape::Triangle{ a, b, c, ba, cb, ac, normal } => {
                use crate::utils::{clamp,dot2};

                let pa = point - a;
                let pb = point - b;
                let pc = point - c;

                let sa = pa.dot(&ba.cross(&normal)).signum();
                let sb = pb.dot(&cb.cross(&normal)).signum();
                let sc = pc.dot(&ac.cross(&normal)).signum();

                if sa + sb + sc < 2.0 {
                    let d2a = dot2(&(ba*clamp(ba.dot(&pa) / dot2(&ba), 0.0, 1.0) - pa));
                    let d2b = dot2(&(cb*clamp(cb.dot(&pb) / dot2(&cb), 0.0, 1.0) - pb));
                    let d2c = dot2(&(ac*clamp(ac.dot(&pc) / dot2(&ac), 0.0, 1.0) - pc));
                    d2a.min(d2b).min(d2c)
                } else {
                    normal.dot(&pa).powi(2) / normal.dot(normal)
                }.sqrt()
            },

            PrimShape::XZPlane => {
                point.y
            }
        }
    }

    /// Construct a triangle.
    pub fn triangle(a: &Point3<f32>, b: &Point3<f32>, c: &Point3<f32>) -> Self {
        let ba = b - a;
        let cb = c - b;
        let ac = a - c;
        let normal = ba.cross(&ac);
        PrimShape::Triangle{
            a: a.clone(),
            b: b.clone(),
            c: c.clone(),
            ba, cb, ac, normal
        }
    }

    /// Construct a triangle with an explicit normal.
    pub fn triangle_with_normal(
        a: &Point3<f32>,
        b: &Point3<f32>,
        c: &Point3<f32>,
        normal: &Vector3<f32>,
    ) -> Self {
        let ba = b - a;
        let cb = c - b;
        let ac = a - c;
        PrimShape::Triangle{
            a: a.clone(),
            b: b.clone(),
            c: c.clone(),
            normal: normal.clone(),
            ba, cb, ac,
        }
    }

    /// Compute the bounding volume for this primitive.
    pub fn bounding_volume(&self) -> AABB {
        match self {
            PrimShape::Sphere => {
                AABB::new(
                    Point3::new(-1.0, -1.0, -1.0),
                    Point3::new( 1.0,  1.0,  1.0),
                )
            },

            PrimShape::Cylinder{ radius, length } => {
                AABB::new(
                    Point3::new(-radius, -radius, -length),
                    Point3::new(*radius, *radius, *length),
                )
            },

            PrimShape::RectangularPrism{ width, height, depth } => {
                AABB::new(
                    Point3::new(-width, -height, -depth),
                    Point3::new(*width, *height, *depth),
                )
            },

            PrimShape::Triangle{ a, b, c, .. } => {
                AABB::from_points(a, b).union_point(c)
            },

            PrimShape::XZPlane => {
                let mut aabb = AABB::max();
                aabb.max.y = 0.0;
                aabb
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

    /// A bunch of nodes grouped together
    Group{
        nodes: Vec<ShapeId>,
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

    /// Intersect nodes.
    Intersect{
        nodes: Vec<ShapeId>,
    },

    /// A transformation applied to a sub-graph
    Transform{
        matrix: Matrix4<f32>,
        inverse: Matrix4<f32>,
        scale_factor: f32,
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
    pub fn sdf(&self, scene: &Scene, self_id: ShapeId, point: &Point3<f32>, result: &mut SDFResult) {
        match self {
            Shape::PrimShape{ shape } => {
                result.object_id = self_id;
                result.distance = shape.sdf(point);
                result.object_space_point = point.clone();
            },

            // A group differs from a union in that the individual objects hit by the SDF are
            // maintained as the result -- the whole isn't considered the hit.
            Shape::Group{ nodes } => {
                result.distance = std::f32::INFINITY;
                let mut tmp = result.clone();

                for node in nodes {
                    scene.get_shape(*node).sdf(scene, *node, point, &mut tmp);
                    if tmp.distance < result.distance {
                        *result = tmp.clone();
                    }
                }
            },

            Shape::Union{ nodes } => {
                result.distance = std::f32::INFINITY;
                let mut tmp = result.clone();

                for node in nodes {
                    scene.get_shape(*node).sdf(scene, *node, point, &mut tmp);
                    if tmp.distance < result.distance {
                        result.distance = tmp.distance;
                        result.material = tmp.material;
                        result.pattern = tmp.pattern;
                    }
                }

                // Override the object id of the individual hit to be that of the entire union
                result.object_id = self_id;

                // Make texturing relative to the union, not the individual object
                result.object_space_point = point.clone();
            },

            Shape::SmoothUnion{ k, first, second } => {
                use crate::utils::{mix,clamp};

                let mut tmp = result.clone();

                scene.get_shape(*first).sdf(scene, *first, point, result);
                scene.get_shape(*second).sdf(scene, *second, point, &mut tmp);

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
                scene.get_shape(*first).sdf(scene, *first, point, result);

                // figure out the distance for the part being subtracted
                let mut tmp = result.clone();
                scene.get_shape(*second).sdf(scene, *second, point, &mut tmp);
                let sub = -tmp.distance;

                if result.distance <= sub {
                    result.distance = sub;

                    // keep texturing information from the shape that was subtracted.
                    result.material = tmp.material;
                    result.pattern = tmp.pattern;
                }

                // override the object id to be that of the subtraction
                result.object_id = self_id;
            },

            Shape::Intersect{ nodes } => {
                result.distance = std::f32::NEG_INFINITY;
                let mut tmp = result.clone();

                for node in nodes {
                    scene.get_shape(*node).sdf(scene, *node, point, &mut tmp);
                    if tmp.distance > result.distance {
                        result.distance = tmp.distance;
                        result.material = tmp.material;
                        result.pattern = tmp.pattern;
                    }
                }

                // override the object id to be that of the subtraction
                result.object_id = self_id;

                // Make texturing relative to the intersection, not the individual object
                result.object_space_point = point.clone();
            },

            Shape::Transform{ inverse, scale_factor, node, .. } => {
                let p = inverse.transform_point(point);
                scene.get_shape(*node).sdf(scene, *node, &p, result);
                result.distance *= *scale_factor;
            },

            Shape::Material{ pattern, material, node } => {
                scene.get_shape(*node).sdf(scene, *node, point, result);
                result.material = *material;
                result.pattern = *pattern;
            },

            Shape::Onion{ thickness, node } => {
                scene.get_shape(*node).sdf(scene, *node, point, result);
                result.distance = result.distance.abs() - thickness;
                result.object_id = self_id;
            },
        }
    }

    pub fn transform(matrix: &Matrix4<f32>, scale_factor: f32, node: ShapeId) -> Self {
        let inverse = matrix.try_inverse().expect("Unable to invert transformation matrix");
        Shape::Transform{ matrix: matrix.clone(), inverse, scale_factor, node }
    }

    pub fn rotation(axisangle: Vector3<f32>, node: ShapeId) -> Self {
        Self::transform(&Matrix4::new_rotation(axisangle), 1.0, node)
    }

    /// Translate the sub-graph by the given vector.
    pub fn translation(vec: &Vector3<f32>, node: ShapeId) -> Self {
        Self::transform(&Matrix4::new_translation(vec), 1.0, node)
    }

    /// Scale each dimension by a constant amount.
    pub fn uniform_scaling(amount: f32, node: ShapeId) -> Self {
        Self::transform(&Matrix4::new_scaling(amount), amount, node)
    }

    pub fn group(nodes: Vec<ShapeId>) -> Self {
        Shape::Group{ nodes }
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

    pub fn intersect(nodes: Vec<ShapeId>) -> Self {
        Shape::Intersect{ nodes }
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

    let shape = PrimShape::RectangularPrism{ width: 1.0, height: 1.0, depth: 1.0 };
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
