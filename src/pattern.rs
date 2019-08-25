
use nalgebra::{Vector2,Point3,Matrix4};

use crate::canvas::Color;

#[derive(Copy,Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub struct PatternId(usize);

#[derive(Debug)]
pub struct Patterns {
    patterns: Vec<Pattern>,
}

impl Patterns {
    pub fn new() -> Self {
        Patterns { patterns: Vec::with_capacity(10), }
    }

    pub fn add_pattern(&mut self, pattern: Pattern) -> PatternId {
        self.patterns.push(pattern);
        PatternId(self.patterns.len() - 1)
    }

    pub fn get_pattern(&self, pid: PatternId) -> &Pattern {
        unsafe { self.patterns.get_unchecked(pid.0) }
    }
}

#[derive(Clone,Debug)]
pub enum Pattern {
    /// Just a solid color
    Solid{
        color: Color,
    },

    /// Fade from the first pattern to the second from 0 - 1.
    Gradient{
        first: PatternId,
        second: PatternId,
    },

    /// Striped
    Stripe{
        first: PatternId,
        second: PatternId,
    },

    /// Circles of width one
    Circles{
        first: PatternId,
        second: PatternId,
    },

    /// 3D Checkerboard
    Checkers{
        first: PatternId,
        second: PatternId,
    },

    /// Transformation
    Transform{
        // the inverse of the supplied matrix
        transform: Matrix4<f32>,
        pattern: PatternId,
    },
}

impl Pattern {
    pub fn solid(color: Color) -> Self {
        Pattern::Solid{ color }
    }

    pub fn gradient(first: PatternId, second: PatternId) -> Self {
        Pattern::Gradient{ first, second }
    }

    pub fn stripe(first: PatternId, second: PatternId) -> Self {
        Pattern::Stripe{ first, second }
    }

    pub fn circles(first: PatternId, second: PatternId) -> Self {
        Pattern::Circles{ first, second }
    }

    pub fn checkers(first: PatternId, second: PatternId) -> Self {
        Pattern::Checkers{ first, second }
    }

    pub fn transform(matrix: &Matrix4<f32>, pattern: PatternId) -> Self {
        let inv = matrix.try_inverse().expect("Unable to invert transformation matrix");
        Pattern::Transform{ transform: inv, pattern }
    }

    pub fn color_at<'a,Pats>(&'a self, store: &Pats, point: &Point3<f32>)
        -> Color
        where Pats: Fn(PatternId) -> &'a Pattern
    {
        match self {
            Pattern::Solid{ color } => {
                color.clone()
            },

            Pattern::Gradient{ first, second } => {
                if point.x < 0.0 {
                    store(*first).color_at(store, point)
                } else if point.x > 1.0 {
                    store(*second).color_at(store, point)
                } else {
                    let a = store(*first).color_at(store, point);
                    let b = store(*second).color_at(store, point);
                    (a * (1.0 - point.x)) + (b * point.x)
                }
            },

            Pattern::Stripe{ first, second } => {
                if point.x.floor() % 2.0 == 0.0 {
                    store(*first).color_at(store, point)
                } else {
                    store(*second).color_at(store, point)
                }
            },

            Pattern::Circles{ first, second } => {
                let dist = Vector2::new(point.x, point.z).magnitude();
                if dist.floor() % 2.0 == 0.0 {
                    store(*first).color_at(store, point)
                } else {
                    store(*second).color_at(store, point)
                }
            },

            Pattern::Checkers{ first, second } => {
                let val = (point.x.floor() + point.y.floor() + point.z.floor()) as isize;
                if val % 2 == 0 {
                    store(*first).color_at(store, point)
                } else {
                    store(*second).color_at(store, point)
                }
            },

            Pattern::Transform{ transform, pattern } => {
                let new_point = transform.transform_point(point);
                store(*pattern).color_at(store, &new_point)
            }
        }
    }
}

impl Default for Pattern {
    fn default() -> Self {
        Pattern::Solid { color: Color::white() }
    }
}

#[test]
fn test_stripes() {
    let mut store = Patterns::new();
    let black = store.add_pattern(Pattern::solid(Color::black()));
    let white = store.add_pattern(Pattern::solid(Color::white()));
    let tex = Pattern::stripe(black, white);
    let lookup = |pid| store.get_pattern(pid);
    assert_eq!(tex.color_at(&lookup, &Point3::new(0.0, 0.0, 0.0)), Color::black());
    assert_eq!(tex.color_at(&lookup, &Point3::new(1.0, 0.0, 0.0)), Color::white());
    assert_eq!(tex.color_at(&lookup, &Point3::new(2.5, 0.0, 0.0)), Color::black());
}
