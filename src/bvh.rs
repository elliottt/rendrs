use nalgebra::{Matrix4, Point3, Vector3};

use crate::transform::ApplyTransform;

#[derive(Debug, Clone, PartialEq)]
pub struct BoundingBox {
    min: Point3<f32>,
    max: Point3<f32>,
}

fn min_point(a: &Point3<f32>, b: &Point3<f32>) -> Point3<f32> {
    Point3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z))
}

fn max_point(a: &Point3<f32>, b: &Point3<f32>) -> Point3<f32> {
    Point3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z))
}

impl BoundingBox {
    pub fn new(a: Point3<f32>, b: Point3<f32>) -> Self {
        let min = min_point(&a, &b);
        let max = max_point(&a, &b);
        BoundingBox { min, max }
    }

    pub fn centroid(&self) -> Point3<f32> {
        Point3::new(
            self.min.x + (self.max.x - self.min.x) / 2.,
            self.min.y + (self.max.y - self.min.y) / 2.,
            self.min.z + (self.max.z - self.min.z) / 2.,
        )
    }

    pub fn min() -> Self {
        BoundingBox {
            min: Point3::new(std::f32::MAX, std::f32::MAX, std::f32::MAX),
            max: Point3::new(std::f32::MIN, std::f32::MIN, std::f32::MIN),
        }
    }

    pub fn max() -> Self {
        BoundingBox {
            min: Point3::new(std::f32::MIN, std::f32::MIN, std::f32::MIN),
            max: Point3::new(std::f32::MAX, std::f32::MAX, std::f32::MAX),
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        BoundingBox {
            min: min_point(&self.min, &other.min),
            max: max_point(&self.max, &other.max),
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        BoundingBox {
            min: max_point(&self.min, &other.min),
            max: min_point(&self.max, &other.max),
        }
    }

    pub fn contains(&self, p: &Point3<f32>) -> bool {
        self.min < p.clone() && self.max > p.clone()
    }

    pub fn add_point(&mut self, p: &Point3<f32>) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.min.z = self.min.z.min(p.z);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
        self.max.z = self.max.z.max(p.z);
    }
}

impl ApplyTransform for BoundingBox {
    fn transform(&self, m: &Matrix4<f32>) -> Self {
        let right = m.column(0);
        let xa = right * self.min.x;
        let xb = right * self.max.x;
        let xmin = Vector3::new(xa.x.min(xb.x), xa.y.min(xb.y), xa.z.min(xb.z));
        let xmax = Vector3::new(xa.x.max(xb.x), xa.y.max(xb.y), xa.z.max(xb.z));

        let up = m.column(1);
        let ya = up * self.min.y;
        let yb = up * self.max.y;
        let ymin = Vector3::new(ya.x.min(yb.x), ya.y.min(yb.y), ya.z.min(yb.z));
        let ymax = Vector3::new(ya.x.max(yb.x), ya.y.max(yb.y), ya.z.max(yb.z));

        let back = m.column(2);
        let za = back * self.min.z;
        let zb = back * self.max.z;
        let zmin = Vector3::new(za.x.min(zb.x), za.y.min(zb.y), za.z.min(zb.z));
        let zmax = Vector3::new(za.x.max(zb.x), za.y.max(zb.y), za.z.max(zb.z));

        let translate = {
            let col = m.column(3);
            Point3::new(col[0], col[1], col[2])
        };

        Self::new(
            translate + (xmin + ymin + zmin),
            translate + (xmax + ymax + zmax),
        )
    }
}

#[test]
fn test_contains() {
    let a = BoundingBox::new(Point3::new(1., 1., 1.), Point3::new(-1., -1., -1.));
    assert!(a.contains(&a.centroid()));
}

#[test]
fn test_union() {
    let a = BoundingBox::new(Point3::new(1., 1., 1.), Point3::new(-1., -1., -1.));
    assert_eq!(a, a.union(&BoundingBox::min()));
    assert_eq!(BoundingBox::max(), a.union(&BoundingBox::max()));
}

#[test]
fn test_intersect() {
    let a = BoundingBox::new(Point3::new(1., 1., 1.), Point3::new(-1., -1., -1.));
    assert_eq!(a, a.intersect(&BoundingBox::max()));
    assert_eq!(BoundingBox::min(), a.intersect(&BoundingBox::min()));
}
