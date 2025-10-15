use super::Palette;

#[derive(Clone)]
pub struct Canvas {
    data: Vec<u8>,
    width: usize,
    height: usize,
}

impl Canvas {
    pub fn new(width: usize, height: usize, _palette: &Palette) -> Self {
        // Initialize with black (color index 0)
        let data = vec![0; width * height];
        Self {
            data,
            width,
            height,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color_index: u8) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = color_index;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Option<u8> {
        if x < self.width && y < self.height {
            Some(self.data[y * self.width + x])
        } else {
            None
        }
    }
}
