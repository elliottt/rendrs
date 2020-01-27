use nalgebra::{Matrix4, Point3, Vector3};

use crate::{
    bounding_volume::AABB, material::MaterialId, pattern::PatternId, scene::Scene, shapes::ShapeId,
};

/// Reflect a vector through a normal.
pub fn reflect(vec: &Vector3<f32>, normal: &Vector3<f32>) -> Vector3<f32> {
    let dot = vec.dot(normal);
    vec - (normal * (2.0 * dot))
}

#[derive(Debug, Clone)]
pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
    pub inv_direction: Point3<f32>,
    pub sign: f32,
}

#[derive(Debug, Clone)]
pub struct SDFResult {
    pub distance: f32,
    pub object_space_point: Point3<f32>,
    pub object_id: ShapeId,
    pub material: MaterialId,
    pub pattern: PatternId,
}

impl Ray {
    pub const MIN_DIST: f32 = 0.001;
    pub const MAX_DIST: f32 = 100.0;

    pub fn new(origin: Point3<f32>, direction: Vector3<f32>, sign: f32) -> Self {
        let inv_direction = Point3::new(
            if direction.x != 0.0 {
                1.0 / direction.x
            } else {
                std::f32::INFINITY
            },
            if direction.y != 0.0 {
                1.0 / direction.y
            } else {
                std::f32::INFINITY
            },
            if direction.z != 0.0 {
                1.0 / direction.z
            } else {
                std::f32::INFINITY
            },
        );
        Ray {
            origin,
            direction,
            inv_direction,
            sign,
        }
    }

    pub fn position(&self, t: f32) -> Point3<f32> {
        self.origin + (self.direction * t)
    }

    pub fn march(&self, max_steps: usize, scene: &Scene) -> Option<MarchResult> {
        let mut pos = self.clone();
        let mut total_dist: f32 = 0.0;
        for i in 0..max_steps {
            let res = scene.sdf(&pos);
            let signed_radius = self.sign * res.distance;

            if signed_radius < Self::MIN_DIST {
                return Some(MarchResult {
                    steps: i,
                    distance: total_dist,
                    object_id: res.object_id,
                    object_space_point: res.object_space_point,
                    final_ray: pos,
                    material: res.material,
                    pattern: res.pattern,
                });
            }

            total_dist += signed_radius;

            if total_dist > Self::MAX_DIST {
                break;
            }

            pos.origin += signed_radius * pos.direction;
        }

        None
    }

    /// Make a new ray that has the given transformation applied to it.
    pub fn transform(&self, matrix: &Matrix4<f32>) -> Self {
        let origin = matrix.transform_point(&self.origin);
        let direction = matrix.transform_vector(&self.direction);
        Self::new(origin, direction, self.sign)
    }

    /// Returns `true` when the ray intersects the bounding volume.
    ///
    /// https://tavianator.com/fast-branchless-raybounding-box-intersections/
    pub fn intersects(&self, aabb: &AABB) -> bool {
        let t1 = Point3::new(
            (aabb.min.x - self.origin.x) * self.inv_direction.x,
            (aabb.min.y - self.origin.y) * self.inv_direction.y,
            (aabb.min.z - self.origin.z) * self.inv_direction.z,
        );
        let t2 = Point3::new(
            (aabb.max.x - self.origin.x) * self.inv_direction.x,
            (aabb.max.y - self.origin.y) * self.inv_direction.y,
            (aabb.max.z - self.origin.z) * self.inv_direction.z,
        );

        let min = Point3::new(t1.x.min(t2.x), t1.y.min(t2.y), t1.z.min(t2.z));
        let max = Point3::new(t1.x.max(t2.x), t1.y.max(t2.y), t1.z.max(t2.z));

        let tmin = min.x.max(min.y).max(min.z);
        let tmax = max.x.min(max.y).min(max.z);

        tmax >= tmin
    }
}

#[derive(Debug)]
pub struct MarchResult {
    pub steps: usize,
    pub distance: f32,
    pub object_id: ShapeId,
    pub object_space_point: Point3<f32>,
    pub final_ray: Ray,
    pub material: MaterialId,
    pub pattern: PatternId,
}

impl MarchResult {
    /// Compute the normal to this result
    pub fn normal(&self, scene: &Scene) -> Vector3<f32>
    {
        let mut pos = self.final_ray.clone();

        let res = scene.sdf(&pos);
        let offset = Vector3::new(0.0001, 0.0, 0.0);

        pos.origin = self.final_ray.origin - offset.xyy();
        let px = scene.sdf(&pos);

        pos.origin = self.final_ray.origin - offset.yxy();
        let py = scene.sdf(&pos);

        pos.origin = self.final_ray.origin - offset.yyx();
        let pz = scene.sdf(&pos);
        Vector3::new(
            res.distance - px.distance,
            res.distance - py.distance,
            res.distance - pz.distance,
        )
        .normalize()
    }
}

#[test]
fn test_intersect() {
    let rays = vec![
        Ray::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 1.0), 1.0),
        Ray::new(
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(-1.0, 0.0, 1.0),
            1.0,
        ),
    ];

    for p in rays {
        assert!(p.intersects(&AABB::max()));

        assert!(p.intersects(&AABB::new(
            Point3::new(-1.0, -1.0, -1.0),
            Point3::new(1.0, 1.0, 1.0)
        )));

        assert!(!p.intersects(&AABB::new(
            Point3::new(2.0, -2.0, 2.0),
            Point3::new(4.0, 4.0, 4.0)
        )));
    }
}

#[test]
fn test_position() {
    let p = Ray::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0), 1.0);

    assert_eq!(p.position(0.0), p.origin);
    assert_eq!(p.position(1.0), Point3::new(0.0, 1.0, 0.0));
    assert_eq!(p.position(-1.0), Point3::new(0.0, -1., 0.0));
}

#[test]
fn test_transform() {
    let p = Ray::new(Point3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0), 1.0);

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
    use crate::assert_eq_f32;
    use crate::{
        scene::Scene,
        shapes::{PrimShape, Shape},
    };

    let ray = Ray::new(
        Point3::new(0.0, 0.0, -5.0),
        Vector3::new(0.0, 0.0, 1.0),
        1.0,
    );

    let mut scene = Scene::new();
    let sphere = scene.add(Shape::PrimShape {
        shape: PrimShape::Sphere,
    });

    // test an intersection
    let mut result = ray
        .march(4, &scene)
        .expect("Failed to march ray");
    assert_eq_f32!(result.distance, 4.0);

    result = ray
        .march(4, &scene)
        .expect("Failed to march ray");
    assert_eq_f32!(result.distance, 3.0);

    // test a miss
    assert!(ray.march(100, &scene).is_none());
}

#[test]
fn test_reflect() {
    use crate::assert_eq_f32;

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

        assert_eq_f32!(r.x, 1.0);
        assert_eq_f32!(r.y, 0.0);
        assert_eq_f32!(r.z, 0.0);
    }
}
