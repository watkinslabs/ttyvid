use anyhow::{Context, Result};
use serde_json::json;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::capture::{CaptureSession, OutputEvent};

/// Writes asciicast v2 format (.cast files)
pub struct CastWriter {
    writer: BufWriter<File>,
    header_written: bool,
}

impl CastWriter {
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::create(path)
            .with_context(|| format!("Failed to create output file: {}", path.display()))?;

        Ok(Self {
            writer: BufWriter::new(file),
            header_written: false,
        })
    }

    /// Write asciicast v2 header
    pub fn write_header(&mut self, width: u16, height: u16) -> Result<()> {
        if self.header_written {
            return Ok(());
        }

        // Get environment info
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());
        let term = std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string());

        // Build header (asciicast v2 format)
        let header = json!({
            "version": 2,
            "width": width,
            "height": height,
            "timestamp": CaptureSession::get_start_timestamp(),
            "env": {
                "SHELL": shell,
                "TERM": term,
            },
            "title": "ttyvid recording",
        });

        // Write header line
        writeln!(self.writer, "{}", header)?;
        self.header_written = true;

        Ok(())
    }

    /// Write an output event
    pub fn write_event(&mut self, event: &OutputEvent) -> Result<()> {
        if !self.header_written {
            anyhow::bail!("Cannot write event before header");
        }

        if event.data.is_empty() {
            return Ok(());
        }

        // Convert bytes to string (might contain invalid UTF-8, that's okay)
        let data_str = String::from_utf8_lossy(&event.data);

        // Write event line: [time, "o", data]
        let event_json = json!([event.timestamp, "o", data_str]);
        writeln!(self.writer, "{}", event_json)?;

        Ok(())
    }

    /// Flush and close the writer
    pub fn close(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}
