use nalgebra::{Point2, Vector2};

pub trait Sampler: std::marker::Send + std::marker::Sync + Clone {
    type PixelIterator: Iterator<Item = Point2<f32>>;

    /// Produce an iterator that will traverse the samples for a single pixel.
    fn pixel(&mut self, pixel: &Point2<f32>) -> Self::PixelIterator;

    /// A size-hint for the number of samples computed for each pixel.
    fn samples_per_pixel(&self) -> usize;
}

#[derive(Debug, Clone)]
pub struct UniformSampler {
    step: Point2<f32>,
    size: usize,
}

impl UniformSampler {
    /// Construct a new uniform sampler that will sample the center of each cell of the
    /// width x height sub-pixel grid.
    pub fn new(width: u32, height: u32) -> Self {
        let x = (1. / (width as f32)).min(1.);
        let y = (1. / (height as f32)).min(1.);
        Self {
            step: Point2::new(x, y),
            size: (width * height) as usize,
        }
    }
}

pub struct UniformIterator {
    done: bool,
    base: Point2<f32>,
    step: Point2<f32>,
    pos: Vector2<f32>,
}

impl Iterator for UniformIterator {
    type Item = Point2<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let p = self.base + self.pos;

        self.pos.x += self.step.x;

        if self.pos.x >= 1. {
            self.pos.x = self.step.x / 2.;
            self.pos.y += self.step.y;
        }

        if self.pos.y >= 1. {
            self.done = true;
        }

        Some(p)
    }
}

impl Sampler for UniformSampler {
    type PixelIterator = UniformIterator;

    fn pixel(&mut self, pixel: &Point2<f32>) -> Self::PixelIterator {
        UniformIterator {
            done: false,
            base: pixel.clone(),
            step: self.step.clone(),
            pos: Vector2::new(self.step.x / 2., self.step.y / 2.),
        }
    }

    fn samples_per_pixel(&self) -> usize {
        self.size
    }
}

#[test]
fn test_uniform_sampler() {
    let mut sampler = UniformSampler::new(1, 1);
    let samples: Vec<_> = sampler.pixel(&Point2::new(0., 0.)).take(10).collect();
    println!("{:?}", samples);
    assert_eq!(1, samples.len());
    assert_eq!(1, sampler.samples_per_pixel());
    assert_eq!(Point2::new(0.5, 0.5), samples[0]);

    let mut sampler = UniformSampler::new(2, 2);
    let samples: Vec<_> = sampler.pixel(&Point2::new(0., 0.)).take(10).collect();
    println!("{:?}", samples);
    assert_eq!(4, samples.len());
    assert_eq!(4, sampler.samples_per_pixel());
    assert_eq!(Point2::new(0.25, 0.25), samples[0]);
    assert_eq!(Point2::new(0.75, 0.75), samples[3]);
}
