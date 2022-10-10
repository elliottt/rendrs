use nalgebra::{Point2, Vector2};

pub trait Sampler: std::marker::Send + std::marker::Sync {
    /// Produce an iterator that will traverse the samples for a single pixel.
    fn pixel_samples(&mut self, samples: &mut Vec<Point2<f32>>, pixel: &Point2<f32>);

    /// A size-hint for the number of samples computed for each pixel.
    fn samples_per_pixel(&self) -> usize;

    fn clone_sampler(&self) -> Box<dyn Sampler>;
}

impl<S: Sampler + ?Sized> Sampler for Box<S> {
    fn pixel_samples(&mut self, samples: &mut Vec<Point2<f32>>, pixel: &Point2<f32>) {
        self.as_mut().pixel_samples(samples, pixel)
    }

    fn samples_per_pixel(&self) -> usize {
        self.as_ref().samples_per_pixel()
    }

    fn clone_sampler(&self) -> Box<dyn Sampler> {
        self.as_ref().clone_sampler()
    }
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

impl Sampler for UniformSampler {
    fn pixel_samples(&mut self, samples: &mut Vec<Point2<f32>>, pixel: &Point2<f32>) {
        let mut pos = Vector2::new(self.step.x / 2., self.step.y / 2.);

        while pos.y < 1. {
            samples.push(pixel + pos);

            pos.x += self.step.x;

            if pos.x >= 1. {
                pos.x = self.step.x / 2.;
                pos.y += self.step.y;
            }
        }
    }

    fn samples_per_pixel(&self) -> usize {
        self.size
    }

    fn clone_sampler(&self) -> Box<dyn Sampler> {
        Box::new(self.clone())
    }
}

#[test]
fn test_uniform_sampler() {
    let mut sampler = UniformSampler::new(1, 1);
    let mut samples = Vec::new();
    sampler.pixel_samples(&mut samples, &Point2::new(0., 0.));
    println!("{:?}", samples);
    assert_eq!(1, samples.len());
    assert_eq!(1, sampler.samples_per_pixel());
    assert_eq!(Point2::new(0.5, 0.5), samples[0]);

    let mut sampler = UniformSampler::new(2, 2);
    samples.clear();
    sampler.pixel_samples(&mut samples, &Point2::new(0., 0.));
    println!("{:?}", samples);
    assert_eq!(4, samples.len());
    assert_eq!(4, sampler.samples_per_pixel());
    assert_eq!(Point2::new(0.25, 0.25), samples[0]);
    assert_eq!(Point2::new(0.75, 0.75), samples[3]);
}
