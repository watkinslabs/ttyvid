use anyhow::{Result, bail, Context};
use image::{Rgba, RgbaImage};
use std::path::Path;
use std::io::Cursor;
use crate::theme::{Layer, NineSliceConfig};
use crate::renderer::Canvas;

// Include embedded layers
include!(concat!(env!("OUT_DIR"), "/embedded_layers.rs"));

pub struct LayerImage {
    pub frames: Vec<RgbaImage>,  // Multiple frames for animated GIFs
    pub delays: Vec<u16>,         // Delay for each frame in centiseconds (1/100 second)
    pub width: u32,
    pub height: u32,
    pub is_animated: bool,
}

impl LayerImage {
    /// Get a specific frame or the first frame if index is out of bounds
    pub fn get_frame(&self, frame_index: usize) -> &RgbaImage {
        if frame_index < self.frames.len() {
            &self.frames[frame_index]
        } else {
            &self.frames[0]
        }
    }

    /// Get total number of frames
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get the delay for a specific frame in centiseconds
    pub fn get_delay(&self, frame_index: usize) -> u16 {
        if frame_index < self.delays.len() {
            self.delays[frame_index]
        } else {
            10 // Default 100ms (10 centiseconds)
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        // Try embedded first if path has no directory component (just filename)
        if path.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true) {
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if let Some(&embedded_data) = EMBEDDED_LAYERS.get(filename) {
                    return Self::load_from_bytes(embedded_data, filename);
                }
            }
        }

        // Fall back to filesystem
        if path.extension().and_then(|e| e.to_str()) == Some("gif") {
            Self::load_animated_gif(path)
        } else {
            let img = image::open(path)
                .with_context(|| format!("Failed to load layer image: {}", path.display()))?;

            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();

            Ok(Self {
                frames: vec![rgba],
                delays: vec![10],
                width,
                height,
                is_animated: false,
            })
        }
    }

    pub fn load_from_bytes(data: &[u8], name: &str) -> Result<Self> {
        if name.ends_with(".gif") {
            Self::load_animated_gif_from_bytes(data, name)
        } else {
            let img = image::load_from_memory(data)
                .with_context(|| format!("Failed to load embedded layer image: {}", name))?;

            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();

            Ok(Self {
                frames: vec![rgba],
                delays: vec![10],
                width,
                height,
                is_animated: false,
            })
        }
    }

    fn load_animated_gif(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open GIF: {}", path.display()))?;

        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);

        let mut decoder = decoder.read_info(file)
            .with_context(|| format!("Failed to decode GIF: {}", path.display()))?;

        let mut frames = Vec::new();
        let mut delays = Vec::new();
        let (width, height) = (decoder.width() as u32, decoder.height() as u32);

        while let Some(frame) = decoder.read_next_frame()? {
            let rgba_data = frame.buffer.to_vec();
            let frame_width = frame.width as u32;
            let frame_height = frame.height as u32;
            let frame_left = frame.left as u32;
            let frame_top = frame.top as u32;

            let frame_image = RgbaImage::from_vec(frame_width, frame_height, rgba_data)
                .context("Failed to create image from GIF frame")?;

            // Pad frame to full GIF dimensions if needed
            let padded_frame = if frame_width != width || frame_height != height {
                let mut padded = RgbaImage::new(width, height);
                // Copy frame data at the correct offset
                for y in 0..frame_height {
                    for x in 0..frame_width {
                        let pixel = frame_image.get_pixel(x, y);
                        padded.put_pixel(x + frame_left, y + frame_top, *pixel);
                    }
                }
                padded
            } else {
                frame_image
            };

            frames.push(padded_frame);
            delays.push(frame.delay); // delay in centiseconds
        }

        if frames.is_empty() {
            bail!("GIF has no frames: {}", path.display());
        }

        let is_animated = frames.len() > 1;

        Ok(Self {
            frames,
            delays,
            width,
            height,
            is_animated,
        })
    }

    fn load_animated_gif_from_bytes(data: &[u8], name: &str) -> Result<Self> {
        let cursor = Cursor::new(data);

        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);

        let mut decoder = decoder.read_info(cursor)
            .with_context(|| format!("Failed to decode embedded GIF: {}", name))?;

        let mut frames = Vec::new();
        let mut delays = Vec::new();
        let (width, height) = (decoder.width() as u32, decoder.height() as u32);

        while let Some(frame) = decoder.read_next_frame()? {
            let rgba_data = frame.buffer.to_vec();
            let frame_width = frame.width as u32;
            let frame_height = frame.height as u32;
            let frame_left = frame.left as u32;
            let frame_top = frame.top as u32;

            let frame_image = RgbaImage::from_vec(frame_width, frame_height, rgba_data)
                .context("Failed to create image from embedded GIF frame")?;

            // Pad frame to full GIF dimensions if needed
            let padded_frame = if frame_width != width || frame_height != height {
                let mut padded = RgbaImage::new(width, height);
                for y in 0..frame_height {
                    for x in 0..frame_width {
                        let pixel = frame_image.get_pixel(x, y);
                        padded.put_pixel(x + frame_left, y + frame_top, *pixel);
                    }
                }
                padded
            } else {
                frame_image
            };

            frames.push(padded_frame);
            delays.push(frame.delay);
        }

        if frames.is_empty() {
            bail!("Embedded GIF has no frames: {}", name);
        }

        let is_animated = frames.len() > 1;

        Ok(Self {
            frames,
            delays,
            width,
            height,
            is_animated,
        })
    }

    /// Perform 9-slice scaling on a specific frame
    /// Python uses inclusive coordinates where width = right - left + 1
    /// and creates 1-pixel overlaps between regions
    pub fn nineslice_scale(&self, config: &NineSliceConfig, target_width: u32, target_height: u32, frame_index: usize) -> Result<RgbaImage> {
        let image = self.get_frame(frame_index);
        use crate::theme::NineSliceValue;

        // Resolve "auto" values to image dimensions
        let resolve = |val: &NineSliceValue, default: i32| -> i32 {
            match val {
                NineSliceValue::Auto => default,
                NineSliceValue::Value(v) => *v,
            }
        };

        let max_x = (self.width - 1) as i32;
        let max_y = (self.height - 1) as i32;

        let outer_left = resolve(&config.outer_left, 0);
        let outer_top = resolve(&config.outer_top, 0);
        let outer_right = resolve(&config.outer_right, max_x);
        let outer_bottom = resolve(&config.outer_bottom, max_y);
        let inner_left = resolve(&config.inner_left, 0);
        let inner_top = resolve(&config.inner_top, 0);
        let inner_right = resolve(&config.inner_right, max_x);
        let inner_bottom = resolve(&config.inner_bottom, max_y);

        // Python: src_1 = rect(outer.left, outer.top, inner.left, inner.top)
        // Width calculation: inner.left - outer.left + 1 (inclusive coordinates)
        let src_1_w = inner_left - outer_left + 1;
        let src_1_h = inner_top - outer_top + 1;

        // Python: src_2 = rect(inner.left+1, outer.top, inner.right-1, inner.top)
        // Creates 1-pixel overlap with src_1 and src_3
        let src_2_w = (inner_right - 1) - (inner_left + 1) + 1;
        let src_2_h = inner_top - outer_top + 1;

        // Python: src_3 = rect(inner.right, outer.top, outer.right, inner.top)
        let src_3_w = outer_right - inner_right + 1;
        let src_3_h = inner_top - outer_top + 1;

        // Python: src_9 = rect(inner.right, inner.bottom, outer.right, outer.bottom)
        let src_9_w = outer_right - inner_right + 1;
        let src_9_h = outer_bottom - inner_bottom + 1;

        // Center region
        let center_src_w = src_2_w;
        let center_src_h = (inner_bottom - 1) - (inner_top + 1) + 1;

        // Python: dst_inner = rect(dst.left+src_1.width-1, dst.top+src_1.height-1,
        //                          dst.right-src_9.width+1, dst.bottom-src_9.height+1)
        let target_w = target_width as i32;
        let target_h = target_height as i32;

        let dst_inner_left = 0 + src_1_w - 1;
        let dst_inner_top = 0 + src_1_h - 1;
        let dst_inner_right = (target_w - 1) - src_9_w + 1;
        let dst_inner_bottom = (target_h - 1) - src_9_h + 1;

        let center_dst_w = dst_inner_right - dst_inner_left - 1;
        let center_dst_h = dst_inner_bottom - dst_inner_top - 1;

        if center_dst_w < 0 || center_dst_h < 0 {
            bail!("Target dimensions too small for 9-slice scaling");
        }

        let mut result = RgbaImage::new(target_width, target_height);

        // Corner 1 (top-left)
        self.blit_region_from(image, &mut result,
            outer_left, outer_top, src_1_w, src_1_h,
            0, 0, src_1_w, src_1_h)?;

        // Corner 3 (top-right)
        self.blit_region_from(image, &mut result,
            inner_right, outer_top, src_3_w, src_3_h,
            dst_inner_right, 0, src_3_w, src_3_h)?;

        // Corner 7 (bottom-left)
        self.blit_region_from(image, &mut result,
            outer_left, inner_bottom, src_1_w, src_9_h,
            0, dst_inner_bottom, src_1_w, src_9_h)?;

        // Corner 9 (bottom-right)
        self.blit_region_from(image, &mut result,
            inner_right, inner_bottom, src_9_w, src_9_h,
            dst_inner_right, dst_inner_bottom, src_9_w, src_9_h)?;

        // Edge 2 (top)
        self.scale_blit_from(image, &mut result,
            inner_left + 1, outer_top, src_2_w, src_2_h,
            dst_inner_left + 1, 0, center_dst_w, src_2_h)?;

        // Edge 8 (bottom)
        self.scale_blit_from(image, &mut result,
            inner_left + 1, inner_bottom, src_2_w, src_9_h,
            dst_inner_left + 1, dst_inner_bottom, center_dst_w, src_9_h)?;

        // Edge 4 (left)
        self.scale_blit_from(image, &mut result,
            outer_left, inner_top + 1, src_1_w, center_src_h,
            0, dst_inner_top + 1, src_1_w, center_dst_h)?;

        // Edge 6 (right)
        self.scale_blit_from(image, &mut result,
            inner_right, inner_top + 1, src_3_w, center_src_h,
            dst_inner_right, dst_inner_top + 1, src_3_w, center_dst_h)?;

        // Center (region 5)
        self.scale_blit_from(image, &mut result,
            inner_left + 1, inner_top + 1, center_src_w, center_src_h,
            dst_inner_left + 1, dst_inner_top + 1, center_dst_w, center_dst_h)?;

        Ok(result)
    }

    fn blit_region_from(&self, src: &RgbaImage, dst: &mut RgbaImage,
                   src_x: i32, src_y: i32, src_w: i32, src_h: i32,
                   dst_x: i32, dst_y: i32, dst_w: i32, dst_h: i32) -> Result<()> {
        for y in 0..dst_h.min(src_h) {
            for x in 0..dst_w.min(src_w) {
                let sx = (src_x + x) as u32;
                let sy = (src_y + y) as u32;
                let dx = (dst_x + x) as u32;
                let dy = (dst_y + y) as u32;

                if sx < self.width && sy < self.height && dx < dst.width() && dy < dst.height() {
                    let pixel = src.get_pixel(sx, sy);
                    dst.put_pixel(dx, dy, *pixel);
                }
            }
        }
        Ok(())
    }

    fn scale_blit_from(&self, src: &RgbaImage, dst: &mut RgbaImage,
                  src_x: i32, src_y: i32, src_w: i32, src_h: i32,
                  dst_x: i32, dst_y: i32, dst_w: i32, dst_h: i32) -> Result<()> {
        if src_w <= 0 || src_h <= 0 || dst_w <= 0 || dst_h <= 0 {
            return Ok(());
        }

        for dy in 0..dst_h {
            for dx in 0..dst_w {
                // Map destination pixel to source pixel (nearest neighbor)
                let sx = src_x + (dx * src_w / dst_w);
                let sy = src_y + (dy * src_h / dst_h);

                let src_px = (sx as u32, sy as u32);
                let dst_px = ((dst_x + dx) as u32, (dst_y + dy) as u32);

                if src_px.0 < self.width && src_px.1 < self.height
                   && dst_px.0 < dst.width() && dst_px.1 < dst.height() {
                    let pixel = src.get_pixel(src_px.0, src_px.1);
                    dst.put_pixel(dst_px.0, dst_px.1, *pixel);
                }
            }
        }
        Ok(())
    }

    /// Composite a specific frame of this layer onto a canvas
    pub fn composite_onto(&self, canvas: &mut Canvas, offset_x: i32, offset_y: i32, palette: &[u8], frame_index: usize) {
        let image = self.get_frame(frame_index);
        let canvas_width = canvas.width();
        let canvas_height = canvas.height();

        for y in 0..self.height {
            for x in 0..self.width {
                let dst_x = x as i32 + offset_x;
                let dst_y = y as i32 + offset_y;

                if dst_x >= 0 && dst_x < canvas_width as i32
                   && dst_y >= 0 && dst_y < canvas_height as i32 {
                    let pixel = image.get_pixel(x, y);

                    // Convert RGBA to palette index (find nearest color)
                    if pixel[3] > 128 { // Check alpha threshold
                        let color_idx = self.find_nearest_palette_color(pixel, palette);
                        canvas.set_pixel(dst_x as usize, dst_y as usize, color_idx);
                    }
                }
            }
        }
    }

    fn find_nearest_palette_color(&self, pixel: &Rgba<u8>, palette: &[u8]) -> u8 {
        let r = pixel[0] as i32;
        let g = pixel[1] as i32;
        let b = pixel[2] as i32;

        let mut best_idx = 0;
        let mut best_dist = i32::MAX;

        for i in 0..(palette.len() / 3) {
            let pr = palette[i * 3] as i32;
            let pg = palette[i * 3 + 1] as i32;
            let pb = palette[i * 3 + 2] as i32;

            let dr = r - pr;
            let dg = g - pg;
            let db = b - pb;
            let dist = dr * dr + dg * dg + db * db;

            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }

            if dist == 0 {
                break;
            }
        }

        best_idx as u8
    }
}

pub struct LayerRenderer {
    layers: Vec<(LayerImage, Layer)>,
}

impl LayerRenderer {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
        }
    }

    pub fn add_layer(&mut self, image: LayerImage, layer: Layer) {
        self.layers.push((image, layer));
    }

    /// Calculate which frame to display for an animated layer based on elapsed time
    fn calculate_frame_index(&self, image: &LayerImage, layer: &Layer, current_time_ms: f64) -> usize {
        if !image.is_animated || image.frame_count() == 1 {
            return 0;
        }

        let anim_config = layer.animation.as_ref();
        let speed = anim_config.map(|a| a.speed).unwrap_or(1.0);
        let should_loop = anim_config.map(|a| a.r#loop).unwrap_or(true);
        let start_frame = anim_config.map(|a| a.start_frame).unwrap_or(0);

        // Calculate total animation duration in milliseconds
        let total_duration_ms: f64 = image.delays.iter()
            .map(|&d| d as f64 * 10.0) // Convert centiseconds to milliseconds
            .sum();

        if total_duration_ms == 0.0 {
            return start_frame;
        }

        // Apply speed multiplier
        let adjusted_time_ms = current_time_ms * speed;

        // Calculate elapsed time within animation
        let elapsed = if should_loop {
            adjusted_time_ms % total_duration_ms
        } else {
            adjusted_time_ms.min(total_duration_ms)
        };

        // Find which frame we should be displaying
        let mut acc_time = 0.0;
        for (idx, &delay) in image.delays.iter().enumerate() {
            acc_time += delay as f64 * 10.0; // Convert to ms
            if elapsed < acc_time {
                return (start_frame + idx) % image.frame_count();
            }
        }

        // Fallback to last frame if not looping
        if should_loop {
            start_frame
        } else {
            image.frame_count() - 1
        }
    }

    pub fn render_underlays(&self, canvas: &mut Canvas, palette: &[u8], current_time_ms: f64) {
        for (image, layer) in &self.layers {
            if layer.depth < 0 {
                let frame_index = self.calculate_frame_index(image, layer, current_time_ms);
                self.render_layer(image, layer, canvas, palette, frame_index);
            }
        }
    }

    pub fn render_overlays(&self, canvas: &mut Canvas, palette: &[u8], current_time_ms: f64) {
        for (image, layer) in &self.layers {
            if layer.depth >= 0 {
                let frame_index = self.calculate_frame_index(image, layer, current_time_ms);
                self.render_layer(image, layer, canvas, palette, frame_index);
            }
        }
    }

    fn render_layer(&self, image: &LayerImage, layer: &Layer, canvas: &mut Canvas, palette: &[u8], frame_index: usize) {
        use crate::theme::LayerMode;

        match layer.mode {
            LayerMode::Copy => self.render_copy(image, layer, canvas, palette, frame_index),
            LayerMode::Center => self.render_center(image, layer, canvas, palette, frame_index),
            LayerMode::NineSlice => self.render_9slice(image, layer, canvas, palette, frame_index),
            LayerMode::ThreeSlice => self.render_9slice(image, layer, canvas, palette, frame_index), // TODO: implement proper 3-slice
            LayerMode::Scale => self.render_scale(image, layer, canvas, palette, frame_index),
            LayerMode::Tile => self.render_tile(image, layer, canvas, palette, frame_index),
            LayerMode::Stretch | LayerMode::None => {
                // For now, treat these like copy
                self.render_copy(image, layer, canvas, palette, frame_index);
            }
        }
    }

    fn render_copy(&self, image: &LayerImage, layer: &Layer, canvas: &mut Canvas, palette: &[u8], frame_index: usize) {
        // Copy mode: copy source bounds to dst bounds without scaling
        let (dst_x, dst_y, _dst_w, _dst_h) = self.calculate_dst_rect(layer, canvas, image);
        image.composite_onto(canvas, dst_x, dst_y, palette, frame_index);
    }

    fn render_center(&self, image: &LayerImage, _layer: &Layer, canvas: &mut Canvas, palette: &[u8], frame_index: usize) {
        // Center mode: center the image on the canvas
        let canvas_width = canvas.width() as i32;
        let canvas_height = canvas.height() as i32;
        let image_width = image.width as i32;
        let image_height = image.height as i32;

        let offset_x = (canvas_width - image_width) / 2;
        let offset_y = (canvas_height - image_height) / 2;

        image.composite_onto(canvas, offset_x, offset_y, palette, frame_index);
    }

    fn render_9slice(&self, image: &LayerImage, layer: &Layer, canvas: &mut Canvas, palette: &[u8], frame_index: usize) {
        // 9-slice mode: scale to dst_bounds using 9-slice algorithm
        if let Some(ref nineslice_config) = layer.nineslice {
            let (dst_x, dst_y, dst_w, dst_h) = self.calculate_dst_rect(layer, canvas, image);

            // Scale to the calculated destination size
            match image.nineslice_scale(nineslice_config, dst_w as u32, dst_h as u32, frame_index) {
                Ok(scaled_image) => {
                    let scaled_layer_image = LayerImage {
                        frames: vec![scaled_image],
                        delays: vec![10],
                        width: dst_w as u32,
                        height: dst_h as u32,
                        is_animated: false,
                    };
                    // Composite at the calculated destination position
                    scaled_layer_image.composite_onto(canvas, dst_x, dst_y, palette, 0);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to 9-slice scale layer: {}", e);
                }
            }
        }
    }

    fn render_scale(&self, image: &LayerImage, layer: &Layer, canvas: &mut Canvas, palette: &[u8], frame_index: usize) {
        // Scale mode: scale source bounds to dst bounds using nearest-neighbor
        let (dst_x, dst_y, dst_w, dst_h) = self.calculate_dst_rect(layer, canvas, image);

        if dst_w <= 0 || dst_h <= 0 {
            return;
        }

        // Get the source frame
        let src_frame = image.get_frame(frame_index);

        // Scale using nearest-neighbor interpolation
        let mut scaled = RgbaImage::new(dst_w as u32, dst_h as u32);

        let src_w = image.width as i32;
        let src_h = image.height as i32;

        for dy in 0..dst_h {
            for dx in 0..dst_w {
                // Map destination pixel to source pixel (nearest neighbor)
                let sx = (dx * src_w / dst_w) as u32;
                let sy = (dy * src_h / dst_h) as u32;

                if sx < image.width && sy < image.height {
                    let pixel = src_frame.get_pixel(sx, sy);
                    scaled.put_pixel(dx as u32, dy as u32, *pixel);
                }
            }
        }

        // Create a temporary LayerImage with the scaled frame
        let scaled_layer = LayerImage {
            frames: vec![scaled],
            delays: vec![10],
            width: dst_w as u32,
            height: dst_h as u32,
            is_animated: false,
        };

        // Composite the scaled image onto the canvas
        scaled_layer.composite_onto(canvas, dst_x, dst_y, palette, 0);
    }

    fn render_tile(&self, image: &LayerImage, layer: &Layer, canvas: &mut Canvas, palette: &[u8], frame_index: usize) {
        // Tile mode: repeat image across canvas
        let canvas_width = canvas.width() as i32;
        let canvas_height = canvas.height() as i32;

        let mut y = 0;
        while y < canvas_height {
            let mut x = 0;
            while x < canvas_width {
                image.composite_onto(canvas, x, y, palette, frame_index);
                x += image.width as i32;
            }
            y += image.height as i32;
        }
    }

    fn calculate_dst_rect(&self, layer: &Layer, canvas: &Canvas, image: &LayerImage) -> (i32, i32, i32, i32) {
        use crate::theme::BoundValue;

        let canvas_width = canvas.width() as i32;
        let canvas_height = canvas.height() as i32;
        let image_width = image.width as i32;
        let image_height = image.height as i32;

        // Python uses (width-1) as the max coordinate for positioning
        // This matches the Python behavior: total_width = self.width - 1 + padding
        let max_x = canvas_width - 1;
        let max_y = canvas_height - 1;

        // Get dst_bounds or default
        let (left, top, right, bottom) = if let Some(ref bounds) = layer.dst_bounds {
            (
                bounds.left.clone(),
                bounds.top.clone(),
                bounds.right.clone(),
                bounds.bottom.clone(),
            )
        } else {
            (
                BoundValue::Value(0),
                BoundValue::Value(0),
                BoundValue::Auto,
                BoundValue::Auto,
            )
        };

        // Calculate positions (negative values get added to max coordinate)
        // Python: if temp.dst.left <0 : temp.dst.left +=total_width
        let dst_x = match left {
            BoundValue::Auto => 0,
            BoundValue::Value(x) => {
                if x < 0 {
                    max_x + x  // e.g., 1428 + (-110) = 1318
                } else {
                    x
                }
            }
        };

        let dst_y = match top {
            BoundValue::Auto => 0,
            BoundValue::Value(y) => {
                if y < 0 {
                    max_y + y
                } else {
                    y
                }
            }
        };

        let dst_right = match right {
            BoundValue::Auto => max_x,
            BoundValue::Value(r) => {
                if r < 0 {
                    max_x + r
                } else {
                    r
                }
            }
        };

        let dst_bottom = match bottom {
            BoundValue::Auto => max_y,
            BoundValue::Value(b) => {
                if b < 0 {
                    max_y + b
                } else {
                    b
                }
            }
        };

        let dst_w = dst_right - dst_x + 1;
        let dst_h = dst_bottom - dst_y + 1;

        (dst_x, dst_y, dst_w, dst_h)
    }

}
