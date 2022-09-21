use nalgebra::{Matrix4, Point3, Vector3};

use crate::{ray::Ray, transform::ApplyTransform};

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
        self.min + self.extent()
    }

    pub fn extent(&self) -> Vector3<f32> {
        (self.max - self.min) / 2.
    }

    /// True when the bound contains no volume.
    pub fn is_empty(&self) -> bool {
        self.min.x >= self.max.x && self.min.y >= self.max.y && self.min.z >= self.max.z
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

    pub fn union_point(&self, other: &Point3<f32>) -> Self {
        BoundingBox {
            min: min_point(&self.min, other),
            max: max_point(&self.max, other),
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        BoundingBox {
            min: max_point(&self.min, &other.min),
            max: min_point(&self.max, &other.max),
        }
    }

    #[cfg(test)]
    pub fn contains(&self, p: &Point3<f32>) -> bool {
        self.min < p.clone() && self.max > p.clone()
    }

    /// True when the ray would intersect this bounding box.
    pub fn intersects(&self, ray: &Ray) -> bool {
        let t1 = Point3::new(
            (self.min.x - ray.position.x) * ray.inv_direction.x,
            (self.min.y - ray.position.y) * ray.inv_direction.y,
            (self.min.z - ray.position.z) * ray.inv_direction.z,
        );
        let t2 = Point3::new(
            (self.max.x - ray.position.x) * ray.inv_direction.x,
            (self.max.y - ray.position.y) * ray.inv_direction.y,
            (self.max.z - ray.position.z) * ray.inv_direction.z,
        );

        let min = Point3::new(t1.x.min(t2.x), t1.y.min(t2.y), t1.z.min(t2.z));
        let max = Point3::new(t1.x.max(t2.x), t1.y.max(t2.y), t1.z.max(t2.z));

        let tmin = min.x.max(min.y).max(min.z);
        let tmax = max.x.min(max.y).min(max.z);

        tmax >= tmin
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
fn test_union_point() {
    let p = Point3::new(1., 1., 1.);
    let a = BoundingBox::min().union_point(&p);
    assert_eq!(p, a.min);
    assert_eq!(p, a.max);
}

#[test]
fn test_intersect() {
    let a = BoundingBox::new(Point3::new(1., 1., 1.), Point3::new(-1., -1., -1.));
    assert_eq!(a, a.intersect(&BoundingBox::max()));
    assert_eq!(BoundingBox::min(), a.intersect(&BoundingBox::min()));
}

impl ApplyTransform for BoundingBox {
    fn transform(&self, m: &Matrix4<f32>) -> Self {
        // compute the centroid and extent
        let extent = self.extent();
        let centroid = self.min + extent;

        // transform the centroid to find the new origin
        let centroid = centroid.transform(m);

        // transform the extent by the abs matrix to find the new extent
        let extent = extent.transform(&m.abs());

        Self {
            min: centroid - extent,
            max: centroid + extent,
        }
    }
}

#[test]
fn test_bounding_box_transform() {
    use crate::transform::Transform;

    let bound = BoundingBox::new(Point3::new(-1., -1., 0.), Point3::new(1., 1., 0.));
    let other =
        bound.apply(&Transform::new().rotate(&Vector3::new(std::f32::consts::FRAC_PI_2, 0., 0.)));
    let total = bound.union(&other);

    assert!(total.contains(&Point3::new(0.5, 0.5, 0.5)));
}

#[derive(Debug, Clone)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone)]
struct Node {
    /// The offset to the right subtree, or the start of the values.
    offset: u16,

    /// The number of values present.
    len: u16,

    /// The bounds of this node.
    bounds: BoundingBox,
}

impl Node {
    fn internal(bounds: BoundingBox) -> Self {
        Self {
            offset: 0,
            len: 0,
            bounds,
        }
    }

    fn leaf(bounds: BoundingBox, offset: usize, len: usize) -> Self {
        Self {
            offset: offset as u16,
            len: len as u16,
            bounds,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct BVH<T> {
    nodes: Vec<Node>,
    values: Vec<T>,
}

impl<T: Clone> BVH<T> {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn from_nodes(mut values: Vec<(BoundingBox, T)>) -> Self {
        let mut bvh = Self::new();
        bvh.build(&mut values, 0);
        bvh
    }

    fn build(&mut self, values: &mut [(BoundingBox, T)], start: usize) {
        assert!(!values.is_empty());

        let (bounds, centroid) = values.iter().fold(
            (BoundingBox::min(), BoundingBox::min()),
            |(bounds, centroid), (bound, _)| {
                (bounds.union(bound), centroid.union_point(&bound.centroid()))
            },
        );

        // If the centroids of all the values are the same, there's not point in trying to reduce
        // any further. Conveniently, this is true when the values slice is a singleton.
        if centroid.is_empty() {
            self.nodes.push(Node::leaf(bounds, start, values.len()));
            self.values.extend(values.iter().map(|(_, v)| v.clone()));
            return;
        }

        // Partition the values about the mid-point of the largest centroid bound axis.
        let (mid_point, axis) = largest_axis(&centroid);
        let compare: Box<dyn Fn(&BoundingBox) -> bool> = match axis {
            Axis::X => Box::new(|b| b.centroid().x >= mid_point),
            Axis::Y => Box::new(|b| b.centroid().y >= mid_point),
            Axis::Z => Box::new(|b| b.centroid().z >= mid_point),
        };

        // there's no obvious way to partition values in a slice, so instead we sort according to
        // the negation of compare, to ensure that values that are less than the midpoint are in
        // the front of the slice.
        values.sort_unstable_by_key(|(bound, _)| !compare(bound));
        let middle = values.partition_point(|(b, _)| compare(b));
        let (left, right) = values.split_at_mut(middle);
        assert!(!left.is_empty() && !right.is_empty());

        let cur = self.nodes.len();
        self.nodes.push(Node::internal(bounds));

        self.build(left, start);

        // update the offset after writing the left subtree
        self.nodes[cur].offset = self.nodes.len() as u16;

        self.build(right, start + middle);
    }

    pub fn fold_intersections<R, F: FnMut(R, T) -> R>(&self, ray: &Ray, acc: R, mut fun: F) -> R {
        self.intersections_rec(ray, 0, acc, &mut fun)
    }

    fn intersections_rec<R, F>(&self, ray: &Ray, ix: usize, acc: R, fun: &mut F) -> R
    where
        F: FnMut(R, T) -> R,
    {
        let node = &self.nodes[ix];
        if node.bounds.intersects(ray) {
            if node.len > 0 {
                let start = node.offset as usize;
                let end = start + node.len as usize;
                self.values[start..end].iter().cloned().fold(acc, fun)
            } else {
                let acc = self.intersections_rec(ray, ix + 1, acc, fun);
                self.intersections_rec(ray, ix + node.offset as usize, acc, fun)
            }
        } else {
            acc
        }
    }

    pub fn bounding_box(&self) -> BoundingBox {
        assert!(!self.nodes.is_empty());
        self.nodes[0].bounds.clone()
    }
}

fn largest_axis(bound: &BoundingBox) -> (f32, Axis) {
    let diff = bound.max - bound.min;
    if diff.x > diff.y {
        if diff.x > diff.z {
            (bound.min.x + diff.x / 2., Axis::X)
        } else {
            (bound.min.z + diff.z / 2., Axis::Z)
        }
    } else if diff.x > diff.z {
        (bound.min.x + diff.x / 2., Axis::X)
    } else {
        (bound.min.y + diff.y / 2., Axis::Y)
    }
}