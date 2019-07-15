
use nalgebra::{Point3,Vector3};

pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

pub struct MarchResult {
    pub steps: usize,
    pub dist: f32,
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
                return Some(MarchResult{ steps: i, dist: total_dist, point: pos })
            }
        }

        None
    }

}

#[test]
fn test_position() {
    let p = Ray::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));

    assert_eq!(p.position(0.0), p.origin);
    assert_eq!(p.position(1.0), Point3::new(0.0, 1.0, 0.0));
    assert_eq!(p.position(-1.0), Point3::new(0.0, -1., 0.0));
}
