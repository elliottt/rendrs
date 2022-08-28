use std::fmt::Write;

#[derive(Debug, Default)]
pub struct Pixel {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// A buffer of pixel data, with the bottom-left being `(0,0)`.
pub struct Canvas {
    width: u32,
    height: u32,
    buffer: Vec<Pixel>,
}

/// An iterator for the rows of the resulting image, starting at the top and working down. This is
/// suitable for using when saving the [`Canvas`].
pub struct Rows<'a> {
    canvas: &'a Canvas,
    row: usize,
}

impl Pixel {

    pub fn to_u8(&self) -> [u8; 3] {
        let convert = |x: f32| { (x * 255.0).min(255.0).max(0.0) as u8 };
        [ convert(self.r), convert(self.g), convert(self.b) ]
    }

    /// Convert the [`Pixel`] to grayscale.
    pub fn to_grayscale(&self) -> f32 {
        0.3 * self.r + 0.59 * self.g + 0.11 * self.b
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

    /// Mutate a pixel in the [`Canvas`].
    pub fn get_mut(&mut self, x: usize, y: usize) -> &mut Pixel {
        let ix = self.index(x, y);
        &mut self.buffer[ix]
    }

    /// Fetch a pixel in the [`Canvas`].
    pub fn get(&mut self, x: usize, y: usize) -> &Pixel {
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
            for pixel in row {
                data.extend_from_slice(&pixel.to_u8())
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
    type Item = &'a [Pixel];

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
