
use nalgebra::Point3;

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

#[derive(Debug)]
pub enum Axis { X, Y, Z }

#[derive(Debug,Clone)]
pub struct BVHNodeId(usize);

#[derive(Debug)]
pub enum BVHNode<T> {
    Internal {
        left: Option<BVHNodeId>,
        right: Option<BVHNodeId>,
        bounds: AABB,
        axis: Axis,
    },
    Leaf {
        bounds: AABB,
        value: T,
    },
}

impl<T> BVHNode<T> {
    pub fn internal(bounds: AABB, axis: Axis) -> Self {
        BVHNode::Internal{ left: None, right: None, bounds, axis }
    }

    pub fn leaf(bounds: AABB, value: T) -> Self {
        BVHNode::Leaf{ bounds, value }
    }

    pub fn bounds<'a>(&'a self) -> &'a AABB {
        match self {
            BVHNode::Internal{ ref bounds, .. } => bounds,
            BVHNode::Leaf{ ref bounds, .. } => bounds,
        }
    }
}

#[derive(Debug)]
pub struct BVH<T> {
    nodes: Vec<BVHNode<T>>,
}

impl<T> BVH<T> where T: Clone {

    pub fn new() -> Self {
        BVH{ nodes: Vec::new() }
    }

    pub fn from_nodes<GetBound>(nodes: &mut [T], get_bound: &GetBound) -> Self
        where GetBound: Fn(&T) -> &AABB
    {
        let mut bvh = Self::new();

        if nodes.is_empty() {
            return bvh;
        }

        bvh.add_nodes(nodes, get_bound);
        bvh
    }

    fn add_node(&mut self, node: BVHNode<T>) -> BVHNodeId {
        self.nodes.push(node);
        BVHNodeId(self.nodes.len() - 1)
    }

    fn get_node_mut<'a>(&'a mut self, nid: BVHNodeId) -> &'a mut BVHNode<T> {
        unsafe { self.nodes.get_unchecked_mut(nid.0) }
    }

    fn add_nodes<GetBound>(&mut self, nodes: &mut [T], get_bound: &GetBound) -> Option<BVHNodeId>
        where GetBound: Fn(&T) -> &AABB
    {
        if nodes.is_empty() {
            return None
        }

        let mut bounds = get_bound(&nodes[0]).clone();

        if nodes.len() == 1 {
            return Some(self.add_node(BVHNode::leaf(bounds, nodes[0].clone())));
        }

        let mut centroid_bound = AABB::from_point(bounds.centroid());

        for node in &nodes[1..] {
            let bound = get_bound(node);
            bounds.union_mut(bound);
            centroid_bound.union_point_mut(&bound.centroid());
        }

        // partition the nodes about the mid-point of the centroid bound
        let (axis,mid) = centroid_bound.max_axis();
        let pivot_index = utils::partition_by(nodes, |node| {
            let bound = get_bound(&node);
            match axis {
                Axis::X => bound.centroid().x < mid,
                Axis::Y => bound.centroid().y < mid,
                Axis::Z => bound.centroid().z < mid,
            }
        }).expect("Invalid centroid bound");

        let nid = self.add_node(BVHNode::internal(bounds, axis));

        let lid = self.add_nodes(&mut nodes[0..pivot_index], get_bound);
        let rid = self.add_nodes(&mut nodes[pivot_index..], get_bound);

        if let BVHNode::Internal{ ref mut left, ref mut right, .. } =
            self.get_node_mut(nid.clone()) {
            *left = lid;
            *right = rid;
        }

        Some(nid)
    }

    pub fn bounding_volume<'a>(&'a self) -> Option<&'a AABB> {
        if self.nodes.is_empty() {
            None
        } else {
            Some(&self.nodes[0].bounds())
        }
    }

    pub fn intersect(&self, ray: &Ray) -> Vec<T> {
        let mut results = Vec::new();
        if !self.nodes.is_empty() {
            self.intersect_rec(ray, BVHNodeId(0), &mut results);
        }
        results
    }

    fn intersect_rec(&self, ray: &Ray, nid: BVHNodeId, results: &mut Vec<T>) {
        match unsafe { self.nodes.get_unchecked(nid.0) } {
            BVHNode::Internal{ left, right, ref bounds, .. } => {
                if ray.intersects(bounds) {
                    left.clone().map(|lid| self.intersect_rec(ray, lid, results));
                    right.clone().map(|rid| self.intersect_rec(ray, rid, results));
                }
            },

            BVHNode::Leaf{ ref bounds, value } => {
                if ray.intersects(bounds) {
                    results.push(value.clone())
                }
            },
        }
    }
}

#[test]
fn test_bvh() {
    {
        let mut nodes = vec![
            ( AABB::new([1.0, 1.0, 1.0].into(), [2.0, 2.0, 2.0].into()), 1 ),
        ];

        let bvh = BVH::from_nodes(&mut nodes, &|(a,_)| a);
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
        let mut nodes = vec![
            ( AABB::new([1.0, 1.0, 1.0].into(), [2.0, 2.0, 2.0].into()), 1 ),
            ( AABB::new([3.0, 3.0, 1.0].into(), [4.0, 4.0, 2.0].into()), 2 ),
            ( AABB::new([0.5, 0.5, 1.0].into(), [2.0, 2.0, 2.0].into()), 3 ),
        ];

        let bvh = BVH::from_nodes(&mut nodes, &|(a,_)| a);
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
