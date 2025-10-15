mod font;
mod canvas;
mod colors;
mod truetype_font;

#[cfg(feature = "gpu")]
mod gpu_renderer;

pub use font::Font;
pub use canvas::Canvas;
pub use colors::Palette;
pub use truetype_font::{TrueTypeFont, query_terminal_font};

#[cfg(feature = "gpu")]
pub use gpu_renderer::GpuRenderer;

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

    /// Create a rasterizer with a custom font (for TrueType support)
    pub fn with_font(font: Font) -> Self {
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

    /// Render grid with cursor visible at specified position
    pub fn render_grid_with_cursor(&self, grid: &Grid, cursor_x: usize, cursor_y: usize) -> Canvas {
        let (width, height) = self.canvas_size(grid.width(), grid.height());
        let mut canvas = Canvas::new(width, height, &self.palette);

        // Render each cell
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                if let Some(cell) = grid.get_cell(x, y) {
                    // Invert colors at cursor position
                    if x == cursor_x && y == cursor_y {
                        self.render_cell_inverted(cell, x, y, &mut canvas);
                    } else {
                        self.render_cell(cell, x, y, &mut canvas);
                    }
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

        // Get character bitmap with UTF-8 mapping (supports both FD and TrueType fonts)
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

    /// Render cell with inverted colors (for cursor)
    fn render_cell_inverted(&self, cell: &Cell, col: usize, row: usize, canvas: &mut Canvas) {
        let x = col * self.font.width();
        let y = row * self.font.height();

        // Invert fg/bg for cursor
        let (fg, bg) = if cell.flags.contains(CellFlags::REVERSE) {
            (cell.fg_color, cell.bg_color)  // Already reversed, so swap back
        } else {
            (cell.bg_color, cell.fg_color)  // Normal, so invert
        };

        // Get character bitmap with UTF-8 mapping (supports both FD and TrueType fonts)
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

/// Create a renderer with automatic GPU/CPU selection
/// When compiled with --features gpu, attempts to use GPU and falls back to CPU
/// When compiled without gpu feature, always uses CPU
#[cfg(feature = "gpu")]
pub fn create_renderer_auto(font_name: Option<&str>) -> Box<dyn RenderBackend> {
    let font = Font::load(font_name);
    let palette = Palette::default();

    // Try GPU first, automatically falls back to CPU if GPU unavailable
    Box::new(GpuRenderer::new(font, palette))
}

#[cfg(not(feature = "gpu"))]
pub fn create_renderer_auto(font_name: Option<&str>) -> Box<dyn RenderBackend> {
    let font = Font::load(font_name);
    Box::new(Rasterizer::new(font_name))
}

/// Trait for render backends (CPU or GPU)
pub trait RenderBackend {
    fn render_grid(&self, grid: &Grid) -> Canvas;
    fn render_grid_with_cursor(&self, grid: &Grid, cursor_x: usize, cursor_y: usize) -> Canvas;
    fn canvas_size(&self, cols: usize, rows: usize) -> (usize, usize);
    fn render_title(&self, canvas: &mut Canvas, x: i32, y: i32, text: &str, fg_color: u8, bg_color: u8, size: f32);
}

// Implement RenderBackend for CPU Rasterizer
impl RenderBackend for Rasterizer {
    fn render_grid(&self, grid: &Grid) -> Canvas {
        self.render_grid(grid)
    }

    fn render_grid_with_cursor(&self, grid: &Grid, cursor_x: usize, cursor_y: usize) -> Canvas {
        self.render_grid_with_cursor(grid, cursor_x, cursor_y)
    }

    fn canvas_size(&self, cols: usize, rows: usize) -> (usize, usize) {
        self.canvas_size(cols, rows)
    }

    fn render_title(&self, canvas: &mut Canvas, x: i32, y: i32, text: &str, fg_color: u8, bg_color: u8, size: f32) {
        self.render_title(canvas, x, y, text, fg_color, bg_color, size)
    }
}
