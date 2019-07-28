
use nalgebra::{Matrix4,Point3,Vector3};

/// Reflect a vector through a normal.
pub fn reflect(vec: &Vector3<f32>, normal: &Vector3<f32>) -> Vector3<f32> {
    let dot = vec.dot(normal);
    vec - (normal * (2.0 * dot))
}

#[derive(Debug)]
pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

pub struct SDFResult<Mat> {
    pub distance: f32,
    pub object_space_point: Point3<f32>,
    pub material: Mat,
}

impl Ray {

    pub const MIN_DIST: f32 = 0.001;
    pub const MAX_DIST: f32 = 100.0;

    pub fn new(origin: Point3<f32>, direction: Vector3<f32>) -> Self {
        Ray{ origin, direction }
    }

    pub fn position(&self, t: f32) -> Point3<f32> {
        self.origin + (self.direction * t)
    }

    pub fn march<SDF,Mat>(&self, max_steps: usize, sdf: SDF)
        -> Option<MarchResult<Mat>>
        where SDF: Fn(&Point3<f32>) -> SDFResult<Mat>,
    {
        let mut pos = self.origin.clone();
        let mut total_dist = 0.0;
        for i in 0 .. max_steps {
            let res = sdf(&pos);
            total_dist += res.distance;

            // the ray has failed to hit anything in the scene
            if total_dist >= Self::MAX_DIST {
                return None
            }

            pos += res.distance * self.direction;

            // the ray has gotten close enough to something to be considered a hit
            if res.distance <= Self::MIN_DIST {
                return Some(MarchResult{
                    steps: i,
                    distance: total_dist,
                    object_space_point: res.object_space_point.clone(),
                    world_space_point: pos,
                    material: res.material,
                })
            }
        }

        None
    }

    /// Make a new ray that has the given transformation applied to it.
    pub fn transform(&self, matrix: &Matrix4<f32>) -> Self {
        let origin = matrix.transform_point(&self.origin);
        let direction = matrix.transform_vector(&self.direction);
        Self::new(origin, direction)
    }

}

#[derive(Debug)]
pub struct MarchResult<Mat> {
    pub steps: usize,
    pub distance: f32,
    pub object_space_point: Point3<f32>,
    pub world_space_point: Point3<f32>,
    pub material: Mat,
}

impl<Mat> MarchResult<Mat> {

    /// Compute the normal to this result
    ///
    /// NOTE: the variable M here is different from Mat, to underscore the fact that the material
    /// is not used in the computation of the normal.
    pub fn normal<SDF,M>(&self, sdf: SDF)
        -> Vector3<f32>
        where SDF: Fn(&Point3<f32>) -> SDFResult<M>,
    {
        let res = sdf(&self.world_space_point);
        let offset = Vector3::new(0.001, 0.0, 0.0);

        let px = sdf(&(self.world_space_point - offset.xyy()));
        let py = sdf(&(self.world_space_point - offset.yxy()));
        let pz = sdf(&(self.world_space_point - offset.yyx()));
        Vector3::new(
            res.distance - px.distance,
            res.distance - py.distance,
            res.distance - pz.distance,
        ).normalize()
    }

}

#[test]
fn test_position() {
    let p = Ray::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));

    assert_eq!(p.position(0.0), p.origin);
    assert_eq!(p.position(1.0), Point3::new(0.0, 1.0, 0.0));
    assert_eq!(p.position(-1.0), Point3::new(0.0, -1., 0.0));
}

#[test]
fn test_transform() {
    let p = Ray::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));

    {
        let m = Matrix4::new_translation(&Vector3::new(0.0, 1.0, 0.0));
        assert_eq!(p.transform(&m).origin, Point3::new(0.0, 1.0, 0.0));
        assert_eq!(p.transform(&m).direction, p.direction);
    }

    {
        let m = Matrix4::new_rotation(Vector3::new(0.0, 1.0, 0.0));
        assert_eq!(p.transform(&m).origin, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(p.transform(&m).direction, Vector3::new(0.0, 1.0, 0.0));
    }

    {
        let m = Matrix4::new_nonuniform_scaling(&Vector3::new(2.0, 3.0, 4.0));
        assert_eq!(p.transform(&m).origin, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(p.transform(&m).direction, Vector3::new(0.0, 3.0, 0.0));
    }
}

#[test]
fn test_march() {
    use crate::shapes::{Scene,Shape};

    let ray = Ray::new(Point3::new(0.0, 0.0, -5.0), Vector3::new(0.0, 0.0, 1.0));

    let mut scene = Scene::new();
    let sphere = scene.sphere();
    let scaled = scene.add(Shape::uniform_scaling(2.0, sphere));
    let moved = scene.add(Shape::translation(&Vector3::new(5.0, 0.0, 0.0), sphere));

    // test an intersection
    let result = ray.march(100, |pt| scene.sdf_from(&scaled, pt)).expect("Failed to march ray");
    assert_eq!(result.distance, 3.0);

    // test a miss
    assert!(ray.march(100, |pt| scene.sdf_from(&moved, pt)).is_none());
}

#[test]
fn test_reflect() {
    {
        let v = Vector3::new(1.0, -1.0, 0.0);
        let normal = Vector3::new(0.0, 1.0, 0.0);
        assert_eq!(reflect(&v, &normal), Vector3::new(1.0, 1.0, 0.0));
    }

    {
        let v = Vector3::new(0.0, -1.0, 0.0);
        let s2d2 = f32::sqrt(2.0) / 2.0;
        let normal = Vector3::new(s2d2, s2d2, 0.0);
        let r = reflect(&v, &normal);

        assert!(r.x - 1.0 < std::f32::EPSILON);
        assert!(r.y < std::f32::EPSILON);
        assert_eq!(r.z, 0.0);
    }
}
