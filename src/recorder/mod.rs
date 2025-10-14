mod pty;
mod capture;
mod writer;

pub use pty::PtyRecorder;
pub use capture::CaptureSession;
pub use writer::CastWriter;

use anyhow::Result;
use std::path::PathBuf;

/// Configuration for recording sessions
#[derive(Debug, Clone)]
pub struct RecordConfig {
    /// Output file path (.cast, .gif, or .webm)
    pub output: PathBuf,

    /// Command to execute (None = spawn shell)
    pub command: Option<Vec<String>>,

    /// Terminal width in columns
    pub columns: u16,

    /// Terminal height in rows
    pub rows: u16,

    /// Maximum idle time (compress longer pauses to this)
    pub max_idle: Option<f64>,

    /// Enable pause/resume with Ctrl+\
    pub allow_pause: bool,

    /// Show real-time stats
    pub show_stats: bool,

    /// Environment variables to set
    pub env: Vec<(String, String)>,
}

impl Default for RecordConfig {
    fn default() -> Self {
        Self {
            output: PathBuf::from("recording.cast"),
            command: None,
            columns: 80,
            rows: 24,
            max_idle: None,
            allow_pause: true,
            show_stats: false,  // Disabled by default - interferes with terminal display
            env: Vec::new(),
        }
    }
}

/// Main recorder interface
pub struct Recorder {
    config: RecordConfig,
}

impl Recorder {
    pub fn new(config: RecordConfig) -> Self {
        Self { config }
    }

    /// Start recording session
    pub fn record(&self) -> Result<()> {
        eprintln!("Starting recording...");

        if self.config.allow_pause {
            eprintln!("Press Ctrl+\\ to pause/resume");
        }

        eprintln!("Press Ctrl+D or type 'exit' to stop recording");
        eprintln!();

        // Create PTY recorder
        let mut pty = PtyRecorder::new(&self.config)?;

        // Create capture session
        let mut capture = CaptureSession::new(&self.config);

        // Create output writer
        let mut writer = CastWriter::new(&self.config.output)?;

        // Write header
        writer.write_header(self.config.columns, self.config.rows)?;

        // Main recording loop
        pty.run(&mut capture, &mut writer)?;

        // Finalize
        writer.close()?;

        eprintln!("\nRecording saved to: {}", self.config.output.display());

        Ok(())
    }
}
