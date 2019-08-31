
use nalgebra::Point3;

pub struct AABB {
    pub min: Point3<f32>,
    pub max: Point3<f32>,
}

impl AABB {
    pub fn new(min: Point3<f32>, max: Point3<f32>) -> Self {
        AABB{ min, max }
    }

    /// Construct an AABB that encloses a single point.
    pub fn from_point(point: Point3<f32>) -> Self {
        AABB{ min: point.clone(), max: point }
    }

    /// Construct an AABB that encloses all points.
    pub fn max() -> Self {
        AABB{
            min: Point3::new(std::f32::MIN, std::f32::MIN, std::f32::MIN),
            max: Point3::new(std::f32::MAX, std::f32::MAX, std::f32::MAX),
        }
    }

    /// Construct a new AABB that encompasses the space of the two.
    pub fn union(&self, other: &Self) -> Self {
        let min = Point3::new(
            self.min.x.min(other.min.x),
            self.min.y.min(other.min.y),
            self.min.z.min(other.min.z),
        );
        let max = Point3::new(
            self.max.x.max(other.max.x),
            self.max.y.max(other.max.y),
            self.max.z.max(other.max.z),
        );
        AABB{ min, max }
    }

    pub fn union_point(&self, point: &Point3<f32>) -> Self {
        let min = Point3::new(
            self.min.x.min(point.x),
            self.min.y.min(point.y),
            self.min.z.min(point.z),
        );
        let max = Point3::new(
            self.max.x.max(point.x),
            self.max.y.max(point.y),
            self.max.z.max(point.z),
        );
        AABB{ min, max }
    }

    /// Returns `true` when the point lies within the bounding volume.
    pub fn contains(&self, point: Point3<f32>) -> bool {
        self.min <= point && point <= self.max
    }
}
