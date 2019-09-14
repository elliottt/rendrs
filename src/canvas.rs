use image::{ImageBuffer, Rgb};
use std::ops::{Add, AddAssign, Mul};
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct Color {
    data: [f32; 3],
}

impl PartialEq for Color {
    fn eq(&self, other: &Color) -> bool {
        self.r().eq(&other.r()) && self.g().eq(&other.g()) && self.b().eq(&other.b())
    }
}

fn clamp(val: f32) -> f32 {
    if val < 0.0 {
        0.0
    } else if val > 1.0 {
        1.0
    } else {
        val
    }
}

fn f32_to_u8(val: f32) -> u8 {
    f32::max(0.0, f32::min(255.0, val * 255.0)) as u8
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Color { data: [r, g, b] }
    }

    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }

    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn r(&self) -> f32 {
        self.data[0]
    }

    pub fn set_r(&mut self, r: f32) -> &mut Self {
        self.data[0] = clamp(r);
        self
    }

    pub fn g(&self) -> f32 {
        self.data[1]
    }

    pub fn set_g(&mut self, g: f32) -> &mut Self {
        self.data[1] = clamp(g);
        self
    }

    pub fn b(&self) -> f32 {
        self.data[2]
    }

    pub fn set_b(&mut self, b: f32) -> &mut Self {
        self.data[2] = clamp(b);
        self
    }

    pub fn to_rgb(&self) -> Rgb<u8> {
        Rgb([
            f32_to_u8(self.r()),
            f32_to_u8(self.g()),
            f32_to_u8(self.b()),
        ])
    }
}

impl AddAssign for Color {
    fn add_assign(&mut self, rhs: Color) {
        self.add_assign(&rhs)
    }
}

impl AddAssign<&Color> for Color {
    fn add_assign(&mut self, rhs: &Color) {
        for i in 0..3 {
            self.data[i] += rhs.data[i];
        }
    }
}

impl Add for &Color {
    type Output = Color;

    fn add(self, rhs: Self) -> Self::Output {
        let mut data = [0.0; 3];
        for ((part,val),rval) in data.iter_mut().zip(&self.data).zip(&rhs.data) {
            *part = val + rval;
        }

        Color { data }
    }
}

impl Add for Color {
    type Output = Color;

    fn add(self, rhs: Self) -> Self::Output {
        &self + &rhs
    }
}

impl Mul for &Color {
    type Output = Color;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut data = [0.0; 3];
        for ((part,val),rval) in data.iter_mut().zip(&self.data).zip(&rhs.data) {
            *part = val * rval;
        }

        Color { data }
    }
}

impl Mul for Color {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        &self * &rhs
    }
}

impl Mul<f32> for &Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        let mut data = [0.0; 3];
        for (part,val) in data.iter_mut().zip(&self.data) {
            *part = val * rhs;
        }

        Color { data }
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        &self * rhs
    }
}
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pixels: Vec<Color>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        let mut pixels = Vec::new();
        pixels.resize_with(width * height, Default::default);
        Canvas {
            width,
            height,
            pixels,
        }
    }

    pub fn index(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y * self.width + x)
        } else {
            None
        }
    }

    pub fn blit_row(&mut self, y: usize, row: Vec<Color>) {
        let start = y * self.width;
        let slice = self
            .pixels
            .get_mut(start..start + self.width)
            .expect("Missing row");
        for (pixel, color) in slice.iter_mut().zip(row) {
            *pixel = color;
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&Color> {
        self.index(x, y)
            .map(|ix| unsafe { self.pixels.get_unchecked(ix) })
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Color> {
        self.index(x, y)
            .map(move |ix| unsafe { self.pixels.get_unchecked_mut(ix) })
    }

    pub fn save<Q>(&self, path: Q)
    where
        Q: AsRef<Path>,
    {
        let image = ImageBuffer::from_fn(self.width as u32, self.height as u32, |x, y| {
            // invert the y coordinate, otherwise the image will be saved upsidown
            // let ix = (self.height - (y as usize) - 1) * self.width + (x as usize);
            let ix = (y as usize) * self.width + (x as usize);
            unsafe { self.pixels.get_unchecked(ix).to_rgb() }
        });

        image.save(path).unwrap()
    }
}
