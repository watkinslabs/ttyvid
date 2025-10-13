mod font;
mod canvas;
mod colors;

pub use font::Font;
pub use canvas::Canvas;
pub use colors::Palette;

use crate::terminal::{Cell, CellFlags, Grid};
use rayon::prelude::*;

pub struct Rasterizer {
    font: Font,
    palette: Palette,
}

impl Rasterizer {
    pub fn new(font_name: Option<&str>) -> Self {
        let font = Font::load(font_name);
        let palette = Palette::default();

        Self { font, palette }
    }

    pub fn canvas_size(&self, cols: usize, rows: usize) -> (usize, usize) {
        (cols * self.font.width(), rows * self.font.height())
    }

    pub fn render_grid(&self, grid: &Grid) -> Canvas {
        let (width, height) = self.canvas_size(grid.width(), grid.height());
        let mut canvas = Canvas::new(width, height, &self.palette);

        // Render each cell
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                if let Some(cell) = grid.get_cell(x, y) {
                    self.render_cell(cell, x, y, &mut canvas);
                }
            }
        }

        canvas
    }

    fn render_cell(&self, cell: &Cell, col: usize, row: usize, canvas: &mut Canvas) {
        let x = col * self.font.width();
        let y = row * self.font.height();

        let (fg, bg) = if cell.flags.contains(CellFlags::REVERSE) {
            (cell.bg_color, cell.fg_color)
        } else {
            (cell.fg_color, cell.bg_color)
        };

        // Get character bitmap with UTF-8 mapping
        let glyph = self.font.get_glyph_utf8(cell.character);

        // Render glyph
        for gy in 0..self.font.height() {
            for gx in 0..self.font.width() {
                let pixel_x = x + gx;
                let pixel_y = y + gy;

                if pixel_x < canvas.width() && pixel_y < canvas.height() {
                    let is_foreground = glyph[gy * self.font.width() + gx];
                    let color = if is_foreground { fg } else { bg };
                    canvas.set_pixel(pixel_x, pixel_y, color);
                }
            }
        }
    }

    /// Render a title string at the specified position with size multiplier
    pub fn render_title(&self, canvas: &mut Canvas, x: i32, y: i32, text: &str, fg_color: u8, bg_color: u8, size: f32) {
        self.font.render_string(canvas, x, y, text, fg_color, bg_color, size);
    }
}
