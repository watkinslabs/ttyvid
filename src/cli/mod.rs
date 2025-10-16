use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ttyvid")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Record and convert terminal sessions to video", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    // Legacy mode: if no subcommand, behave like convert
    /// Input asciicast file (reads from stdin if not provided)
    #[arg(short, long, global = true)]
    pub input: Option<PathBuf>,

    /// Output file
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,

    /// Theme name or path
    #[arg(short, long, global = true, default_value = "default")]
    pub theme: String,

    /// Font name (bitmap font)
    #[arg(short = 'f', long, global = true)]
    pub font: Option<String>,

    /// System font name or path (TrueType/OpenType)
    /// Use "monospace", "default", or "system" for system default monospace font
    #[arg(long, global = true)]
    pub system_font: Option<String>,

    /// Font size in pixels for TrueType fonts (height of the character cell)
    #[arg(long, global = true, default_value = "16")]
    pub font_size: usize,

    /// Frames per second (3-100)
    #[arg(long, global = true, default_value = "10")]
    pub fps: u32,

    /// Video quality for WebM (0-100, higher is better)
    #[arg(short = 'q', long, global = true, default_value = "50")]
    pub quality: u8,

    /// Speed multiplier
    #[arg(long, global = true, default_value = "1.0")]
    pub speed: f64,

    /// Terminal width in columns
    #[arg(short = 'c', long, global = true)]
    pub columns: Option<usize>,

    /// Terminal height in rows
    #[arg(short = 'r', long, global = true)]
    pub rows: Option<usize>,

    /// Number of loops (0 = infinite)
    #[arg(short, long, global = true, default_value = "0")]
    pub r#loop: u16,

    /// Delay before loop restart (milliseconds)
    #[arg(short, long, global = true, default_value = "100")]
    pub delay: u16,

    /// Remove gaps in recording
    #[arg(short = 'g', long, global = true)]
    pub no_gaps: bool,

    /// Add trailer at end (1.5s pause before loop)
    #[arg(long, global = true)]
    pub trailer: bool,

    /// Title text
    #[arg(long, global = true)]
    pub title: Option<String>,

    /// Disable auto line wrap
    #[arg(long, global = true)]
    pub no_autowrap: bool,

    /// Hide cursor in output
    #[arg(long, global = true)]
    pub no_cursor: bool,

    /// Use terminal's default color palette instead of theme palette
    #[arg(long, global = true)]
    pub terminal_colors: bool,

    /// Use current terminal size (overrides --columns and --rows)
    #[arg(long, global = true)]
    pub terminal_size: bool,

    /// Clone terminal colors and size (enables --terminal-colors and --terminal-size)
    #[arg(long, global = true)]
    pub clone: bool,

    /// Underlay image path
    #[arg(long, global = true)]
    pub underlay: Option<PathBuf>,

    /// Output format (optional, auto-detected from extension)
    #[arg(long, global = true, value_name = "FORMAT")]
    pub format: Option<String>,

    /// Generate multiple formats (comma-separated: cast,gif,webm,md)
    #[arg(long, global = true, value_delimiter = ',')]
    pub formats: Vec<String>,

    /// Start MCP (Model Context Protocol) server
    #[arg(long)]
    pub mcp: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Record a terminal session
    Record {
        /// Output file (.cast, .gif, or .webm)
        #[arg(short, long)]
        output: PathBuf,

        /// Command to execute (if not provided, spawns shell)
        #[arg(last = true)]
        command: Vec<String>,

        /// Maximum idle time in seconds (compress longer pauses)
        #[arg(long)]
        max_idle: Option<f64>,

        /// Disable pause/resume with Ctrl+\
        #[arg(long)]
        no_pause: bool,

        /// Show real-time stats display during recording
        #[arg(long)]
        stats: bool,

        /// Verbose output (show detailed messages)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Convert .cast file to video
    Convert {
        /// Input asciicast file
        #[arg(short, long)]
        input: PathBuf,

        /// Output file (.gif or .webm)
        #[arg(short, long)]
        output: PathBuf,
    },

    /// List available fonts
    ListFonts {
        /// Show system TrueType fonts
        #[arg(long)]
        system: bool,

        /// Show embedded bitmap fonts
        #[arg(long)]
        bitmap: bool,
    },
}
