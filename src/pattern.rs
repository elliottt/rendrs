
use nalgebra::Point3;

use crate::canvas::Color;

#[derive(Copy,Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub struct PatternId(pub usize);

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

    /// Striped
    Stripe{
        first: PatternId,
        second: PatternId,
    }
}

impl Pattern {
    pub fn solid(color: Color) -> Self {
        Pattern::Solid{ color }
    }

    pub fn stripe(first: PatternId, second: PatternId) -> Self {
        Pattern::Stripe{ first, second }
    }

    pub fn color_at<'a,Pats>(&'a self, store: Pats, point: &Point3<f32>)
        -> &'a Color
        where Pats: Fn(PatternId) -> &'a Pattern
    {
        match self {
            Pattern::Solid{ color } => {
                &color
            },

            Pattern::Stripe{ first, second } => {
                if (point.x.floor() as isize) % 2 == 0 {
                    store(*first).color_at(store, point)
                } else {
                    store(*second).color_at(store, point)
                }
            },
        }
    }
}

#[test]
fn test_stripes() {
    let mut store = Patterns::new();
    let black = store.add_pattern(Pattern::solid(Color::black()));
    let white = store.add_pattern(Pattern::solid(Color::white()));
    let tex = Pattern::stripe(black, white);
    let lookup = |pid| store.get_pattern(pid);
    assert_eq!(tex.color_at(lookup, &Point3::new(0.0, 0.0, 0.0)), &Color::black());
    assert_eq!(tex.color_at(lookup, &Point3::new(1.0, 0.0, 0.0)), &Color::white());
    assert_eq!(tex.color_at(lookup, &Point3::new(2.5, 0.0, 0.0)), &Color::black());
}
