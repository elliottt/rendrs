use crate::{
    bounding_volume::AABB,
    material::{Light, Material, MaterialId, Materials},
    pattern::{Pattern, PatternId, Patterns},
    ray::{Ray, SDFResult},
    shapes::{Shape, ShapeId, Shapes},
};

#[derive(Debug)]
pub struct Scene {
    shapes: Shapes,

    lights: Vec<Light>,

    materials: Materials,
    pub default_material: MaterialId,

    patterns: Patterns,
    pub default_pattern: PatternId,

    // the root of the world
    root: Option<ShapeId>,
}

impl Scene {
    pub fn new() -> Self {
        let mut materials = Materials::new();
        let default_material = materials.add_material(Material::default());

        let mut patterns = Patterns::new();
        let default_pattern = patterns.add_pattern(Pattern::default());

        Scene {
            shapes: Shapes::new(),
            lights: Vec::new(),
            materials,
            default_material,
            patterns,
            default_pattern,
            root: None,
        }
    }

    pub fn add(&mut self, shape: Shape) -> ShapeId {
        self.shapes.add_shape(shape)
    }

    pub fn set_root(&mut self, node: ShapeId) {
        self.root = Some(node);
    }

    pub fn get_shape(&self, shape: ShapeId) -> &'_ Shape {
        self.shapes.get_shape(shape)
    }

    pub fn get_bounding_volume(&self, shape: ShapeId) -> AABB {
        self.shapes.get_shape(shape).bounding_volume(self)
    }

    pub fn add_light(&mut self, light: Light) {
        self.lights.push(light);
    }

    pub fn num_lights(&self) -> usize {
        self.lights.len()
    }

    pub fn iter_lights(&self) -> impl Iterator<Item = &Light> {
        self.lights.iter()
    }

    pub fn add_pattern(&mut self, pattern: Pattern) -> PatternId {
        self.patterns.add_pattern(pattern)
    }

    pub fn get_pattern(&self, pattern: PatternId) -> &'_ Pattern {
        self.patterns.get_pattern(pattern)
    }

    pub fn add_material(&mut self, material: Material) -> MaterialId {
        self.materials.add_material(material)
    }

    pub fn get_material(&self, mid: MaterialId) -> &'_ Material {
        self.materials.get_material(mid)
    }

    pub fn sdf(&self, ray: &Ray) -> SDFResult {
        self.root
            .map(|root| self.sdf_from(root.clone(), ray))
            .expect("empty world")
    }

    pub fn sdf_from(&self, root: ShapeId, ray: &Ray) -> SDFResult {
        let mut result = SDFResult {
            distance: std::f32::INFINITY,
            object_space_point: ray.origin.clone(),
            object_id: root,
            material: self.default_material,
            pattern: self.default_pattern,
        };
        self.get_shape(root).sdf(self, root, ray, &mut result);
        result
    }
}
