#[derive(Debug, Default, Clone)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// A buffer of color data, with the bottom-left being `(0,0)`.
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

impl std::ops::Add for &Color {
    type Output = Color;
    fn add(self, rhs: &Color) -> Self::Output {
        let mut out = self.clone();
        out += rhs;
        out
    }
}

impl std::ops::AddAssign<&Color> for Color {
    fn add_assign(&mut self, rhs: &Color) {
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
    }
}

impl std::ops::AddAssign for Color {
    fn add_assign(&mut self, rhs: Color) {
        self.add_assign(&rhs)
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

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn index(&self, x: usize, y: usize) -> usize {
        (self.width as usize) * y + x
    }

    /// Mutate a color in the [`Canvas`].
    pub fn get_mut(&mut self, x: usize, y: usize) -> &mut Color {
        let ix = self.index(x, y);
        &mut self.buffer[ix]
    }

    /// Fetch a color in the [`Canvas`].
    pub fn get(&mut self, x: usize, y: usize) -> &Color {
        let ix = self.index(x, y);
        &self.buffer[ix]
    }

    /// Return an iterator to the rows of the image.
    pub fn rows(&self) -> Rows {
        Rows {
            canvas: self,
            row: (self.height as usize),
        }
    }

    /// Return raw image RGB8 data for the image.
    pub fn data(&self) -> Vec<u8> {
        let size = (self.width * self.height) as usize;
        let mut data = Vec::with_capacity(size);

        for row in self.rows() {
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

        for row in self.rows() {
            for col in row {
                let g = col.to_grayscale();
                let index = (g * bound) as usize;
                buf.push(bytes[index] as char);
            }
            buf.push('\n');
        }

        buf
    }
}

impl<'a> Iterator for Rows<'a> {
    type Item = &'a [Color];

    fn next(&mut self) -> Option<Self::Item> {
        if self.row == 0 {
            return None;
        }

        self.row -= 1;

        let len = self.canvas.width as usize;
        let start = self.row * len;

        Some(&self.canvas.buffer[start..start + len])
    }
}
