
use nalgebra::{Vector3,Point3,Matrix4};

use crate::canvas::Color;
use crate::ray::reflect;

#[derive(Copy,Clone,Ord,PartialOrd,Eq,PartialEq,Debug)]
pub struct NodeId(usize);

#[derive(Copy,Clone,Ord,PartialOrd,Eq,PartialEq,Debug)]
pub struct MaterialId(usize);

#[derive(Clone,Debug)]
pub enum Material {

    /// Phong shaded material
    Phong{
        color: Color,
        ambient: f32,
        diffuse: f32,
        specular: f32,
        shininess: f32,
    },

}

impl Default for Material {
    fn default() -> Self {
        Material::Phong{
            color: Color::new(1.0, 1.0, 1.0),
            ambient: 0.1,
            diffuse: 0.9,
            specular: 0.9,
            shininess: 200.0,
        }
    }
}

impl Material {
    pub fn lighting(
        &self,
        light: &Light,
        point: &Point3<f32>,
        dir: &Vector3<f32>,
        normal: &Vector3<f32>,
        visible: bool,
    ) -> Color {
        match self {
            Material::Phong{ color, ambient, diffuse, specular, shininess } => {
                let effectivec = color * &light.color;
                let lightv = (light.position - point).normalize();
                let ambientc = &effectivec * *ambient;
                let light_dot_normal = lightv.dot(normal);

                let diffuse_specular =
                    if !visible || light_dot_normal < 0.0 {
                        Color::black()
                    } else {
                        let specularc = {
                            let reflectv = reflect(& -lightv, normal);
                            let reflect_dot_eye = reflectv.dot(dir);
                            if reflect_dot_eye <= 0.0 {
                                Color::black()
                            } else {
                                let factor = reflect_dot_eye.powf(*shininess);
                                &light.color * (specular * factor)
                            }
                        };

                        let diffusec = &effectivec * (diffuse * light_dot_normal);
                        &diffusec + &specularc
                    };

                &ambientc + &diffuse_specular
            },
        }
    }
}

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
        material: MaterialId,
        node: NodeId,
    },
}

impl Shape {
    fn sdf<'a>(&self, scene: &'a Scene, point: &Point3<f32>) -> (f32,MaterialId) {
        match self {
            Shape::PrimShape{ shape } => {
                (shape.sdf(point), scene.default_material())
            },

            Shape::Union{nodes} => {
                nodes
                    .iter()
                    .map(|node| scene.sdf_from(node, point))
                    .min_by(|a,b| a.0.partial_cmp(&b.0).expect("failed to compare"))
                    .expect("Missing nodes to union")
            },

            Shape::Subtract{ first, second } => {
                let (da, mat) = scene.sdf_from(first, point);
                let (db, _) = scene.sdf_from(second, point);
                (f32::max(da, -db), mat)
            },

            Shape::Transform{ matrix, node } => {
                let p = matrix.transform_point(point);
                scene.sdf_from(node, &p)
            },

            Shape::UniformScale{ amount, node } => {
                let p = point / *amount;
                let (dist,mat) = scene.sdf_from(node, &p);
                (dist * amount, mat)
            },

            Shape::Material{ material, node } => {
                let (dist,_) = scene.sdf_from(node, point);
                (dist, material.clone())
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

    pub fn material(material: MaterialId, node: NodeId) -> Self {
        Shape::Material{ material, node }
    }
}

pub struct Light {
    pub position: Point3<f32>,
    pub color: Color,
}

pub struct Scene {
    members: Vec<Shape>,
    lights: Vec<Light>,
    materials: Vec<Material>,

    // the roots of the world
    world: Vec<NodeId>,
}

impl Scene {

    pub fn new() -> Self {
        let mut scene = Scene{
            members: Vec::new(),
            lights: Vec::new(),
            materials: Vec::new(),
            world: Vec::new(),
        };

        // record primitives
        scene.add(Shape::PrimShape{ shape: PrimShape::Sphere });
        scene.add(Shape::PrimShape{ shape: PrimShape::XZPlane });
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

    pub fn iter_lights(&self) -> impl Iterator<Item=&Light> {
        self.lights.iter()
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

    pub fn default_material(&self) -> MaterialId {
        MaterialId(0)
    }

    pub fn sdf(&self, point: &Point3<f32>) -> (f32,MaterialId) {
        self.world
            .iter()
            .map(|root| self.sdf_from(root, point))
            .min_by(|a,b| a.0.partial_cmp(&b.0).expect("failed to compare"))
            .expect("Empty world")
    }

    pub fn sdf_from(&self, root: &NodeId, point: &Point3<f32>) -> (f32,MaterialId) {
        unsafe { self.members.get_unchecked(root.0).sdf(self, point) }
    }

}

#[test]
fn test_lighting() {
    let m = Material::default();
    let pos = Point3::origin();

    {
        let eyev = Vector3::new(0.0, 0.0, -1.0);
        let normalv = Vector3::new(0.0, 0.0, -1.0);
        let light = Light{
            position: Point3::new(0.0, 0.0, -10.0),
            color: Color::new(1.0, 1.0, 1.0)
        };
        let res = m.lighting(&light, &pos, &eyev, &normalv, true);
        assert_eq!(res.r(), 1.9);
        assert_eq!(res.g(), 1.9);
        assert_eq!(res.b(), 1.9);

        let res = m.lighting(&light, &pos, &eyev, &normalv, false);
        assert_eq!(res.r(), 0.1);
        assert_eq!(res.g(), 0.1);
        assert_eq!(res.b(), 0.1);
    }

    {
        let s2d2 = f32::sqrt(2.0) / 2.0;
        let eyev = Vector3::new(0.0, s2d2, -s2d2);
        let normalv = Vector3::new(0.0, 0.0, -1.0);
        let light = Light{
            position: Point3::new(0.0, 0.0, -10.0),
            color: Color::new(1.0, 1.0, 1.0)
        };
        let res = m.lighting(&light, &pos, &eyev, &normalv, true);
        assert_eq!(res.r(), 1.0);
        assert_eq!(res.g(), 1.0);
        assert_eq!(res.b(), 1.0);
    }
}
