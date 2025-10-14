use anyhow::Result;
use gif::{Encoder, Frame, Repeat};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::renderer::{Canvas, Palette};

pub struct GifEncoder {
    encoder: Encoder<BufWriter<File>>,
    width: u16,
    height: u16,
    previous_frame: Option<Vec<u8>>,
    transparent_index: Option<u8>,
    global_palette: Vec<u8>, // Store global palette RGB values
}

impl GifEncoder {
    pub fn new(path: &Path, width: usize, height: usize, palette: &Palette, loop_count: u16, transparent_index: Option<u8>) -> Result<Self> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let global_palette = palette.colors().to_vec();

        let mut encoder = Encoder::new(
            writer,
            width as u16,
            height as u16,
            &global_palette,
        )?;

        // Set loop count (0 = infinite)
        encoder.set_repeat(if loop_count == 0 {
            Repeat::Infinite
        } else {
            Repeat::Finite(loop_count)
        })?;

        Ok(Self {
            encoder,
            width: width as u16,
            height: height as u16,
            previous_frame: None,
            transparent_index,
            global_palette,
        })
    }

    fn create_local_palette(&self, frame_data: &[u8]) -> (Vec<u8>, Vec<u8>) {
        // Collect unique color indices used in this frame
        let mut color_set = std::collections::HashSet::new();
        for &idx in frame_data {
            color_set.insert(idx);
        }

        // Sort for consistent output
        let mut unique_colors: Vec<u8> = color_set.into_iter().collect();
        unique_colors.sort_unstable();

        // Create mapping from global index to local index
        let mut index_map = vec![0u8; 256];
        for (local_idx, &global_idx) in unique_colors.iter().enumerate() {
            index_map[global_idx as usize] = local_idx as u8;
        }

        // Build local palette (RGB values from global palette)
        let mut local_palette = Vec::with_capacity(unique_colors.len() * 3);
        for &global_idx in &unique_colors {
            let rgb_idx = (global_idx as usize) * 3;
            local_palette.extend_from_slice(&self.global_palette[rgb_idx..rgb_idx + 3]);
        }

        // Remap frame data to local indices
        let remapped_data: Vec<u8> = frame_data.iter()
            .map(|&idx| index_map[idx as usize])
            .collect();

        (local_palette, remapped_data)
    }

    pub fn add_frame(&mut self, canvas: &Canvas, delay_centiseconds: u16) -> Result<()> {
        let data = canvas.data();

        let (left, top, width, height, frame_data) = if let Some(ref prev) = self.previous_frame {
            // Compute diff - only encode changed region
            self.compute_diff(prev, data)
        } else {
            // First frame - encode everything
            (0, 0, self.width, self.height, data.to_vec())
        };

        // Save current frame for next diff
        self.previous_frame = Some(data.to_vec());

        // Create local palette with only colors used in this frame
        let (local_palette, remapped_data) = self.create_local_palette(&frame_data);

        // Create frame with remapped data
        let mut frame = Frame::from_indexed_pixels(
            width,
            height,
            remapped_data,
            self.transparent_index,
        );

        frame.delay = delay_centiseconds;
        frame.left = left;
        frame.top = top;

        // Set local palette on the frame
        frame.palette = Some(local_palette);

        // Use Keep disposal - previous frame content remains where not overwritten
        frame.dispose = gif::DisposalMethod::Keep;

        self.encoder.write_frame(&frame)?;

        Ok(())
    }

    pub fn finish(self) -> Result<()> {
        // Encoder will be dropped and flushed automatically
        Ok(())
    }

    fn compute_diff(&self, prev: &[u8], curr: &[u8]) -> (u16, u16, u16, u16, Vec<u8>) {
        // Find bounding box of changes
        let width = self.width as usize;
        let height = self.height as usize;

        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0;
        let mut max_y = 0;

        let mut has_changes = false;

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                if prev[idx] != curr[idx] {
                    has_changes = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }

        if !has_changes {
            // No changes - return a tiny 1x1 frame
            return (0, 0, 1, 1, vec![curr[0]]);
        }

        // Extract the diff region
        let diff_width = max_x - min_x + 1;
        let diff_height = max_y - min_y + 1;
        let mut frame_data = Vec::with_capacity(diff_width * diff_height);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                frame_data.push(curr[y * width + x]);
            }
        }

        (
            min_x as u16,
            min_y as u16,
            diff_width as u16,
            diff_height as u16,
            frame_data,
        )
    }
}
