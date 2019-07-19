
use std::ops::{Add,Mul};
use std::path::Path;
use image::{ImageBuffer,Rgb};


#[derive(Clone,Debug,Default)]
pub struct Color {
    data: [f32; 3],
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
    (val * 255.0).floor() as u8
}

impl Color {

    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Color{ data: [r, g, b] }
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
        Rgb([ f32_to_u8(self.r()), f32_to_u8(self.g()), f32_to_u8(self.b()) ])
    }

}

impl Add for &Color {
    type Output = Color;

    fn add(self, rhs: Self) -> Self::Output {
        let mut data = [0.0; 3];
        for i in 0 .. 3 {
            data[i] = self.data[i] + rhs.data[i];
        }

        Color{ data }
    }
}

impl Mul for &Color {

    type Output = Color;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut data = [0.0; 3];
        for i in 0 .. 3 {
            data[i] = self.data[i] * rhs.data[i];
        }

        Color{ data }
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
        for i in 0 .. 3 {
            data[i] = self.data[i] * rhs;
        }

        Color{ data }
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
        Canvas{ width, height, pixels }
    }

    pub fn index(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y * self.width + x)
        } else {
            None
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&Color> {
        self.index(x, y).map( |ix| unsafe { self.pixels.get_unchecked(ix) })
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Color> {
        self.index(x, y).map(move |ix| unsafe { self.pixels.get_unchecked_mut(ix) })
    }

    pub fn save<Q>(&self, path: Q)
    where Q: AsRef<Path>
    {
        let image = ImageBuffer::from_fn(self.width as u32, self.height as u32, |x,y| {
            let ix = (y as usize) * self.width + (x as usize);
            unsafe { self.pixels.get_unchecked(ix).to_rgb() }
        });

        image.save(path).unwrap()
    }

}
