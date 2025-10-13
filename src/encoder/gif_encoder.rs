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
}

impl GifEncoder {
    pub fn new(path: &Path, width: usize, height: usize, palette: &Palette, loop_count: u16) -> Result<Self> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let mut encoder = Encoder::new(
            writer,
            width as u16,
            height as u16,
            palette.colors(),
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
        })
    }

    pub fn add_frame(&mut self, canvas: &Canvas, delay_centiseconds: u16) -> Result<()> {
        let data = canvas.data();

        // Compute difference region if we have a previous frame
        let (left, top, width, height, frame_data) = if let Some(prev) = &self.previous_frame {
            self.compute_diff(prev, data)
        } else {
            // First frame - use entire canvas
            (0, 0, self.width, self.height, data.to_vec())
        };

        // Create and write frame
        let mut frame = Frame::from_indexed_pixels(
            width,
            height,
            frame_data,
            None, // No local palette
        );

        frame.delay = delay_centiseconds;
        frame.left = left;
        frame.top = top;

        // Set disposal method to keep previous frame
        frame.dispose = gif::DisposalMethod::Keep;

        self.encoder.write_frame(&frame)?;

        // Store current frame for next diff
        self.previous_frame = Some(data.to_vec());

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
