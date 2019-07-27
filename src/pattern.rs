
use nalgebra::Point3;

use crate::canvas::Color;

#[derive(Clone,Debug)]
pub enum Pattern {
    /// Just a solid color
    Solid{
        color: Color,
    },

    /// Striped
    Stripe{
        first: Color,
        second: Color,
    }
}

impl Pattern {
    pub fn solid(color: Color) -> Self {
        Pattern::Solid{ color }
    }

    pub fn stripe(first: Color, second: Color) -> Self {
        Pattern::Stripe{ first, second }
    }

    pub fn color_at(&self, point: &Point3<f32>) -> &Color {
        match self {
            Pattern::Solid{ color } => {
                &color
            },

            Pattern::Stripe{ first, second } => {
                if (point.x.floor() as isize) % 2 == 0 {
                    &first
                } else {
                    &second
                }
            },
        }
    }
}

#[test]
fn test_stripes() {
    let tex = Pattern::stripe(Color::black(), Color::white());
    assert_eq!(tex.color_at(&Point3::new(0.0, 0.0, 0.0)), &Color::black());
    assert_eq!(tex.color_at(&Point3::new(1.0, 0.0, 0.0)), &Color::white());
    assert_eq!(tex.color_at(&Point3::new(2.5, 0.0, 0.0)), &Color::black());
}
