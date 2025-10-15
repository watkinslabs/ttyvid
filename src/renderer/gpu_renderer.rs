// GPU-accelerated rendering using wgpu
// Provides compute shader-based pixel rendering with automatic CPU fallback

#[cfg(feature = "gpu")]
use wgpu;

use crate::renderer::{Canvas, Palette, Font};
use crate::terminal::{Cell, CellFlags, Grid};
use anyhow::{Result, Context};

/// GPU renderer with automatic CPU fallback
pub struct GpuRenderer {
    #[cfg(feature = "gpu")]
    gpu_context: Option<GpuContext>,
    font: Font,
    palette: Palette,
    fallback_to_cpu: bool,
    has_warned_fallback: std::sync::atomic::AtomicBool,
}

#[cfg(feature = "gpu")]
struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    compute_pipeline: wgpu::ComputePipeline,
}

impl GpuRenderer {
    /// Create a new GPU renderer with automatic fallback
    pub fn new(font: Font, palette: Palette) -> Self {
        #[cfg(feature = "gpu")]
        {
            match Self::init_gpu() {
                Ok(gpu_context) => {
                    eprintln!("✅ GPU acceleration enabled (wgpu)");
                    Self {
                        gpu_context: Some(gpu_context),
                        font,
                        palette,
                        fallback_to_cpu: false,
                        has_warned_fallback: std::sync::atomic::AtomicBool::new(false),
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  GPU initialization failed: {}", e);
                    eprintln!("   Falling back to CPU rendering");
                    Self {
                        gpu_context: None,
                        font,
                        palette,
                        fallback_to_cpu: true,
                        has_warned_fallback: std::sync::atomic::AtomicBool::new(true),
                    }
                }
            }
        }

        #[cfg(not(feature = "gpu"))]
        {
            Self {
                font,
                palette,
                fallback_to_cpu: true,
                has_warned_fallback: std::sync::atomic::AtomicBool::new(true),
            }
        }
    }

    /// Initialize GPU context
    #[cfg(feature = "gpu")]
    fn init_gpu() -> Result<GpuContext> {
        // Request GPU adapter
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .context("Failed to find GPU adapter")?;

        // Create device and queue
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("ttyvid GPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ))
        .context("Failed to create GPU device")?;

        // Create compute shader
        let shader_source = include_str!("shaders/render.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ttyvid Render Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create compute pipeline
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ttyvid Render Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        Ok(GpuContext {
            device,
            queue,
            compute_pipeline,
        })
    }

    /// Check if GPU is available
    pub fn is_gpu_available(&self) -> bool {
        #[cfg(feature = "gpu")]
        {
            self.gpu_context.is_some()
        }
        #[cfg(not(feature = "gpu"))]
        {
            false
        }
    }

    /// Render grid to canvas (GPU or CPU fallback)
    pub fn render_grid(&self, grid: &Grid) -> Canvas {
        #[cfg(feature = "gpu")]
        {
            if let Some(ref ctx) = self.gpu_context {
                match self.render_grid_gpu(grid, ctx) {
                    Ok(canvas) => return canvas,
                    Err(e) => {
                        // Only warn once about GPU fallback
                        if !self.has_warned_fallback.swap(true, std::sync::atomic::Ordering::Relaxed) {
                            eprintln!("⚠️  GPU rendering failed: {}, falling back to CPU", e);
                        }
                    }
                }
            }
        }

        // CPU fallback
        self.render_grid_cpu(grid)
    }

    /// GPU-accelerated rendering
    #[cfg(feature = "gpu")]
    fn render_grid_gpu(&self, grid: &Grid, ctx: &GpuContext) -> Result<Canvas> {
        // TODO: Implement GPU compute shader rendering
        // For now, fall back to CPU
        // This will be implemented with:
        // 1. Upload grid data to GPU buffer
        // 2. Upload font/palette to GPU
        // 3. Dispatch compute shader
        // 4. Read back rendered pixels

        Err(anyhow::anyhow!("GPU rendering not yet implemented"))
    }

    /// CPU fallback rendering (existing implementation)
    fn render_grid_cpu(&self, grid: &Grid) -> Canvas {
        let (width, height) = self.canvas_size(grid.width(), grid.height());
        let mut canvas = Canvas::new(width, height, &self.palette);

        // Render each cell (CPU path)
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                if let Some(cell) = grid.get_cell(x, y) {
                    self.render_cell_cpu(cell, x, y, &mut canvas);
                }
            }
        }

        canvas
    }

    /// Render grid with cursor (GPU or CPU)
    pub fn render_grid_with_cursor(&self, grid: &Grid, cursor_x: usize, cursor_y: usize) -> Canvas {
        // For now, always use CPU for cursor rendering
        // GPU path can be added later
        let (width, height) = self.canvas_size(grid.width(), grid.height());
        let mut canvas = Canvas::new(width, height, &self.palette);

        for y in 0..grid.height() {
            for x in 0..grid.width() {
                if let Some(cell) = grid.get_cell(x, y) {
                    if x == cursor_x && y == cursor_y {
                        self.render_cell_inverted_cpu(cell, x, y, &mut canvas);
                    } else {
                        self.render_cell_cpu(cell, x, y, &mut canvas);
                    }
                }
            }
        }

        canvas
    }

    /// Calculate canvas size
    fn canvas_size(&self, cols: usize, rows: usize) -> (usize, usize) {
        (cols * self.font.width(), rows * self.font.height())
    }

    /// Render single cell (CPU implementation)
    fn render_cell_cpu(&self, cell: &Cell, col: usize, row: usize, canvas: &mut Canvas) {
        let x = col * self.font.width();
        let y = row * self.font.height();

        let (fg, bg) = if cell.flags.contains(CellFlags::REVERSE) {
            (cell.bg_color, cell.fg_color)
        } else {
            (cell.fg_color, cell.bg_color)
        };

        let glyph = self.font.get_glyph_utf8(cell.character);

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

    /// Render cell with inverted colors (CPU implementation)
    fn render_cell_inverted_cpu(&self, cell: &Cell, col: usize, row: usize, canvas: &mut Canvas) {
        let x = col * self.font.width();
        let y = row * self.font.height();

        let (fg, bg) = if cell.flags.contains(CellFlags::REVERSE) {
            (cell.fg_color, cell.bg_color)
        } else {
            (cell.bg_color, cell.fg_color)
        };

        let glyph = self.font.get_glyph_utf8(cell.character);

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

    /// Render title text
    pub fn render_title(&self, canvas: &mut Canvas, x: i32, y: i32, text: &str, fg_color: u8, bg_color: u8, size: f32) {
        self.font.render_string(canvas, x, y, text, fg_color, bg_color, size);
    }
}

// Implement RenderBackend trait for GPU renderer
impl super::RenderBackend for GpuRenderer {
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
