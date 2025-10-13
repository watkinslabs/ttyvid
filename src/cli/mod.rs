use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ttygif-rust")]
#[command(version = "0.1.0")]
#[command(about = "Convert terminal output to animated GIF", long_about = None)]
pub struct Args {
    /// Input asciicast file (reads from stdin if not provided, supports both .cast format and raw terminal data)
    #[arg(short, long)]
    pub input: Option<PathBuf>,

    /// Output GIF file
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Theme name or path
    #[arg(short, long, default_value = "default")]
    pub theme: String,

    /// Font name
    #[arg(short = 'f', long)]
    pub font: Option<String>,

    /// Frames per second (3-100)
    #[arg(long, default_value = "10")]
    pub fps: u32,

    /// Video quality for WebM (0-100, higher is better, default: 50)
    #[arg(short = 'q', long, default_value = "50")]
    pub quality: u8,

    /// Speed multiplier
    #[arg(long, default_value = "1.0")]
    pub speed: f64,

    /// Terminal width in columns
    #[arg(short = 'c', long)]
    pub columns: Option<usize>,

    /// Terminal height in rows
    #[arg(short = 'r', long)]
    pub rows: Option<usize>,

    /// Number of loops (0 = infinite)
    #[arg(short, long, default_value = "0")]
    pub r#loop: u16,

    /// Delay before loop restart (milliseconds)
    #[arg(short, long, default_value = "100")]
    pub delay: u16,

    /// Remove gaps in recording
    #[arg(short = 'g', long)]
    pub no_gaps: bool,

    /// Add trailer at end
    #[arg(long)]
    pub trailer: bool,

    /// Title text
    #[arg(long)]
    pub title: Option<String>,

    /// Disable auto line wrap
    #[arg(long)]
    pub no_autowrap: bool,

    /// Underlay image path
    #[arg(long)]
    pub underlay: Option<PathBuf>,

    /// Output format (gif, webm with --features webm)
    #[arg(long, value_name = "FORMAT")]
    pub format: Option<String>,
}
