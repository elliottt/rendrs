use crate::math::{Clamp, Mix};

#[derive(Debug, Default, Clone)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// A buffer of color data, with the bottom-left being `(0,0)`.
#[derive(Debug, Default, Clone)]
pub struct Canvas {
    width: u32,
    height: u32,
    buffer: Vec<Color>,
}

/// An iterator for the rows of the resulting image, starting at the top and working down. This is
/// suitable for using when saving the [`Canvas`].
pub struct Rows<'a> {
    canvas: &'a Canvas,
    row: usize,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub fn hex(hex: usize) -> Self {
        let r = ((hex & 0xff0000) >> 16) as f32 / 255.;
        let g = ((hex & 0x00ff00) >> 8) as f32 / 255.;
        let b = (hex & 0x0000ff) as f32 / 255.;
        Color::new(r, g, b)
    }

    pub fn black() -> Self {
        Self::new(0., 0., 0.)
    }

    pub fn is_black(&self) -> bool {
        self.r == 0. && self.g == 0. && self.b == 0.
    }

    pub fn white() -> Self {
        Self::new(1., 1., 1.)
    }

    pub fn to_u8(&self) -> [u8; 3] {
        let convert = |x: f32| (x * 255.0).min(255.0).max(0.0) as u8;
        [convert(self.r), convert(self.g), convert(self.b)]
    }

    /// Convert the [`Color`] to grayscale.
    pub fn to_grayscale(&self) -> f32 {
        0.3 * self.r + 0.59 * self.g + 0.11 * self.b
    }
}

impl Clamp<f32> for &Color {
    type Output = Color;

    fn clamp(self, lo: f32, hi: f32) -> Self::Output {
        Color::new(
            self.r.clamp(lo, hi),
            self.g.clamp(lo, hi),
            self.b.clamp(lo, hi),
        )
    }
}

impl Mix for &Color {
    type Output = Color;

    fn mix(self, b: Self, t: f32) -> Self::Output {
        if t <= 0. {
            self.clone()
        } else if t >= 1. {
            b.clone()
        } else {
            Color::new(
                f32::mix(self.r, b.r, t),
                f32::mix(self.g, b.g, t),
                f32::mix(self.b, b.b, t),
            )
        }
    }
}

impl std::ops::Mul<&Color> for f32 {
    type Output = Color;
    fn mul(self, rhs: &Color) -> Self::Output {
        Color::new(rhs.r * self, rhs.g * self, rhs.b * self)
    }
}

impl std::ops::Mul<Color> for f32 {
    type Output = Color;
    fn mul(self, rhs: Color) -> Self::Output {
        self * &rhs
    }
}

impl std::ops::Mul<f32> for Color {
    type Output = Color;
    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl std::ops::Mul<f32> for &Color {
    type Output = Color;
    fn mul(self, rhs: f32) -> Self::Output {
        let mut out = self.clone();
        out *= rhs;
        out
    }
}

impl std::ops::MulAssign<f32> for Color {
    fn mul_assign(&mut self, rhs: f32) {
        self.r *= rhs;
        self.g *= rhs;
        self.b *= rhs;
    }
}

impl std::ops::Mul for Color {
    type Output = Color;
    fn mul(mut self, rhs: Color) -> Self::Output {
        self *= &rhs;
        self
    }
}

impl std::ops::Mul<&Color> for Color {
    type Output = Color;
    fn mul(mut self, rhs: &Color) -> Self::Output {
        self *= rhs;
        self
    }
}

impl std::ops::Mul for &Color {
    type Output = Color;
    fn mul(self, rhs: &Color) -> Self::Output {
        let mut out = self.clone();
        out *= rhs;
        out
    }
}

impl std::ops::Mul<Color> for &Color {
    type Output = Color;
    fn mul(self, mut rhs: Color) -> Self::Output {
        rhs *= self;
        rhs
    }
}

impl std::ops::MulAssign<&Color> for Color {
    fn mul_assign(&mut self, rhs: &Color) {
        self.r *= rhs.r;
        self.g *= rhs.g;
        self.b *= rhs.b;
    }
}

impl std::ops::Add for Color {
    type Output = Color;

    #[inline]
    fn add(mut self, rhs: Color) -> Self::Output {
        self += &rhs;
        self
    }
}

impl std::ops::Add for &Color {
    type Output = Color;

    #[inline]
    fn add(self, rhs: &Color) -> Self::Output {
        self.clone() + rhs
    }
}

impl std::ops::Add<&Color> for Color {
    type Output = Color;

    #[inline]
    fn add(mut self, rhs: &Color) -> Self::Output {
        self += rhs;
        self
    }
}

impl std::ops::Add<Color> for &Color {
    type Output = Color;

    #[inline]
    fn add(self, mut rhs: Color) -> Self::Output {
        rhs += self;
        rhs
    }
}

impl std::ops::AddAssign for Color {
    #[inline]
    fn add_assign(&mut self, rhs: Color) {
        self.add_assign(&rhs)
    }
}

impl std::ops::AddAssign<&Color> for Color {
    #[inline]
    fn add_assign(&mut self, rhs: &Color) {
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
    }
}

impl Canvas {
    /// Construct a new [`Canvas`].
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        let mut buffer = Vec::with_capacity(size);
        buffer.resize_with(size, Default::default);
        Self {
            width,
            height,
            buffer,
        }
    }

    pub fn blit(&mut self, off_x: u32, off_y: u32, other: &Canvas) {
        let start = off_x as usize;
        let end = start + other.width as usize;
        for (y, src) in other.rows() {
            let dst = self.row_mut(y + off_y as usize);
            dst[start..end].clone_from_slice(src);
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Fetch a row of the canvas.
    pub fn row(&self, y: usize) -> &[Color] {
        let start = y * self.width as usize;
        &self.buffer[start..start + self.width as usize]
    }

    /// Fetch a mutable row of the canvas.
    pub fn row_mut(&mut self, y: usize) -> &mut [Color] {
        let start = y * self.width as usize;
        &mut self.buffer[start..start + self.width as usize]
    }

    /// Return an iterator to the rows of the image.
    pub fn rows(&self) -> Rows {
        Rows {
            canvas: self,
            row: (self.height as usize),
        }
    }

    pub fn coords(&self) -> impl Iterator<Item = (usize, usize)> {
        let width = self.width as usize;
        (0..self.height as usize).flat_map(move |y| (0..width).map(move |x| (x, y)))
    }

    /// Return an iterator to the mutable pixels of the image.
    pub fn pixels_mut(&mut self) -> &mut [Color] {
        &mut self.buffer
    }

    /// Return raw image RGB8 data for the image.
    pub fn data(&self) -> Vec<u8> {
        let size = (self.width * self.height) as usize;
        let mut data = Vec::with_capacity(size);

        for (_, row) in self.rows() {
            for color in row {
                data.extend_from_slice(&color.to_u8())
            }
        }

        data
    }

    /// Return an ascii version of the [`Canvas`].
    pub fn to_ascii(&self) -> String {
        let mut buf = String::new();
        let palette = r#"$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\|()1{}[]?-_+~<>i!lI;:,"^`'. "#;
        let bytes = palette.as_bytes();
        let bound = (palette.len() - 1) as f32;

        for (_, row) in self.rows() {
            for col in row {
                let g = col.to_grayscale().clamp(0., 1.);
                let index = (g * bound) as usize;
                buf.push(bytes[index] as char);
            }
            buf.push('\n');
        }

        buf
    }
}

impl<'a> Iterator for Rows<'a> {
    type Item = (usize, &'a [Color]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.row == 0 {
            return None;
        }

        self.row -= 1;

        Some((self.row, self.canvas.row(self.row)))
    }
}
