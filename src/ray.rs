
use nalgebra::{Matrix4,Point3,Vector3};

pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

pub struct MarchResult {
    pub steps: usize,
    pub distance: f32,
    pub point: Point3<f32>,
}

impl Ray {

    pub fn new(origin: Point3<f32>, direction: Vector3<f32>) -> Self {
        Ray{ origin, direction }
    }

    pub fn position(&self, t: f32) -> Point3<f32> {
        self.origin + (self.direction * t)
    }

    const MAX_STEPS: usize = 100;
    const MIN_DIST: f32 = 0.01;
    const MAX_DIST: f32 = 100.0;

    pub fn march<SDF>(&self, sdf: SDF)
        -> Option<MarchResult>
        where SDF: Fn(&Point3<f32>) -> f32,
    {
        self.march_with(Self::MAX_STEPS, Self::MIN_DIST, Self::MAX_DIST, sdf)
    }

    pub fn march_with<SDF>(&self, max_steps: usize, min_dist: f32, max_dist: f32, sdf: SDF)
        -> Option<MarchResult>
        where SDF: Fn(&Point3<f32>) -> f32,
    {
        let mut pos = self.origin.clone();
        let mut total_dist = 0.0;
        for i in 0 .. max_steps {
            let dist = sdf(&pos);
            total_dist += dist;

            // the ray has failed to hit anything in the scene
            if total_dist >= max_dist {
                return None
            }

            pos += dist * self.direction;

            // the ray has gotten close enough to something to be considered a hit
            if dist <= min_dist {
                return Some(MarchResult{ steps: i, distance: total_dist, point: pos })
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

    // test an intersectino
    let result = ray.march(|pt| scene.sdf(&scaled, pt)).expect("Failed to march ray");
    assert_eq!(result.distance, 3.0);

    // test a miss
    assert!(ray.march(|pt| scene.sdf(&moved, pt)).is_none());
}
