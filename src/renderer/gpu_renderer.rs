// GPU-accelerated rendering using wgpu
// Provides compute shader-based pixel rendering with automatic CPU fallback

#[cfg(feature = "gpu")]
use wgpu;

#[cfg(feature = "gpu")]
use wgpu::util::DeviceExt;

#[cfg(feature = "gpu")]
use bytemuck;

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
struct RenderState {
    cell_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,
    staging_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    canvas_width: usize,
    canvas_height: usize,
    grid_width: usize,
    grid_height: usize,
}

#[cfg(feature = "gpu")]
struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    compute_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    // Pre-allocated persistent buffers (constant across all frames)
    font_buffer: wgpu::Buffer,
    palette_buffer: wgpu::Buffer,
    // Render state (created on first render, reused for all subsequent frames)
    render_state: std::cell::RefCell<Option<RenderState>>,
}

impl GpuRenderer {
    /// Create a new GPU renderer with automatic fallback
    pub fn new(font: Font, palette: Palette) -> Self {
        #[cfg(feature = "gpu")]
        {
            match Self::init_gpu(&font, &palette) {
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

    /// Initialize GPU context with font and palette uploaded once
    #[cfg(feature = "gpu")]
    fn init_gpu(font: &Font, palette: &Palette) -> Result<GpuContext> {
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

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[
                // Binding 0: grid_cells (storage buffer, read-only)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 1: font_data (storage buffer, read-only)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 2: palette (storage buffer, read-only)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 3: output (storage buffer, read-write)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 4: params (uniform buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create compute shader
        let shader_source = include_str!("shaders/render.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ttyvid Render Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Create compute pipeline
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ttyvid Render Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
        });

        // Upload font data ONCE - this is constant across all frames
        let cell_width = font.width();
        let cell_height = font.height();
        let mut font_data = Vec::with_capacity(256 * cell_width * cell_height);
        for char_idx in 0u8..=255u8 {
            let glyph = font.get_glyph(char_idx);
            for pixel in glyph {
                font_data.push(if pixel { 1u32 } else { 0u32 });
            }
        }

        let font_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Font Data Buffer (Persistent)"),
            contents: bytemuck::cast_slice(&font_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Upload palette ONCE - this is constant across all frames
        let dummy_palette = vec![0u32; 256];
        let palette_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Palette Buffer (Persistent)"),
            contents: bytemuck::cast_slice(&dummy_palette),
            usage: wgpu::BufferUsages::STORAGE,
        });

        Ok(GpuContext {
            device,
            queue,
            compute_pipeline,
            bind_group_layout,
            font_buffer,
            palette_buffer,
            render_state: std::cell::RefCell::new(None),
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

    /// BATCH RENDER: Process multiple grids at once (GPU ONLY - eliminates sync overhead)
    #[cfg(feature = "gpu")]
    pub fn render_grids_batch(&self, grids: &[Grid]) -> Result<Vec<Canvas>> {
        if grids.is_empty() {
            return Ok(Vec::new());
        }

        // Try GPU batch rendering
        if let Some(ref ctx) = self.gpu_context {
            match self.render_grids_batch_gpu(grids, ctx) {
                Ok(canvases) => return Ok(canvases),
                Err(e) => {
                    if !self.has_warned_fallback.swap(true, std::sync::atomic::Ordering::Relaxed) {
                        eprintln!("⚠️  GPU batch rendering failed: {}, falling back to CPU", e);
                    }
                }
            }
        }

        // CPU fallback - render each grid individually
        Ok(grids.iter().map(|grid| self.render_grid_cpu(grid)).collect())
    }

    /// BATCH GPU RENDERING: Render ALL grids in one GPU operation (ONE sync point!)
    #[cfg(feature = "gpu")]
    fn render_grids_batch_gpu(&self, grids: &[Grid], ctx: &GpuContext) -> Result<Vec<Canvas>> {
        if grids.is_empty() {
            return Ok(Vec::new());
        }

        let num_frames = grids.len();
        let (canvas_width, canvas_height) = self.canvas_size(grids[0].width(), grids[0].height());
        let cell_width = self.font.width();
        let cell_height = self.font.height();
        let grid_width = grids[0].width();
        let grid_height = grids[0].height();

        eprintln!("🚀 GPU BATCH MODE: Rendering {} frames at once!", num_frames);

        // Prepare ALL frames' cell data at once
        let cells_per_frame = grid_width * grid_height * 4;
        let mut all_cell_data = Vec::with_capacity(num_frames * cells_per_frame);

        for grid in grids {
            for row in 0..grid_height {
                for col in 0..grid_width {
                    if let Some(cell) = grid.get_cell(col, row) {
                        let char_code = cell.character as u32;
                        let flags = if cell.flags.contains(CellFlags::REVERSE) { 1u32 } else { 0u32 };
                        all_cell_data.push(char_code);
                        all_cell_data.push(cell.fg_color as u32);
                        all_cell_data.push(cell.bg_color as u32);
                        all_cell_data.push(flags);
                    } else {
                        all_cell_data.extend_from_slice(&[32u32, 7u32, 0u32, 0u32]);
                    }
                }
            }
        }

        // Create MEGA cell buffer for all frames
        let cell_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Batch Cell Data Buffer (ALL FRAMES)"),
            contents: bytemuck::cast_slice(&all_cell_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Create MEGA output buffer for all frames
        let pixels_per_frame = canvas_width * canvas_height;
        let total_output_size = (num_frames * pixels_per_frame * std::mem::size_of::<u32>()) as u64;
        let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Batch Output Buffer (ALL FRAMES)"),
            size: total_output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Params
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct RenderParams {
            canvas_width: u32,
            canvas_height: u32,
            grid_width: u32,
            grid_height: u32,
            cell_width: u32,
            cell_height: u32,
            font_data_stride: u32,
            padding: u32,
        }

        let params = RenderParams {
            canvas_width: canvas_width as u32,
            canvas_height: canvas_height as u32,
            grid_width: grid_width as u32,
            grid_height: grid_height as u32,
            cell_width: cell_width as u32,
            cell_height: cell_height as u32,
            font_data_stride: (cell_width * cell_height) as u32,
            padding: 0,
        };

        let params_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Params Buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create bind group for batch rendering
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Batch Render Bind Group"),
            layout: &ctx.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cell_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: ctx.font_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: ctx.palette_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create separate output buffers for each frame (no overwriting!)
        let frame_output_size = (pixels_per_frame * std::mem::size_of::<u32>()) as u64;
        let mut frame_outputs = Vec::with_capacity(num_frames);
        let mut frame_bind_groups = Vec::with_capacity(num_frames);

        for frame_idx in 0..num_frames {
            let frame_output = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Frame {} Output", frame_idx)),
                size: frame_output_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            // Create bind group for this frame (different cell data offset, different output buffer)
            let frame_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("Frame {} Bind Group", frame_idx)),
                layout: &ctx.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        // Slice the cell buffer for this frame
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &cell_buffer,
                            offset: (frame_idx * cells_per_frame * 4) as u64,
                            size: Some(std::num::NonZeroU64::new((cells_per_frame * 4) as u64).unwrap()),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: ctx.font_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: ctx.palette_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: frame_output.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            frame_outputs.push(frame_output);
            frame_bind_groups.push(frame_bind_group);
        }

        // Dispatch ALL frames in ONE command buffer!
        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Batch Render Encoder"),
        });

        let workgroup_size = 8;
        let dispatch_x = (canvas_width + workgroup_size - 1) / workgroup_size;
        let dispatch_y = (canvas_height + workgroup_size - 1) / workgroup_size;

        // Render all frames in one command buffer (GPU can pipeline these!)
        for frame_idx in 0..num_frames {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(&format!("Frame {}", frame_idx)),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&ctx.compute_pipeline);
            compute_pass.set_bind_group(0, &frame_bind_groups[frame_idx], &[]);
            compute_pass.dispatch_workgroups(dispatch_x as u32, dispatch_y as u32, 1);
        }

        // Create staging buffer for readback
        let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Batch Staging Buffer"),
            size: total_output_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copy all frame outputs to staging buffer
        for (frame_idx, frame_output) in frame_outputs.iter().enumerate() {
            let staging_offset = (frame_idx * pixels_per_frame * std::mem::size_of::<u32>()) as u64;
            encoder.copy_buffer_to_buffer(frame_output, 0, &staging_buffer, staging_offset, frame_output_size);
        }

        // Submit ALL work in ONE batch!
        ctx.queue.submit(std::iter::once(encoder.finish()));

        // ONE sync point for ALL frames!
        eprintln!("⏳ Syncing GPU→CPU (ONE sync for {} frames)...", num_frames);
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        ctx.device.poll(wgpu::Maintain::Wait);
        receiver.recv().context("Failed to receive buffer mapping result")??;

        let data = buffer_slice.get_mapped_range();
        let all_pixels: &[u32] = bytemuck::cast_slice(&data);

        // Extract canvases from the mega buffer
        let mut canvases = Vec::with_capacity(num_frames);
        for frame_idx in 0..num_frames {
            let frame_offset = frame_idx * pixels_per_frame;
            let frame_pixels = &all_pixels[frame_offset..frame_offset + pixels_per_frame];

            let mut canvas = Canvas::new(canvas_width, canvas_height, &self.palette);
            for y in 0..canvas_height {
                for x in 0..canvas_width {
                    let idx = y * canvas_width + x;
                    canvas.set_pixel(x, y, frame_pixels[idx] as u8);
                }
            }
            canvases.push(canvas);
        }

        drop(data);
        staging_buffer.unmap();

        eprintln!("✅ GPU batch rendering complete!");
        Ok(canvases)
    }

    /// GPU-accelerated rendering (AGGRESSIVELY OPTIMIZED - all buffers persistent)
    #[cfg(feature = "gpu")]
    fn render_grid_gpu(&self, grid: &Grid, ctx: &GpuContext) -> Result<Canvas> {
        let (canvas_width, canvas_height) = self.canvas_size(grid.width(), grid.height());
        let cell_width = self.font.width();
        let cell_height = self.font.height();
        let grid_width = grid.width();
        let grid_height = grid.height();

        // Prepare grid cell data
        let mut cell_data = Vec::with_capacity(grid_width * grid_height * 4);
        for row in 0..grid_height {
            for col in 0..grid_width {
                if let Some(cell) = grid.get_cell(col, row) {
                    let char_code = cell.character as u32;
                    let flags = if cell.flags.contains(CellFlags::REVERSE) { 1u32 } else { 0u32 };
                    cell_data.push(char_code);
                    cell_data.push(cell.fg_color as u32);
                    cell_data.push(cell.bg_color as u32);
                    cell_data.push(flags);
                } else {
                    cell_data.extend_from_slice(&[32u32, 7u32, 0u32, 0u32]);
                }
            }
        }

        // Check if we need to initialize or resize buffers
        let mut render_state = ctx.render_state.borrow_mut();
        let needs_init = match render_state.as_ref() {
            None => true,
            Some(state) => {
                state.canvas_width != canvas_width
                    || state.canvas_height != canvas_height
                    || state.grid_width != grid_width
                    || state.grid_height != grid_height
            }
        };

        if needs_init {
            // Initialize all render buffers (first render or dimensions changed)
            #[repr(C)]
            #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
            struct RenderParams {
                canvas_width: u32,
                canvas_height: u32,
                grid_width: u32,
                grid_height: u32,
                cell_width: u32,
                cell_height: u32,
                font_data_stride: u32,
                padding: u32,
            }

            let params = RenderParams {
                canvas_width: canvas_width as u32,
                canvas_height: canvas_height as u32,
                grid_width: grid_width as u32,
                grid_height: grid_height as u32,
                cell_width: cell_width as u32,
                cell_height: cell_height as u32,
                font_data_stride: (cell_width * cell_height) as u32,
                padding: 0,
            };

            let cell_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cell Data Buffer (Persistent)"),
                contents: bytemuck::cast_slice(&cell_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

            let output_size = (canvas_width * canvas_height * std::mem::size_of::<u32>()) as u64;
            let output_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Output Buffer (Persistent)"),
                size: output_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            let params_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Params Buffer (Persistent)"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            let staging_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Staging Buffer (Persistent)"),
                size: output_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Render Bind Group (Persistent)"),
                layout: &ctx.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: cell_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: ctx.font_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: ctx.palette_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            *render_state = Some(RenderState {
                cell_buffer,
                output_buffer,
                params_buffer,
                staging_buffer,
                bind_group,
                canvas_width,
                canvas_height,
                grid_width,
                grid_height,
            });
        } else {
            // Reuse existing buffers, just update cell data
            let state = render_state.as_ref().unwrap();
            ctx.queue.write_buffer(&state.cell_buffer, 0, bytemuck::cast_slice(&cell_data));
        }

        let state = render_state.as_ref().unwrap();
        let output_size = (canvas_width * canvas_height * std::mem::size_of::<u32>()) as u64;

        // Dispatch compute shader
        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Render Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&ctx.compute_pipeline);
            compute_pass.set_bind_group(0, &state.bind_group, &[]);

            let workgroup_size = 8;
            let dispatch_x = (canvas_width + workgroup_size - 1) / workgroup_size;
            let dispatch_y = (canvas_height + workgroup_size - 1) / workgroup_size;
            compute_pass.dispatch_workgroups(dispatch_x as u32, dispatch_y as u32, 1);
        }

        encoder.copy_buffer_to_buffer(&state.output_buffer, 0, &state.staging_buffer, 0, output_size);
        ctx.queue.submit(std::iter::once(encoder.finish()));

        // Read back results
        let buffer_slice = state.staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        ctx.device.poll(wgpu::Maintain::Wait);
        receiver.recv().context("Failed to receive buffer mapping result")??;

        let data = buffer_slice.get_mapped_range();
        let pixels: &[u32] = bytemuck::cast_slice(&data);

        // Convert to Canvas
        let mut canvas = Canvas::new(canvas_width, canvas_height, &self.palette);
        for y in 0..canvas_height {
            for x in 0..canvas_width {
                let idx = y * canvas_width + x;
                canvas.set_pixel(x, y, pixels[idx] as u8);
            }
        }

        drop(data);
        state.staging_buffer.unmap();

        Ok(canvas)
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
