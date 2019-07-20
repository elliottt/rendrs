
use nalgebra::{Matrix4,Point3,Vector3};

#[derive(Debug)]
pub struct RayConfig {
    pub max_steps: usize,
    pub min_dist: f32,
    pub max_dist: f32,
}

impl RayConfig {
    pub const MAX_STEPS: usize = 100;
    pub const MIN_DIST: f32 = 0.01;
    pub const MAX_DIST: f32 = 100.0;
}

impl Default for RayConfig {
    fn default() -> Self {
        RayConfig{
            max_steps: RayConfig::MAX_STEPS,
            min_dist: RayConfig::MIN_DIST,
            max_dist: RayConfig::MAX_DIST,
        }
    }
}

/// Reflect a vector through a normal.
pub fn reflect(vec: &Vector3<f32>, normal: &Vector3<f32>) -> Vector3<f32> {
    let dot = vec.dot(normal);
    vec - (normal * 2.0 * dot)
}

pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

impl Ray {

    pub fn new(origin: Point3<f32>, direction: Vector3<f32>) -> Self {
        Ray{ origin, direction }
    }

    pub fn position(&self, t: f32) -> Point3<f32> {
        self.origin + (self.direction * t)
    }

    pub fn march<SDF,Mat>(&self, sdf: SDF)
        -> Option<MarchResult<Mat>>
        where SDF: Fn(&Point3<f32>) -> (f32,Mat),
    {
        self.march_with(&Default::default(), sdf)
    }

    pub fn march_with<SDF,Mat>(&self, cfg: &RayConfig, sdf: SDF)
        -> Option<MarchResult<Mat>>
        where SDF: Fn(&Point3<f32>) -> (f32,Mat),
    {
        let mut pos = self.origin.clone();
        let mut total_dist = 0.0;
        for i in 0 .. cfg.max_steps {
            let (dist,material) = sdf(&pos);
            total_dist += dist;

            // the ray has failed to hit anything in the scene
            if total_dist >= cfg.max_dist {
                return None
            }

            pos += dist * self.direction;

            // the ray has gotten close enough to something to be considered a hit
            if dist <= cfg.min_dist {
                return Some(MarchResult{ steps: i, distance: total_dist, point: pos, material })
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

pub struct MarchResult<Mat> {
    pub steps: usize,
    pub distance: f32,
    pub point: Point3<f32>,
    pub material: Mat,
}

impl<Mat> MarchResult<Mat> {

    /// Compute the normal to this result
    pub fn normal<SDF,M>(&self, sdf: SDF)
        -> Vector3<f32>
        where SDF: Fn(&Point3<f32>) -> (f32,M),
    {
        let (dist,_) = sdf(&self.point);
        let offset = Vector3::new(0.001, 0.0, 0.0);

        let (px,_) = sdf(&(self.point - offset.xyy()));
        let (py,_) = sdf(&(self.point - offset.yxy()));
        let (pz,_) = sdf(&(self.point - offset.yyx()));
        Vector3::new(dist - px, dist - py, dist - pz).normalize()
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
    let scaled = scene.add(Shape::uniform_scaling(2.0, sphere.clone()));
    let moved = scene.add(Shape::translation(&Vector3::new(5.0, 0.0, 0.0), sphere.clone()));

    // test an intersection
    let result = ray.march(|pt| scene.sdf(&scaled, pt)).expect("Failed to march ray");
    assert_eq!(result.distance, 3.0);

    // test a miss
    assert!(ray.march(|pt| scene.sdf(&moved, pt)).is_none());
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
        assert!(r.z == 0.0);
    }
}
