
use nalgebra::{Point3,Matrix4,Vector3};

use crate::{utils,ray::Ray};

#[derive(Debug,Clone,PartialEq)]
pub struct AABB {
    pub min: Point3<f32>,
    pub max: Point3<f32>,
}

impl AABB {
    /// Construct an AABB with the given min and max points.
    pub fn new(min: Point3<f32>, max: Point3<f32>) -> Self {
        AABB{ min, max }
    }

    /// Construct an AABB that contains these two points.
    pub fn from_points(a: &Point3<f32>, b: &Point3<f32>) -> Self {
        let min = Point3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let max = Point3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
        Self::new(min, max)
    }

    /// Construct an AABB that encloses a single point.
    pub fn from_point(point: Point3<f32>) -> Self {
        AABB{ min: point.clone(), max: point.clone() }
    }

    /// Construct an AABB that encloses all points.
    pub fn max() -> Self {
        AABB{
            min: Point3::new(std::f32::MIN, std::f32::MIN, std::f32::MIN),
            max: Point3::new(std::f32::MAX, std::f32::MAX, std::f32::MAX),
        }
    }

    /// Construct the centroid of the bounding volume.
    pub fn centroid(&self) -> Point3<f32> {
        Point3::new(
            self.min.x + (self.max.x - self.min.x) / 2.0,
            self.min.y + (self.max.y - self.min.y) / 2.0,
            self.min.z + (self.max.z - self.min.z) / 2.0,
        )
    }

    /// Construct a new AABB that contains the transforomed original.
    pub fn transform(&self, matrix: &Matrix4<f32>) -> Self {
        let right = matrix.column(0);
        let xa = right * self.min.x;
        let xb = right * self.max.x;
        let xmin = Vector3::new(xa.x.min(xb.x), xa.y.min(xb.y), xa.z.min(xb.z));
        let xmax = Vector3::new(xa.x.max(xb.x), xa.y.max(xb.y), xa.z.max(xb.z));

        let up = matrix.column(1);
        let ya = up * self.min.y;
        let yb = up * self.max.y;
        let ymin = Vector3::new(ya.x.min(yb.x), ya.y.min(yb.y), ya.z.min(yb.z));
        let ymax = Vector3::new(ya.x.max(yb.x), ya.y.max(yb.y), ya.z.max(yb.z));

        let back = matrix.column(2);
        let za = back * self.min.z;
        let zb = back * self.max.z;
        let zmin = Vector3::new(za.x.min(zb.x), za.y.min(zb.y), za.z.min(zb.z));
        let zmax = Vector3::new(za.x.max(zb.x), za.y.max(zb.y), za.z.max(zb.z));

        let translate = {
            let col = matrix.column(3);
            Point3::new(col[0], col[1], col[2])
        };

        Self::new(
            translate + (xmin + ymin + zmin),
            translate + (xmax + ymax + zmax),
        )
    }

    /// Construct the intersection of two AABBs.
    pub fn intersect(&self, other: &Self) -> Self {
        let min = Point3::new(
            self.min.x.max(other.min.x),
            self.min.y.max(other.min.y),
            self.min.z.max(other.min.z),
        );
        let max = Point3::new(
            self.max.x.min(other.max.x),
            self.max.y.min(other.max.y),
            self.max.z.min(other.max.z),
        );
        Self::new(min, max)
    }

    pub fn intersect_mut(&mut self, other: &Self) {
        self.min.x = self.min.x.max(other.min.x);
        self.min.y = self.min.y.max(other.min.y);
        self.min.z = self.min.z.max(other.min.z);
        self.max.x = self.max.x.min(other.max.x);
        self.max.y = self.max.y.min(other.max.y);
        self.max.z = self.max.z.min(other.max.z);
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
        Self::new(min, max)
    }

    pub fn union_mut(&mut self, other: &Self) {
        self.min.x = self.min.x.min(other.min.x);
        self.min.y = self.min.y.min(other.min.y);
        self.min.z = self.min.z.min(other.min.z);
        self.max.x = self.max.x.max(other.max.x);
        self.max.y = self.max.y.max(other.max.y);
        self.max.z = self.max.z.max(other.max.z);
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
        Self::new(min, max)
    }

    pub fn union_point_mut(&mut self, point: &Point3<f32>) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }

    /// Returns `true` when the point lies within the bounding volume.
    pub fn contains(&self, point: Point3<f32>) -> bool {
        self.min <= point && point <= self.max
    }

    pub fn max_axis(&self) -> (Axis,f32) {
        let diff = self.max - self.min;
        if diff.x > diff.y {
            if diff.x > diff.z {
                (Axis::X, self.min.x + diff.x / 2.0)
            } else {
                (Axis::Z, self.min.z + diff.z / 2.0)
            }
        } else {
            if diff.y > diff.z {
                (Axis::Y, self.min.y + diff.y / 2.0)
            } else {
                (Axis::Z, self.min.z + diff.z / 2.0)
            }
        }
    }
}

#[derive(Clone,Debug)]
pub enum Axis { X, Y, Z }

#[derive(Clone,Debug)]
pub enum BVHNodeType {
    Internal {
        right_offset: usize,
        axis: Axis,
    },
    Leaf {
        values_start: usize,
        num_nodes: usize,
    },
}

#[derive(Clone,Debug)]
pub struct BVHNode {
    node_type: BVHNodeType,
    bounds: AABB,
}

impl BVHNode {
    pub fn internal(bounds: AABB, axis: Axis) -> Self {
        BVHNode{
            node_type: BVHNodeType::Internal{ right_offset: 0, axis },
            bounds,
        }
    }

    pub fn leaf(bounds: AABB, values_start: usize, num_nodes: usize) -> Self {
        BVHNode{
            node_type: BVHNodeType::Leaf{
                values_start,
                num_nodes,
            },
            bounds,
        }
    }
}

#[derive(Clone,Debug)]
pub struct BVH<T> {
    nodes: Vec<BVHNode>,
    values: Vec<T>,
}

impl<T> BVH<T> where T: Clone {

    pub fn from_nodes<GetBound>(mut values: Vec<T>, get_bound: &GetBound) -> Self
        where GetBound: Fn(&T) -> AABB
    {
        if values.is_empty() {
            BVH { nodes: Vec::new(), values }
        } else {
            let mut nodes = Vec::new();
            Self::add_nodes(&mut nodes, &mut values, 0, get_bound);
            BVH { nodes, values }
        }
    }

    fn add_node(nodes: &mut Vec<BVHNode>, node: BVHNode) -> usize {
        nodes.push(node);
        nodes.len() - 1
    }

    fn get_node_mut<'a>(nodes: &'a mut Vec<BVHNode>, nid: usize) -> &'a mut BVHNode {
        unsafe { nodes.get_unchecked_mut(nid) }
    }

    fn add_nodes<GetBound>(
        nodes: &mut Vec<BVHNode>,
        values: &mut [T],
        start: usize,
        get_bound: &GetBound,
    ) -> usize
        where GetBound: Fn(&T) -> AABB
    {
        let mut bounds = get_bound(&values[0]).clone();
        let mut centroid_bound = AABB::from_point(bounds.centroid());

        for value in &values[1..] {
            let bound = get_bound(value);
            bounds.union_mut(&bound);
            centroid_bound.union_point_mut(&bound.centroid());
        }

        // if the centroid bound collapses to a single point, make a leaf
        if centroid_bound.min == centroid_bound.max {
            return Self::add_node(nodes, BVHNode::leaf(bounds, start, values.len()));
        }

        // partition the values about the mid-point of the centroid bound
        let (axis,mid) = centroid_bound.max_axis();
        let pivot_index = utils::partition_by(values, |value| {
            let bound = get_bound(&value);
            match axis {
                Axis::X => bound.centroid().x < mid,
                Axis::Y => bound.centroid().y < mid,
                Axis::Z => bound.centroid().z < mid,
            }
        }).expect("Invalid centroid bound");

        if pivot_index == 0 {
            panic!("bad mid point: {:?}", centroid_bound)
        }

        let nid = Self::add_node(nodes, BVHNode::internal(bounds, axis));

        let _loff = Self::add_nodes(nodes, &mut values[0..pivot_index], start, get_bound);
        let roff = Self::add_nodes(nodes, &mut values[pivot_index..], start + pivot_index, get_bound);

        let BVHNode{ node_type, .. } = Self::get_node_mut(nodes, nid);
        if let BVHNodeType::Internal{ ref mut right_offset, .. } = node_type {
            *right_offset = roff;
        }

        nid
    }

    pub fn bounding_volume<'a>(&'a self) -> Option<&'a AABB> {
        if self.nodes.is_empty() {
            None
        } else {
            Some(&self.nodes[0].bounds)
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Vec<T> {
        let mut results = Vec::new();
        self.intersect_with(ray, |hit| results.push(hit.clone()));
        results
    }

    pub fn intersect_with<Hit>(&self, ray: &Ray, mut handle_hit: Hit)
        where Hit: FnMut(&T) -> ()
    {
        if !self.nodes.is_empty() {
            self.intersect_rec(ray, 0, &mut handle_hit)
        }
    }

    fn intersect_rec<Hit>(&self, ray: &Ray, offset: usize, handle_hit: &mut Hit)
        where Hit: FnMut(&T) -> ()
    {
        let node = unsafe { self.nodes.get_unchecked(offset) };
        if ray.intersects(&node.bounds) {
            match node.node_type {
                BVHNodeType::Internal{ right_offset, .. } => {
                    self.intersect_rec(ray, offset+1, handle_hit);
                    self.intersect_rec(ray, right_offset, handle_hit);
                },

                BVHNodeType::Leaf{ values_start, num_nodes } => {
                    for value in self.values[values_start..].iter().take(num_nodes) {
                        handle_hit(&value)
                    }
                },
            }
        }
    }
}

#[test]
fn test_bvh() {
    {
        let nodes = vec![
            ( AABB::new([1.0, 1.0, 1.0].into(), [2.0, 2.0, 2.0].into()), 1 ),
        ];

        let bvh = BVH::from_nodes(nodes, &|(a,_)| a.clone());
        assert_eq!(
            bvh.bounding_volume(),
            Some(&AABB::new([1.0, 1.0, 1.0].into(), [2.0, 2.0, 2.0].into()))
        );

        let int = bvh.intersect(&Ray::new([0.0, 0.0, -1.0].into(), [0.0, 0.0, 1.0].into(), 1.0));
        assert_eq!(int.len(), 0);

        let int = bvh.intersect(&Ray::new([1.5, 1.5, -1.0].into(), [0.0, 0.0, 1.0].into(), 1.0));
        assert_eq!(int.len(), 1);
    }

    {
        let nodes = vec![
            ( AABB::new([1.0, 1.0, 1.0].into(), [2.0, 2.0, 2.0].into()), 1 ),
            ( AABB::new([3.0, 3.0, 1.0].into(), [4.0, 4.0, 2.0].into()), 2 ),
            ( AABB::new([0.5, 0.5, 1.0].into(), [2.0, 2.0, 2.0].into()), 3 ),
        ];

        let bvh = BVH::from_nodes(nodes, &|(a,_)| a.clone());
        assert_eq!(
            bvh.bounding_volume(),
            Some(&AABB::new([0.5, 0.5, 1.0].into(), [4.0, 4.0, 2.0].into()))
        );

        let int = bvh.intersect(&Ray::new([0.0, 0.0, -1.0].into(), [0.0, 0.0, 1.0].into(), 1.0));
        assert_eq!(int.len(), 0);

        let int = bvh.intersect(&Ray::new([1.5, 1.5, -1.0].into(), [0.0, 0.0, 1.0].into(), 1.0));
        assert_eq!(int.len(), 2);

        let mut ids: Vec<isize> = int.iter().map(|(_,b)| *b).collect();
        ids.sort();
        assert_eq!(ids, vec![1,3]);
    }
}
