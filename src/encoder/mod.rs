mod gif_encoder;
#[cfg(feature = "webm")]
mod webm_encoder;

pub use gif_encoder::GifEncoder;
#[cfg(feature = "webm")]
pub use webm_encoder::WebmEncoder;

use anyhow::Result;
use std::path::Path;
use crate::renderer::{Canvas, Palette};

/// Trait for animated encoders
pub trait AnimatedEncoder {
    fn add_frame(&mut self, canvas: &Canvas, delay_centiseconds: u16) -> Result<()>;
    fn finish(self) -> Result<()>;
}

/// Output format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Gif,
    #[cfg(feature = "webm")]
    Webm,
}

impl OutputFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "gif" => Some(OutputFormat::Gif),
                #[cfg(feature = "webm")]
                "webm" | "ivf" => Some(OutputFormat::Webm),
                _ => None,
            })
    }

    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Gif => "gif",
            #[cfg(feature = "webm")]
            OutputFormat::Webm => "webm",
        }
    }
}

/// Wrapper enum for different encoder types
pub enum EncoderWrapper {
    Gif(GifEncoder),
    #[cfg(feature = "webm")]
    Webm(WebmEncoder),
}

impl EncoderWrapper {
    pub fn new(
        path: &Path,
        width: usize,
        height: usize,
        palette: &Palette,
        format: OutputFormat,
        loop_count: u16,
        fps: u32,
        quality: u8,
    ) -> Result<Self> {
        match format {
            OutputFormat::Gif => {
                Ok(EncoderWrapper::Gif(GifEncoder::new(path, width, height, palette, loop_count)?))
            }
            #[cfg(feature = "webm")]
            OutputFormat::Webm => {
                Ok(EncoderWrapper::Webm(WebmEncoder::new(path, width, height, palette, fps, quality)?))
            }
        }
    }

    pub fn add_frame(&mut self, canvas: &Canvas, delay_centiseconds: u16) -> Result<()> {
        match self {
            EncoderWrapper::Gif(encoder) => encoder.add_frame(canvas, delay_centiseconds),
            #[cfg(feature = "webm")]
            EncoderWrapper::Webm(encoder) => encoder.add_frame(canvas, delay_centiseconds),
        }
    }

    pub fn finish(self) -> Result<()> {
        match self {
            EncoderWrapper::Gif(encoder) => encoder.finish(),
            #[cfg(feature = "webm")]
            EncoderWrapper::Webm(encoder) => encoder.finish(),
        }
    }
}
