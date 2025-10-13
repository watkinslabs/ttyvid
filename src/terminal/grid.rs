use super::Cell;

pub struct Grid {
    cells: Vec<Cell>,
    width: usize,
    height: usize,
}

impl Grid {
    pub fn new(width: usize, height: usize, fg_color: u8, bg_color: u8) -> Self {
        let cells = vec![Cell::empty(fg_color, bg_color); width * height];
        Self {
            cells,
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

    pub fn write_cell(&mut self, x: usize, y: usize, cell: Cell) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = cell;
        }
    }

    pub fn get_cell(&self, x: usize, y: usize) -> Option<&Cell> {
        if x < self.width && y < self.height {
            Some(&self.cells[y * self.width + x])
        } else {
            None
        }
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn clear(&mut self, fg_color: u8, bg_color: u8) {
        for cell in &mut self.cells {
            *cell = Cell::empty(fg_color, bg_color);
        }
    }

    pub fn scroll_up(&mut self, lines: usize, fg_color: u8, bg_color: u8) {
        if lines == 0 || lines >= self.height {
            self.clear(fg_color, bg_color);
            return;
        }

        // Shift cells up
        for y in 0..(self.height - lines) {
            for x in 0..self.width {
                let src_idx = (y + lines) * self.width + x;
                let dst_idx = y * self.width + x;
                self.cells[dst_idx] = self.cells[src_idx];
            }
        }

        // Clear bottom lines
        for y in (self.height - lines)..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                self.cells[idx] = Cell::empty(fg_color, bg_color);
            }
        }
    }

    pub fn scroll_region_up(&mut self, top: usize, bottom: usize, lines: usize, fg_color: u8, bg_color: u8) {
        let region_height = bottom.saturating_sub(top) + 1;
        if lines == 0 || lines >= region_height {
            // Clear the entire region
            for y in top..=bottom.min(self.height - 1) {
                for x in 0..self.width {
                    let idx = y * self.width + x;
                    self.cells[idx] = Cell::empty(fg_color, bg_color);
                }
            }
            return;
        }

        // Shift cells up within the region
        for y in top..=(bottom - lines).min(self.height - 1) {
            for x in 0..self.width {
                let src_y = y + lines;
                if src_y <= bottom && src_y < self.height {
                    let src_idx = src_y * self.width + x;
                    let dst_idx = y * self.width + x;
                    self.cells[dst_idx] = self.cells[src_idx];
                }
            }
        }

        // Clear bottom lines of the region
        let clear_start = (bottom + 1).saturating_sub(lines);
        for y in clear_start..=bottom.min(self.height - 1) {
            for x in 0..self.width {
                let idx = y * self.width + x;
                self.cells[idx] = Cell::empty(fg_color, bg_color);
            }
        }
    }

    pub fn scroll_region_down(&mut self, top: usize, bottom: usize, lines: usize, fg_color: u8, bg_color: u8) {
        let region_height = bottom.saturating_sub(top) + 1;
        if lines == 0 || lines >= region_height {
            // Clear the entire region
            for y in top..=bottom.min(self.height - 1) {
                for x in 0..self.width {
                    let idx = y * self.width + x;
                    self.cells[idx] = Cell::empty(fg_color, bg_color);
                }
            }
            return;
        }

        // Shift cells down within the region (work backwards to avoid overwriting)
        for y in (top + lines..=bottom.min(self.height - 1)).rev() {
            for x in 0..self.width {
                let src_y = y.saturating_sub(lines);
                if src_y >= top {
                    let src_idx = src_y * self.width + x;
                    let dst_idx = y * self.width + x;
                    self.cells[dst_idx] = self.cells[src_idx];
                }
            }
        }

        // Clear top lines of the region
        for y in top..(top + lines).min(bottom + 1).min(self.height) {
            for x in 0..self.width {
                let idx = y * self.width + x;
                self.cells[idx] = Cell::empty(fg_color, bg_color);
            }
        }
    }
}
