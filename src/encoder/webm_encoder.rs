use anyhow::Result;
use std::fs::File;
use std::io::{BufWriter, Write, Seek};
use std::path::Path;
use rav1e::prelude::*;

use crate::renderer::{Canvas, Palette};

pub struct WebmEncoder {
    writer: BufWriter<File>,
    encoder: rav1e::Context<u8>,
    width: usize,
    height: usize,
    palette_rgb: Vec<[u8; 3]>,
    fps: u32,
    timestamp_scale: u64,
    cluster_timestamp: u64,
    cluster_max_duration: u64, // Maximum duration for a cluster (in ms)
    duration_ms: u64,
    segment_data_start: u64, // Position where segment data starts (for seeking back)
}

impl WebmEncoder {
    pub fn new(path: &Path, width: usize, height: usize, palette: &Palette, fps: u32, quality: u8) -> Result<Self> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        // AV1 requires dimensions to be multiples of 8 for proper alignment
        // Pad dimensions to avoid encoder padding causing offset issues
        let padded_width = ((width + 7) / 8) * 8;
        let padded_height = ((height + 7) / 8) * 8;

        // Quality: 0-100 (higher is better)
        // Map to quantizer: 0-255 (lower is better quality)
        let quantizer = ((100 - quality.min(100)) as usize * 255 / 100).max(20).min(200);
        let min_quantizer = ((quantizer / 2).max(10) as u8).min(200);

        // Map quality to speed preset: 0 (slow/best) to 10 (fast/worst)
        // Quality 0-40: speed 10 (fastest)
        // Quality 40-70: speed 6-8 (medium)
        // Quality 70-100: speed 3-5 (slow/good)
        let speed = if quality < 40 {
            10
        } else if quality < 70 {
            8 - ((quality - 40) / 15)
        } else {
            5 - ((quality - 70) / 15)
        }.min(10).max(0);

        // Create rav1e encoder config for AV1
        let cfg = Config::new()
            .with_encoder_config(EncoderConfig {
                width: padded_width,
                height: padded_height,
                time_base: Rational::new(1, fps as u64),
                speed_settings: SpeedSettings::from_preset(speed),
                quantizer,
                min_quantizer,
                ..Default::default()
            });

        let encoder = cfg.new_context()
            .map_err(|e| anyhow::anyhow!("Failed to create AV1 encoder: {:?}", e))?;

        let palette_rgb = palette.rgb_colors();

        let timestamp_scale = 1_000_000; // 1ms

        // Write WebM header immediately
        let mut temp_encoder = Self {
            writer,
            encoder,
            width: padded_width,
            height: padded_height,
            palette_rgb,
            fps,
            timestamp_scale,
            cluster_timestamp: 0,
            cluster_max_duration: 5000, // 5 seconds per cluster
            duration_ms: 0,
            segment_data_start: 0,
        };

        temp_encoder.write_webm_header()?;
        temp_encoder.start_cluster(0)?;

        Ok(temp_encoder)
    }

    pub fn add_frame(&mut self, canvas: &Canvas, _delay_centiseconds: u16) -> Result<()> {
        let rgb_data = self.canvas_to_rgb(canvas);

        let mut frame = self.encoder.new_frame();

        // Clear frame buffers to black (important if dimensions were padded)
        // Y plane: 16 is black in YUV
        for byte in &mut frame.planes[0].data[..] {
            *byte = 16;
        }
        // U and V planes: 128 is neutral (no color)
        for byte in &mut frame.planes[1].data[..] {
            *byte = 128;
        }
        for byte in &mut frame.planes[2].data[..] {
            *byte = 128;
        }

        self.rgb_to_yuv(&rgb_data, &mut frame);

        self.encoder.send_frame(frame)
            .map_err(|e| anyhow::anyhow!("Failed to send frame to encoder: {:?}", e))?;

        // Collect and write packets immediately
        loop {
            match self.encoder.receive_packet() {
                Ok(packet) => {
                    let pts = (packet.input_frameno as f64 * 1000.0 / self.fps as f64) as u64;
                    let is_key = packet.frame_type == FrameType::KEY;

                    // Update duration
                    self.duration_ms = pts.max(self.duration_ms);

                    // Start new cluster if needed (on keyframe and duration exceeded)
                    if is_key && pts > self.cluster_timestamp + self.cluster_max_duration {
                        self.end_cluster()?;
                        self.start_cluster(pts)?;
                    }

                    // Write frame immediately
                    self.write_simple_block(&packet.data, pts, is_key)?;
                }
                Err(EncoderStatus::Encoded) => break,
                Err(EncoderStatus::LimitReached) => break,
                Err(EncoderStatus::NeedMoreData) => break,
                Err(EncoderStatus::Failure) => {
                    return Err(anyhow::anyhow!("Encoder failure"));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Encoder error: {:?}", e));
                }
            }
        }

        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        // Flush encoder and write remaining packets
        self.encoder.flush();

        loop {
            match self.encoder.receive_packet() {
                Ok(packet) => {
                    let pts = (packet.input_frameno as f64 * 1000.0 / self.fps as f64) as u64;
                    let is_key = packet.frame_type == FrameType::KEY;

                    // Update duration
                    self.duration_ms = pts.max(self.duration_ms);

                    // Write frame immediately
                    self.write_simple_block(&packet.data, pts, is_key)?;
                }
                Err(EncoderStatus::Encoded) => break,
                Err(EncoderStatus::LimitReached) => break,
                Err(e) => {
                    return Err(anyhow::anyhow!("Encoder finish error: {:?}", e));
                }
            }
        }

        // End final cluster
        self.end_cluster()?;

        // Flush buffer
        self.writer.flush()?;

        Ok(())
    }

    fn write_webm_header(&mut self) -> Result<()> {
        // EBML Header
        self.write_ebml_element(0x1A45DFA3, &{
            let mut data = Vec::new();
            self.write_ebml_uint_to_vec(&mut data, 0x4286, 1)?; // EBMLVersion
            self.write_ebml_uint_to_vec(&mut data, 0x42F7, 1)?; // EBMLReadVersion
            self.write_ebml_uint_to_vec(&mut data, 0x42F2, 4)?; // EBMLMaxIDLength
            self.write_ebml_uint_to_vec(&mut data, 0x42F3, 8)?; // EBMLMaxSizeLength
            self.write_ebml_string_to_vec(&mut data, 0x4282, "webm")?; // DocType
            self.write_ebml_uint_to_vec(&mut data, 0x4287, 2)?; // DocTypeVersion
            self.write_ebml_uint_to_vec(&mut data, 0x4285, 2)?; // DocTypeReadVersion
            data
        })?;

        // Segment - with unknown size for streaming
        self.write_element_id(0x18538067)?; // Segment ID
        self.write_size_unknown()?; // Unknown size (streaming)

        self.segment_data_start = self.writer.stream_position()?;

        // Info
        self.write_ebml_element(0x1549A966, &{
            let mut data = Vec::new();
            self.write_ebml_uint_to_vec(&mut data, 0x2AD7B1, self.timestamp_scale)?; // TimestampScale
            self.write_ebml_string_to_vec(&mut data, 0x4D80, "ttygif-rust")?; // MuxingApp
            self.write_ebml_string_to_vec(&mut data, 0x5741, "ttygif-rust")?; // WritingApp
            // Duration will be written at the end if we can seek
            data
        })?;

        // Tracks
        self.write_ebml_element(0x1654AE6B, &{
            let mut data = Vec::new();
            // TrackEntry
            self.write_ebml_element_to_vec(&mut data, 0xAE, &{
                let mut track_data = Vec::new();
                self.write_ebml_uint_to_vec(&mut track_data, 0xD7, 1)?; // TrackNumber
                self.write_ebml_uint_to_vec(&mut track_data, 0x73C5, 1)?; // TrackUID
                self.write_ebml_uint_to_vec(&mut track_data, 0x83, 1)?; // TrackType (video)
                self.write_ebml_string_to_vec(&mut track_data, 0x86, "V_AV1")?; // CodecID

                // Video settings
                self.write_ebml_element_to_vec(&mut track_data, 0xE0, &{
                    let mut video_data = Vec::new();
                    self.write_ebml_uint_to_vec(&mut video_data, 0xB0, self.width as u64)?; // PixelWidth
                    self.write_ebml_uint_to_vec(&mut video_data, 0xBA, self.height as u64)?; // PixelHeight
                    video_data
                })?;

                track_data
            })?;
            data
        })?;

        Ok(())
    }

    fn start_cluster(&mut self, timestamp: u64) -> Result<()> {
        self.cluster_timestamp = timestamp;

        // Write Cluster element with unknown size (streaming)
        self.write_element_id(0x1F43B675)?; // Cluster ID
        self.write_size_unknown()?; // Unknown size

        // Write cluster timestamp
        let mut ts_data = Vec::new();
        self.write_ebml_uint_to_vec(&mut ts_data, 0xE7, timestamp)?; // Timestamp
        self.writer.write_all(&ts_data)?;

        Ok(())
    }

    fn end_cluster(&mut self) -> Result<()> {
        // For streaming mode with unknown sizes, we don't need to do anything special
        // The next cluster or end of file naturally terminates this cluster
        Ok(())
    }

    fn write_simple_block(&mut self, data: &[u8], timestamp: u64, is_keyframe: bool) -> Result<()> {
        let mut block_data = Vec::new();

        // Track number (varint encoded)
        block_data.push(0x81); // Track 1

        // Timestamp relative to cluster
        let relative_ts = (timestamp - self.cluster_timestamp) as i16;
        block_data.extend_from_slice(&relative_ts.to_be_bytes());

        // Flags (keyframe = 0x80, no lacing = 0x00)
        block_data.push(if is_keyframe { 0x80 } else { 0x00 });

        // Frame data
        block_data.extend_from_slice(data);

        // Write SimpleBlock element
        self.write_ebml_element(0xA3, &block_data)?;

        Ok(())
    }


    fn write_ebml_element(&mut self, id: u64, data: &[u8]) -> Result<()> {
        self.write_element_id(id)?;
        self.write_size(data.len() as u64)?;
        self.writer.write_all(data)?;
        Ok(())
    }

    fn write_ebml_element_to_vec(&self, buf: &mut Vec<u8>, id: u64, data: &[u8]) -> Result<()> {
        self.write_element_id_to_vec(buf, id);
        self.write_size_to_vec(buf, data.len() as u64);
        buf.extend_from_slice(data);
        Ok(())
    }

    fn write_ebml_uint_to_vec(&self, buf: &mut Vec<u8>, id: u64, value: u64) -> Result<()> {
        let bytes = if value == 0 {
            vec![0]
        } else {
            let bytes_needed = (64 - value.leading_zeros() + 7) / 8;
            value.to_be_bytes()[(8 - bytes_needed as usize)..].to_vec()
        };
        self.write_ebml_element_to_vec(buf, id, &bytes)
    }

    fn write_ebml_float_to_vec(&self, buf: &mut Vec<u8>, id: u64, value: f64) -> Result<()> {
        self.write_ebml_element_to_vec(buf, id, &value.to_be_bytes())
    }

    fn write_ebml_string_to_vec(&self, buf: &mut Vec<u8>, id: u64, s: &str) -> Result<()> {
        self.write_ebml_element_to_vec(buf, id, s.as_bytes())
    }

    fn write_element_id(&mut self, id: u64) -> Result<()> {
        let bytes = if id <= 0xFF {
            vec![(id & 0xFF) as u8]
        } else if id <= 0xFFFF {
            vec![((id >> 8) & 0xFF) as u8, (id & 0xFF) as u8]
        } else if id <= 0xFFFFFF {
            vec![((id >> 16) & 0xFF) as u8, ((id >> 8) & 0xFF) as u8, (id & 0xFF) as u8]
        } else {
            vec![
                ((id >> 24) & 0xFF) as u8,
                ((id >> 16) & 0xFF) as u8,
                ((id >> 8) & 0xFF) as u8,
                (id & 0xFF) as u8,
            ]
        };
        self.writer.write_all(&bytes)?;
        Ok(())
    }

    fn write_element_id_to_vec(&self, buf: &mut Vec<u8>, id: u64) {
        if id <= 0xFF {
            buf.push((id & 0xFF) as u8);
        } else if id <= 0xFFFF {
            buf.push(((id >> 8) & 0xFF) as u8);
            buf.push((id & 0xFF) as u8);
        } else if id <= 0xFFFFFF {
            buf.push(((id >> 16) & 0xFF) as u8);
            buf.push(((id >> 8) & 0xFF) as u8);
            buf.push((id & 0xFF) as u8);
        } else {
            buf.push(((id >> 24) & 0xFF) as u8);
            buf.push(((id >> 16) & 0xFF) as u8);
            buf.push(((id >> 8) & 0xFF) as u8);
            buf.push((id & 0xFF) as u8);
        }
    }

    fn write_size(&mut self, size: u64) -> Result<()> {
        // EBML variable-size integer
        let bytes = if size < 0x7F {
            vec![(size | 0x80) as u8]
        } else if size < 0x3FFF {
            vec![
                ((size >> 8) | 0x40) as u8,
                (size & 0xFF) as u8,
            ]
        } else if size < 0x1FFFFF {
            vec![
                ((size >> 16) | 0x20) as u8,
                ((size >> 8) & 0xFF) as u8,
                (size & 0xFF) as u8,
            ]
        } else if size < 0xFFFFFFF {
            vec![
                ((size >> 24) | 0x10) as u8,
                ((size >> 16) & 0xFF) as u8,
                ((size >> 8) & 0xFF) as u8,
                (size & 0xFF) as u8,
            ]
        } else {
            vec![
                0x01,
                ((size >> 32) & 0xFF) as u8,
                ((size >> 24) & 0xFF) as u8,
                ((size >> 16) & 0xFF) as u8,
                ((size >> 8) & 0xFF) as u8,
                (size & 0xFF) as u8,
            ]
        };
        self.writer.write_all(&bytes)?;
        Ok(())
    }

    fn write_size_to_vec(&self, buf: &mut Vec<u8>, size: u64) {
        if size < 0x7F {
            buf.push((size | 0x80) as u8);
        } else if size < 0x3FFF {
            buf.push(((size >> 8) | 0x40) as u8);
            buf.push((size & 0xFF) as u8);
        } else if size < 0x1FFFFF {
            buf.push(((size >> 16) | 0x20) as u8);
            buf.push(((size >> 8) & 0xFF) as u8);
            buf.push((size & 0xFF) as u8);
        } else if size < 0xFFFFFFF {
            buf.push(((size >> 24) | 0x10) as u8);
            buf.push(((size >> 16) & 0xFF) as u8);
            buf.push(((size >> 8) & 0xFF) as u8);
            buf.push((size & 0xFF) as u8);
        } else {
            buf.push(0x01);
            buf.push(((size >> 32) & 0xFF) as u8);
            buf.push(((size >> 24) & 0xFF) as u8);
            buf.push(((size >> 16) & 0xFF) as u8);
            buf.push(((size >> 8) & 0xFF) as u8);
            buf.push((size & 0xFF) as u8);
        }
    }

    fn write_size_unknown(&mut self) -> Result<()> {
        // Unknown size marker (all 1s)
        self.writer.write_all(&[0xFF])?;
        Ok(())
    }

    fn canvas_to_rgb(&self, canvas: &Canvas) -> Vec<u8> {
        let data = canvas.data();
        let canvas_width = canvas.width();
        let canvas_height = canvas.height();

        // Create padded RGB data
        let mut rgb_data = Vec::with_capacity(self.width * self.height * 3);

        for y in 0..self.height {
            for x in 0..self.width {
                if y < canvas_height && x < canvas_width {
                    // Copy from canvas
                    let palette_index = data[y * canvas_width + x];
                    let rgb = self.palette_rgb[palette_index as usize];
                    rgb_data.push(rgb[0]);
                    rgb_data.push(rgb[1]);
                    rgb_data.push(rgb[2]);
                } else {
                    // Pad with black
                    rgb_data.push(0);
                    rgb_data.push(0);
                    rgb_data.push(0);
                }
            }
        }

        rgb_data
    }

    fn rgb_to_yuv(&self, rgb: &[u8], frame: &mut Frame<u8>) {
        let y_stride = frame.planes[0].cfg.stride;
        let u_stride = frame.planes[1].cfg.stride;
        let v_stride = frame.planes[2].cfg.stride;

        // Get origin offsets - these are CRITICAL!
        let y_xorigin = frame.planes[0].cfg.xorigin;
        let y_yorigin = frame.planes[0].cfg.yorigin;
        let u_xorigin = frame.planes[1].cfg.xorigin;
        let u_yorigin = frame.planes[1].cfg.yorigin;
        let v_xorigin = frame.planes[2].cfg.xorigin;
        let v_yorigin = frame.planes[2].cfg.yorigin;

        // Copy row by row to handle stride properly WITH origin offset
        for y in 0..self.height {
            let rgb_row_start = y * self.width * 3;
            let y_row_start = (y + y_yorigin) * y_stride + y_xorigin;

            for x in 0..self.width {
                let rgb_idx = rgb_row_start + x * 3;
                let r = rgb[rgb_idx] as i32;
                let g = rgb[rgb_idx + 1] as i32;
                let b = rgb[rgb_idx + 2] as i32;

                // RGB to YUV conversion (ITU-R BT.601)
                let y_val = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
                frame.planes[0].data[y_row_start + x] = y_val.clamp(0, 255) as u8;
            }
        }

        // Process UV planes separately with 2x2 subsampling WITH origin offset
        for y in (0..self.height).step_by(2) {
            let uv_y = y / 2;
            let uv_row_start_u = (uv_y + u_yorigin) * u_stride + u_xorigin;
            let uv_row_start_v = (uv_y + v_yorigin) * v_stride + v_xorigin;
            let rgb_row_start = y * self.width * 3;

            for x in (0..self.width).step_by(2) {
                let uv_x = x / 2;
                let rgb_idx = rgb_row_start + x * 3;

                let r = rgb[rgb_idx] as i32;
                let g = rgb[rgb_idx + 1] as i32;
                let b = rgb[rgb_idx + 2] as i32;

                let u_val = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                let v_val = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;

                frame.planes[1].data[uv_row_start_u + uv_x] = u_val.clamp(0, 255) as u8;
                frame.planes[2].data[uv_row_start_v + uv_x] = v_val.clamp(0, 255) as u8;
            }
        }
    }
}
