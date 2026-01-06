/// Framebuffer for Atari 800 display output
///
/// Atari 800 NTSC resolution: 384x240 pixels (or 320x240 visible area)
/// Each pixel is represented as RGB (3 bytes)

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,  // RGB format: [R, G, B, R, G, B, ...]
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Framebuffer {
        Framebuffer {
            width,
            height,
            pixels: vec![0; width * height * 3],  // 3 bytes per pixel (RGB)
        }
    }

    /// Set a pixel at (x, y) to the given RGB color
    pub fn set_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        if x < self.width && y < self.height {
            let offset = (y * self.width + x) * 3;
            self.pixels[offset] = r;
            self.pixels[offset + 1] = g;
            self.pixels[offset + 2] = b;
        }
    }

    /// Clear the framebuffer to black
    pub fn clear(&mut self) {
        for pixel in self.pixels.iter_mut() {
            *pixel = 0;
        }
    }

    /// Clear the framebuffer to a specific color
    pub fn clear_color(&mut self, r: u8, g: u8, b: u8) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.set_pixel(x, y, r, g, b);
            }
        }
    }

    /// Get a slice of pixels for a specific scanline
    pub fn get_scanline(&self, y: usize) -> &[u8] {
        let start = y * self.width * 3;
        let end = start + self.width * 3;
        &self.pixels[start..end]
    }

    /// Get a mutable slice of pixels for a specific scanline
    pub fn get_scanline_mut(&mut self, y: usize) -> &mut [u8] {
        let start = y * self.width * 3;
        let end = start + self.width * 3;
        &mut self.pixels[start..end]
    }
}
