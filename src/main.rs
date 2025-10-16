use clap::Parser;
use anyhow::Result;
use std::path::PathBuf;

mod cli;
mod input;
mod terminal;
mod renderer;
mod encoder;
mod theme;
mod assets;
mod recorder;
mod mcp_server;

use input::{InputSource, AsciicastReader, StdinReader};
use terminal::TerminalEmulator;
use renderer::{Palette, Canvas, Font, query_terminal_font, RenderBackend};
#[cfg(feature = "gpu")]
use renderer::GpuRenderer;
#[cfg(not(feature = "gpu"))]
use renderer::Rasterizer;
use encoder::{EncoderWrapper, OutputFormat};
use theme::Theme;
use theme::layers::{LayerRenderer, LayerImage};

/// Query terminal default color (OSC 10 for fg, OSC 11 for bg)
fn query_default_terminal_color(osc_number: u8) -> Option<u8> {
    use std::io::{Write, Read};
    use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
    use std::time::Duration;
    use std::sync::mpsc::channel;
    use std::thread;

    let query = format!("\x1b]{};?\x1b\\", osc_number);

    if enable_raw_mode().is_err() {
        return None;
    }

    let mut stderr = std::io::stderr();
    if stderr.write_all(query.as_bytes()).is_err() {
        let _ = disable_raw_mode();
        return None;
    }
    if stderr.flush().is_err() {
        let _ = disable_raw_mode();
        return None;
    }

    // Read response with timeout
    let (tx, rx) = channel();
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buffer = Vec::new();
        let mut temp = [0u8; 1];

        loop {
            if stdin.read_exact(&mut temp).is_ok() {
                buffer.push(temp[0]);
                if buffer.len() >= 2 && buffer[buffer.len() - 2] == 0x1b && buffer[buffer.len() - 1] == b'\\' {
                    break;
                }
                if temp[0] == 0x07 {
                    break;
                }
                if buffer.len() > 1024 {
                    break;
                }
            } else {
                break;
            }
        }
        let _ = tx.send(String::from_utf8_lossy(&buffer).to_string());
    });

    let response = rx.recv_timeout(Duration::from_millis(100)).ok();
    let _ = disable_raw_mode();

    // Parse rgb:RRRR/GGGG/BBBB
    if let Some(resp) = response {
        if let Some(rgb_start) = resp.find("rgb:") {
            let rgb_part = &resp[rgb_start + 4..];
            let parts: Vec<&str> = rgb_part.split('/').collect();
            if parts.len() >= 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u16::from_str_radix(parts[0].trim_end_matches(|c: char| !c.is_ascii_hexdigit()), 16),
                    u16::from_str_radix(parts[1].trim_end_matches(|c: char| !c.is_ascii_hexdigit()), 16),
                    u16::from_str_radix(parts[2].trim_end_matches(|c: char| !c.is_ascii_hexdigit()), 16),
                ) {
                    // Convert to 8-bit and match to standard palette
                    let r8 = (r >> 8) as u8;
                    let g8 = (g >> 8) as u8;
                    let b8 = (b >> 8) as u8;

                    // Simple matching to closest basic 16 color
                    let palette = Palette::default();
                    return Some(palette.match_color_index(r8 as i32, g8 as i32, b8 as i32));
                }
            }
        }
    }

    None
}

/// Find layer file in theme directories
fn find_layer_file(layer_file: &str) -> PathBuf {
    // Try as absolute path first
    let absolute_path = PathBuf::from(layer_file);
    if absolute_path.exists() {
        return absolute_path;
    }

    // Search in theme directories
    let mut search_paths = vec![
        PathBuf::from("themes"),
    ];

    // Add Unix-specific system paths
    #[cfg(not(windows))]
    {
        search_paths.push(PathBuf::from("/usr/share/ttyvid/themes"));
        search_paths.push(PathBuf::from("/usr/local/share/ttyvid/themes"));
    }

    // Add user directories
    if let Some(proj_dirs) = directories::ProjectDirs::from("", "", "ttyvid") {
        for base_path in [proj_dirs.data_dir(), proj_dirs.config_dir()] {
            for search_path in &search_paths {
                let layer_path = base_path.join("themes").join(layer_file);
                if layer_path.exists() {
                    return layer_path;
                }
            }
        }
    }

    // Search in system and current directory
    for base_path in &search_paths {
        let layer_path = base_path.join(layer_file);
        if layer_path.exists() {
            return layer_path;
        }
    }

    // Fall back to relative path (will fail later if not found)
    PathBuf::from("themes").join(layer_file)
}

fn main() -> Result<()> {
    let args = cli::Args::parse();

    // Check for MCP server mode first
    if args.mcp {
        // Start MCP server using tokio runtime
        let runtime = tokio::runtime::Runtime::new()?;
        return runtime.block_on(async {
            mcp_server::start_mcp_server().await
        });
    }

    println!("ttyvid version {}\n", env!("CARGO_PKG_VERSION"));

    // Handle subcommands or legacy mode
    match args.command {
        Some(cli::Command::Record { ref output, ref command, max_idle, no_pause, stats, verbose }) => {
            // Determine output formats
            let output_formats = if !args.formats.is_empty() {
                // Use --formats flag
                args.formats.clone()
            } else {
                // Auto-detect from extension
                let output_ext = output
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("cast");
                vec![output_ext.to_string()]
            };

            // Get base output path (strip extension if present)
            let base_output = output.with_extension("");

            // Always record to .cast file first
            let cast_file = base_output.with_extension("cast");

            // Determine recording dimensions
            let (rec_cols, rec_rows) = if args.clone || args.terminal_size {
                use crossterm::terminal;
                if let Ok((cols, rows)) = terminal::size() {
                    eprintln!("Using terminal size: {}x{}", cols, rows);
                    (cols, rows)
                } else {
                    eprintln!("Warning: Could not query terminal size, using defaults");
                    (args.columns.unwrap_or(80) as u16, args.rows.unwrap_or(24) as u16)
                }
            } else {
                (args.columns.unwrap_or(80) as u16, args.rows.unwrap_or(24) as u16)
            };

            // Configure and run recorder
            let config = recorder::RecordConfig {
                output: cast_file.clone(),
                command: if command.is_empty() { None } else { Some(command.clone()) },
                columns: rec_cols,
                rows: rec_rows,
                max_idle,
                allow_pause: !no_pause,
                show_stats: stats,  // Disabled by default, enable with --stats
                verbose,
                env: Vec::new(),
            };

            let recorder = recorder::Recorder::new(config);
            recorder.record()?;

            // Generate all requested formats
            let keep_cast = output_formats.contains(&"cast".to_string());
            let mut generated_files = vec![];

            for format in &output_formats {
                match format.to_lowercase().as_str() {
                    "cast" => {
                        // Already generated
                        generated_files.push(cast_file.clone());
                    }
                    "gif" => {
                        let gif_file = base_output.with_extension("gif");
                        eprintln!("\nConverting to GIF...");
                        eprintln!("This may take a moment depending on recording length and frame rate.\n");
                        convert_recording(&args, Some(cast_file.clone()), Some(gif_file.clone()))?;
                        generated_files.push(gif_file);
                    }
                    #[cfg(feature = "webm")]
                    "webm" => {
                        let webm_file = base_output.with_extension("webm");
                        eprintln!("\nConverting to WebM...");
                        eprintln!("This may take a moment depending on recording length and frame rate.\n");
                        convert_recording(&args, Some(cast_file.clone()), Some(webm_file.clone()))?;
                        generated_files.push(webm_file);
                    }
                    "md" | "markdown" => {
                        // Generate markdown file with embedded GIF/WebM
                        let md_file = base_output.with_extension("md");
                        generate_markdown(&base_output, &output_formats, &md_file)?;
                        generated_files.push(md_file);
                    }
                    _ => {
                        eprintln!("Warning: Unknown format '{}', skipping", format);
                    }
                }
            }

            // Clean up temporary .cast file if not requested
            if !keep_cast && cast_file.exists() {
                if let Err(e) = std::fs::remove_file(&cast_file) {
                    eprintln!("Warning: Failed to remove temporary file {}: {}", cast_file.display(), e);
                }
            }

            // Show summary
            eprintln!("\nGenerated files:");
            for file in &generated_files {
                eprintln!("  ✓ {}", file.display());
            }
        }
        Some(cli::Command::Convert { ref input, ref output }) => {
            if !args.formats.is_empty() {
                // Multiple formats requested
                let base_output = output.with_extension("");
                let mut generated_files = vec![];

                for format in &args.formats {
                    match format.to_lowercase().as_str() {
                        "cast" => {
                            // Copy the input file
                            let cast_file = base_output.with_extension("cast");
                            std::fs::copy(input, &cast_file)?;
                            generated_files.push(cast_file);
                        }
                        "gif" => {
                            let gif_file = base_output.with_extension("gif");
                            eprintln!("\nConverting to GIF...");
                            convert_recording(&args, Some(input.clone()), Some(gif_file.clone()))?;
                            generated_files.push(gif_file);
                        }
                        #[cfg(feature = "webm")]
                        "webm" => {
                            let webm_file = base_output.with_extension("webm");
                            eprintln!("\nConverting to WebM...");
                            convert_recording(&args, Some(input.clone()), Some(webm_file.clone()))?;
                            generated_files.push(webm_file);
                        }
                        "md" | "markdown" => {
                            let md_file = base_output.with_extension("md");
                            generate_markdown(&base_output, &args.formats, &md_file)?;
                            generated_files.push(md_file);
                        }
                        _ => {
                            eprintln!("Warning: Unknown format '{}', skipping", format);
                        }
                    }
                }

                eprintln!("\nGenerated files:");
                for file in &generated_files {
                    eprintln!("  ✓ {}", file.display());
                }
            } else {
                // Single format (legacy behavior)
                convert_recording(&args, Some(input.clone()), Some(output.clone()))?;
            }
        }
        Some(cli::Command::ListFonts { system, bitmap }) => {
            // If neither flag specified, show both
            let show_system = system || (!system && !bitmap);
            let show_bitmap = bitmap || (!system && !bitmap);

            if show_system {
                println!("System TrueType Fonts:");
                println!("====================\n");
                match renderer::TrueTypeFont::list_system_fonts() {
                    Ok(fonts) => {
                        if fonts.is_empty() {
                            println!("  No system fonts found");
                        } else {
                            for (i, font) in fonts.iter().enumerate() {
                                println!("  {:3}. {}", i + 1, font);
                            }
                            println!("\n  Total: {} fonts", fonts.len());
                        }
                    }
                    Err(e) => {
                        eprintln!("Error listing system fonts: {}", e);
                    }
                }

                println!("\nUsage: ttyvid convert -i input.cast -o output.gif --system-font \"Font Name\"");
                println!("       ttyvid convert -i input.cast -o output.gif --system-font /path/to/font.ttf");
                println!("       ttyvid convert -i input.cast -o output.gif --system-font monospace  (system default)");
                println!();
            }

            if show_bitmap {
                if show_system {
                    println!();
                }
                println!("Embedded Bitmap Fonts:");
                println!("======================\n");
                let font_names = renderer::Font::available_fonts();
                for (i, font) in font_names.iter().enumerate() {
                    println!("  {:3}. {}", i + 1, font);
                }
                println!("\n  Total: {} fonts", font_names.len());
                println!("\nUsage: ttyvid convert -i input.cast -o output.gif --font FontName");
            }
        }
        None => {
            // Legacy mode: no subcommand, behave like convert
            if !args.formats.is_empty() && args.output.is_some() {
                // Multiple formats in legacy mode
                let output = args.output.as_ref().unwrap();
                let base_output = output.with_extension("");
                let mut generated_files = vec![];

                for format in &args.formats {
                    match format.to_lowercase().as_str() {
                        "cast" => {
                            if let Some(ref input) = args.input {
                                let cast_file = base_output.with_extension("cast");
                                std::fs::copy(input, &cast_file)?;
                                generated_files.push(cast_file);
                            }
                        }
                        "gif" => {
                            let gif_file = base_output.with_extension("gif");
                            eprintln!("\nConverting to GIF...");
                            convert_recording(&args, args.input.clone(), Some(gif_file.clone()))?;
                            generated_files.push(gif_file);
                        }
                        #[cfg(feature = "webm")]
                        "webm" => {
                            let webm_file = base_output.with_extension("webm");
                            eprintln!("\nConverting to WebM...");
                            convert_recording(&args, args.input.clone(), Some(webm_file.clone()))?;
                            generated_files.push(webm_file);
                        }
                        "md" | "markdown" => {
                            let md_file = base_output.with_extension("md");
                            generate_markdown(&base_output, &args.formats, &md_file)?;
                            generated_files.push(md_file);
                        }
                        _ => {
                            eprintln!("Warning: Unknown format '{}', skipping", format);
                        }
                    }
                }

                eprintln!("\nGenerated files:");
                for file in &generated_files {
                    eprintln!("  ✓ {}", file.display());
                }
            } else {
                // Single format legacy mode
                convert_recording(&args, args.input.clone(), args.output.clone())?;
            }
        }
    }

    Ok(())
}

fn convert_recording(args: &cli::Args, input: Option<PathBuf>, output: Option<PathBuf>) -> Result<()> {
    // Query terminal size if requested
    let (term_cols, term_rows) = if args.clone || args.terminal_size {
        use crossterm::terminal;
        if let Ok((cols, rows)) = terminal::size() {
            eprintln!("Using terminal size: {}x{}", cols, rows);
            (Some(cols as usize), Some(rows as usize))
        } else {
            eprintln!("Warning: Could not query terminal size, using defaults");
            (None, None)
        }
    } else {
        (None, None)
    };

    // Determine output format
    let output_format = if let Some(ref fmt_str) = args.format {
        // Explicit format specified
        let explicit_format = match fmt_str.to_lowercase().as_str() {
            "gif" => OutputFormat::Gif,
            #[cfg(feature = "webm")]
            "webm" => OutputFormat::Webm,
            _ => {
                #[cfg(feature = "webm")]
                {
                    anyhow::bail!("Unknown format: {}. Supported formats: gif, webm", fmt_str)
                }
                #[cfg(not(feature = "webm"))]
                {
                    anyhow::bail!("Unknown format: {}. Supported formats: gif (compile with --features webm for WebM support)", fmt_str)
                }
            }
        };

        // Warn if explicit format doesn't match output file extension
        if let Some(ref path) = output {
            if let Some(detected_format) = OutputFormat::from_path(path) {
                if detected_format != explicit_format {
                    eprintln!("Warning: Specified format '{:?}' doesn't match output file extension '.{}'",
                        explicit_format, path.extension().and_then(|e| e.to_str()).unwrap_or("?"));
                    eprintln!("         Using specified format: {:?}", explicit_format);
                }
            }
        }

        explicit_format
    } else if let Some(ref path) = output {
        // Auto-detect from output file extension
        OutputFormat::from_path(path).unwrap_or(OutputFormat::Gif)
    } else {
        // Default to GIF
        OutputFormat::Gif
    };

    // Determine output path
    let output_path = if let Some(path) = output {
        path
    } else {
        // Auto-generate filename with proper extension
        let ext = output_format.extension();
        let mut index = 0;
        loop {
            let filename = format!("ttyvid-{:04}.{}", index, ext);
            if !std::path::Path::new(&filename).exists() {
                break PathBuf::from(filename);
            }
            index += 1;
            if index >= 10000 {
                anyhow::bail!("No available output filenames (ttyvid-0000.{} to ttyvid-9999.{} all exist)", ext, ext);
            }
        }
    };

    // Read input events
    let mut input_source: Box<dyn InputSource> = if let Some(ref path) = input {
        Box::new(AsciicastReader::new(path)?)
    } else {
        Box::new(StdinReader::new(
            args.columns.unwrap_or(80),
            args.rows.unwrap_or(25),
        ))
    };

    let mut events = input_source.read_events()?;
    let metadata = input_source.metadata();

    // Determine dimensions: terminal size > explicit args > metadata
    let width = term_cols.or(args.columns).unwrap_or(metadata.width);
    let height = term_rows.or(args.rows).unwrap_or(metadata.height);

    // Load theme
    let theme = {
        let theme_path = std::path::Path::new(&args.theme);
        if theme_path.exists() && theme_path.is_file() {
            // Direct path to theme file
            Theme::load(theme_path)?
        } else {
            // Load by name (searches filesystem + embedded)
            Theme::load_by_name(&args.theme)?
        }
    };

    println!(" - input: {}", input.as_deref().unwrap_or(std::path::Path::new("stdin")).display());
    println!(" - output: {}", output_path.display());
    println!(" - format: {:?}", output_format);
    println!(" - theme: {}", theme.name);
    println!(" - speed: {}", args.speed);
    println!(" - events: {}", events.len());
    println!(" - character dimensions: {}x{}", width, height);

    // Apply speed multiplier to timestamps
    if args.speed != 1.0 {
        for event in &mut events {
            event.timestamp /= args.speed;
        }
    }

    // Remove gaps if requested
    if args.no_gaps {
        remove_gaps(&mut events);
    }

    // Calculate total duration and frame count
    let duration = if !events.is_empty() {
        events.last().unwrap().timestamp
    } else {
        0.0
    };
    let frame_rate = args.fps.clamp(1, 100);
    let frame_count = ((duration * frame_rate as f64).ceil() as usize).max(1);

    // Add trailer frames if requested (1.5 seconds holding the final state)
    let trailer_frame_count = if args.trailer {
        (frame_rate as f64 * 1.5).round() as usize
    } else {
        0
    };
    let total_frame_count = frame_count + trailer_frame_count;

    println!(" - frame rate: {}", frame_rate);
    println!(" - frames: {}", frame_count);
    println!(" - seconds: {:.2}", duration);
    if args.trailer {
        println!(" - trailer: {} frames (1.5s)", trailer_frame_count);
    }

    // Query terminal colors early if needed (gets palette + default colors in one go)
    let (palette, term_default_fg, term_default_bg) = if args.clone || args.terminal_colors {
        eprintln!("Querying terminal for colors...");
        // This queries the full palette AND default fg/bg colors
        let (pal, fg, bg) = Palette::from_terminal();
        eprintln!("Terminal colors detected: fg={:?}, bg={:?}", fg, bg);
        (Some(pal), fg, bg)
    } else if let Some(ref theme_palette) = theme.palette {
        // Use theme's custom palette
        (Some(Palette::from_theme(theme_palette)), None, None)
    } else {
        // Fall back to default
        (Some(Palette::default()), None, None)
    };

    // Create terminal emulator with colors (terminal colors override theme)
    let default_fg = term_default_fg.unwrap_or(theme.default_foreground);
    let default_bg = term_default_bg.unwrap_or(theme.default_background);
    eprintln!("Using colors: fg={}, bg={}", default_fg, default_bg);
    let mut terminal = TerminalEmulator::new(width, height, !args.no_autowrap, default_fg, default_bg);

    // Create rasterizer with font (GPU-accelerated if compiled with --features gpu)
    #[cfg(feature = "gpu")]
    let rasterizer = {
        let font = if let Some(ref system_font) = args.system_font {
            eprintln!("Loading system font: {} at size {}", system_font, args.font_size);
            if let Some(ttf_font) = Font::from_system_font(system_font, args.font_size) {
                eprintln!("Successfully loaded system font (cell size: {}x{})", ttf_font.width(), ttf_font.height());
                ttf_font
            } else {
                eprintln!("Failed to load system font, falling back to embedded bitmap font");
                Font::load(args.font.as_deref())
            }
        } else if args.clone {
            if let Some(font_name) = query_terminal_font() {
                eprintln!("Terminal font detected: {}", font_name);
                if let Some(ttf_font) = Font::from_system_font(&font_name, args.font_size) {
                    eprintln!("Loaded TrueType font: {} (cell size: {}x{})", font_name, ttf_font.width(), ttf_font.height());
                    ttf_font
                } else {
                    eprintln!("Could not load font '{}', falling back to embedded font", font_name);
                    Font::load(args.font.as_deref())
                }
            } else {
                eprintln!("Could not detect terminal font, using embedded font");
                Font::load(args.font.as_deref())
            }
        } else {
            Font::load(args.font.as_deref())
        };
        GpuRenderer::new(font, palette.as_ref().unwrap().clone())
    };

    #[cfg(not(feature = "gpu"))]
    let rasterizer = if let Some(ref system_font) = args.system_font {
        eprintln!("Loading system font: {} at size {}", system_font, args.font_size);
        if let Some(ttf_font) = Font::from_system_font(system_font, args.font_size) {
            eprintln!("Successfully loaded system font (cell size: {}x{})", ttf_font.width(), ttf_font.height());
            Rasterizer::with_font(ttf_font)
        } else {
            eprintln!("Failed to load system font, falling back to embedded bitmap font");
            Rasterizer::new(args.font.as_deref())
        }
    } else if args.clone {
        if let Some(font_name) = query_terminal_font() {
            eprintln!("Terminal font detected: {}", font_name);
            if let Some(ttf_font) = Font::from_system_font(&font_name, args.font_size) {
                eprintln!("Loaded TrueType font: {} (cell size: {}x{})", font_name, ttf_font.width(), ttf_font.height());
                Rasterizer::with_font(ttf_font)
            } else {
                eprintln!("Could not load font '{}', falling back to embedded font", font_name);
                Rasterizer::new(args.font.as_deref())
            }
        } else {
            eprintln!("Could not detect terminal font, using embedded font");
            Rasterizer::new(args.font.as_deref())
        }
    } else {
        Rasterizer::new(args.font.as_deref())
    };

    let (term_pixel_width, term_pixel_height) = rasterizer.canvas_size(width, height);

    // Apply theme padding
    let (padding_left, padding_top, padding_right, padding_bottom) = if let Some(ref padding) = theme.padding {
        (padding.left as usize, padding.top as usize, padding.right as usize, padding.bottom as usize)
    } else {
        (0, 0, 0, 0)
    };

    let pixel_width = term_pixel_width + padding_left + padding_right;
    let pixel_height = term_pixel_height + padding_top + padding_bottom;

    println!(" - terminal pixel dimensions: {}x{}", term_pixel_width, term_pixel_height);
    if padding_left > 0 || padding_top > 0 || padding_right > 0 || padding_bottom > 0 {
        println!(" - padding: L:{} T:{} R:{} B:{}", padding_left, padding_top, padding_right, padding_bottom);
    }
    println!(" - final pixel dimensions: {}x{}", pixel_width, pixel_height);

    // Load theme layers (without pre-processing - render per mode in frame loop)
    let mut layer_renderer = LayerRenderer::new();
    for layer in &theme.layers {
        // Search for layer file in multiple locations
        let layer_path = find_layer_file(&layer.file);

        match LayerImage::load(&layer_path) {
            Ok(layer_image) => {
                let anim_info = if layer_image.is_animated {
                    format!(" [{} frames, animated]", layer_image.frame_count())
                } else {
                    String::new()
                };
                println!(" - loaded layer: {} ({}x{}) mode={:?} depth={}{}",
                    layer.file, layer_image.width, layer_image.height, layer.mode, layer.depth, anim_info);
                layer_renderer.add_layer(layer_image, layer.clone());
            }
            Err(e) => {
                eprintln!("Warning: Failed to load layer image {}: {}", layer.file, e);
            }
        }
    }

    // Use the palette we already queried earlier (don't query again!)
    let palette = palette.unwrap(); // Safe because we always set Some() above

    // Use terminal background for canvas fill (overrides theme background)
    let background_color = term_default_bg.unwrap_or(theme.background);
    eprintln!("Canvas background color index: {}", background_color);

    // Don't use transparency in final GIF - transparency is handled during layer compositing
    // The final output should be fully opaque with the theme background color
    let transparent_index = None;

    let mut encoder = EncoderWrapper::new(
        &output_path,
        pixel_width,
        pixel_height,
        &palette,
        output_format,
        args.r#loop,
        frame_rate,
        args.quality.clamp(0, 100),
        transparent_index,
    )?;

    // GPU BATCH MODE: Process frames in two passes
    // Pass 1: Collect all Grid snapshots (~14MB for 282 frames)
    // Pass 2: Batch render ALL grids at once (ONE GPU sync!)

    let frame_duration = 1.0 / frame_rate as f64;
    let delay_centiseconds = (100.0 / frame_rate as f64).round() as u16;

    #[cfg(feature = "gpu")]
    let use_batch_rendering = rasterizer.is_gpu_available();
    #[cfg(not(feature = "gpu"))]
    let use_batch_rendering = false;

    // PASS 1: Collect all grid snapshots
    let term_canvases: Vec<Canvas> = if use_batch_rendering {
        let mut grids = Vec::with_capacity(total_frame_count);
        let mut event_idx = 0;

        for frame_num in 0..total_frame_count {
            let current_time = frame_num as f64 * frame_duration;

            // Process all events up to current time (only for non-trailer frames)
            if frame_num < frame_count {
                while event_idx < events.len() && events[event_idx].timestamp <= current_time {
                    terminal.feed_bytes(&events[event_idx].data);
                    event_idx += 1;
                }
            }

            // Clone the grid snapshot (Grid is cheap to clone - just Vec<Cell> where Cell is Copy)
            grids.push(terminal.grid().clone());
        }

        // PASS 2: GPU BATCH RENDER (ONE sync for ALL frames!)
        #[cfg(feature = "gpu")]
        {
            let batch_result = rasterizer.render_grids_batch(&grids);
            match batch_result {
                Ok(canvases) => {
                    canvases
                }
                Err(e) => {
                    eprintln!("GPU batch render failed: {}, falling back to frame-by-frame", e);
                    // Fallback: render each grid individually
                    grids.iter().map(|grid| {
                        if !args.no_cursor && terminal.state().display_cursor {
                            let (cursor_x, cursor_y) = terminal.state().cursor_get_position();
                            rasterizer.render_grid_with_cursor(grid, cursor_x as usize, cursor_y as usize)
                        } else {
                            rasterizer.render_grid(grid)
                        }
                    }).collect()
                }
            }
        }
        #[cfg(not(feature = "gpu"))]
        {
            vec![] // Won't be used
        }
    } else {
        // CPU MODE: Render frame-by-frame as before
        vec![] // We'll render in the loop below
    };

    // PASS 3: Composite with layers and encode
    let start_time = std::time::Instant::now();
    let mut event_idx = 0;
    for frame_num in 0..total_frame_count {
        let current_time = frame_num as f64 * frame_duration;

        // Get or render terminal canvas
        let term_canvas = if use_batch_rendering {
            // Use pre-rendered canvas from batch
            term_canvases[frame_num].clone()
        } else {
            // CPU path: process events and render frame-by-frame
            if frame_num < frame_count {
                while event_idx < events.len() && events[event_idx].timestamp <= current_time {
                    terminal.feed_bytes(&events[event_idx].data);
                    event_idx += 1;
                }
            }

            if !args.no_cursor && terminal.state().display_cursor {
                let (cursor_x, cursor_y) = terminal.state().cursor_get_position();
                rasterizer.render_grid_with_cursor(
                    terminal.grid(),
                    cursor_x as usize,
                    cursor_y as usize
                )
            } else {
                rasterizer.render_grid(terminal.grid())
            }
        };

        // Create final canvas with padding
        let mut canvas = Canvas::new(pixel_width, pixel_height, &palette);

        // Fill with background color (terminal bg overrides theme bg)
        for y in 0..pixel_height {
            for x in 0..pixel_width {
                canvas.set_pixel(x, y, background_color);
            }
        }

        // Convert current time to milliseconds for animation
        let current_time_ms = current_time * 1000.0;

        // Render underlay layers (depth < 0)
        layer_renderer.render_underlays(&mut canvas, palette.colors(), current_time_ms);

        // Composite terminal output onto canvas with padding offset
        for y in 0..term_pixel_height {
            for x in 0..term_pixel_width {
                if let Some(color) = term_canvas.get_pixel(x, y) {
                    canvas.set_pixel(x + padding_left, y + padding_top, color);
                }
            }
        }

        // Render title text if provided
        if let Some(ref title_text) = args.title {
            if !title_text.is_empty() {
                if let Some(ref title_config) = theme.title {
                    rasterizer.render_title(
                        &mut canvas,
                        title_config.x,
                        title_config.y,
                        title_text,
                        title_config.foreground,
                        title_config.background,
                        title_config.font_size,
                    );
                }
            }
        }

        // Render overlay layers (depth >= 0)
        layer_renderer.render_overlays(&mut canvas, palette.colors(), current_time_ms);

        // Add frame to GIF
        encoder.add_frame(&canvas, delay_centiseconds)?;

        // Progress indicator with ETA
        let percent = ((frame_num + 1) as f64 / total_frame_count as f64 * 100.0) as usize;
        let status = if frame_num >= frame_count {
            format!("[TRAILER {}/{}]", frame_num - frame_count + 1, trailer_frame_count)
        } else {
            format!("{} of {:.2}s", (current_time as f32).min(duration as f32), duration)
        };

        // Calculate ETA
        let elapsed = start_time.elapsed().as_secs_f64();
        let eta_str = if frame_num > 0 {
            let frames_remaining = total_frame_count - (frame_num + 1);
            let seconds_per_frame = elapsed / (frame_num + 1) as f64;
            let eta_seconds = frames_remaining as f64 * seconds_per_frame;
            if eta_seconds < 60.0 {
                format!("ETA: {:.1}s", eta_seconds)
            } else {
                format!("ETA: {:.1}m", eta_seconds / 60.0)
            }
        } else {
            "ETA: --".to_string()
        };

        print!("\r  {} {}% Frame: {}/{} {} FPS {}       ",
            status,
            percent,
            frame_num + 1,
            total_frame_count,
            frame_rate,
            eta_str
        );
        use std::io::Write;
        std::io::stdout().flush()?;
    }

    println!();

    // Finish encoding
    encoder.finish()?;

    let total_time = start_time.elapsed();
    let time_str = if total_time.as_secs() < 60 {
        format!("{:.1}s", total_time.as_secs_f64())
    } else {
        let minutes = total_time.as_secs() / 60;
        let seconds = total_time.as_secs() % 60;
        format!("{}m {}s", minutes, seconds)
    };

    println!("\n✓ {:?} created: {} (total time: {})", output_format, output_path.display(), time_str);

    Ok(())
}

fn generate_markdown(base_path: &PathBuf, formats: &[String], md_file: &PathBuf) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    let mut content = String::new();

    // Get the base filename for the title
    let title = base_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Terminal Recording");

    content.push_str(&format!("# {}\n\n", title));

    // Embed the GIF or WebM if available
    if formats.contains(&"gif".to_string()) {
        let gif_name = base_path.with_extension("gif");
        let gif_filename = gif_name.file_name().and_then(|n| n.to_str()).unwrap_or("output.gif");
        content.push_str(&format!("![{}]({})\n\n", title, gif_filename));
    } else if formats.contains(&"webm".to_string()) {
        let webm_name = base_path.with_extension("webm");
        let webm_filename = webm_name.file_name().and_then(|n| n.to_str()).unwrap_or("output.webm");
        content.push_str(&format!("<video src=\"{}\" controls></video>\n\n", webm_filename));
    }

    // Add link to .cast file if available
    if formats.contains(&"cast".to_string()) {
        let cast_name = base_path.with_extension("cast");
        let cast_filename = cast_name.file_name().and_then(|n| n.to_str()).unwrap_or("output.cast");
        content.push_str(&format!("## Files\n\n"));
        content.push_str(&format!("- [Asciinema recording]({})\n", cast_filename));
    }

    content.push_str(&format!("\n---\n\nGenerated with [ttyvid](https://github.com/ndonald2/ttyvid) v{}\n", env!("CARGO_PKG_VERSION")));

    let mut file = File::create(md_file)?;
    file.write_all(content.as_bytes())?;

    Ok(())
}

fn remove_gaps(events: &mut [input::Event]) {
    if events.is_empty() {
        return;
    }

    let mut prev_time = 0.0;
    let mut gap_offset = 0.0;

    for event in events.iter_mut() {
        let gap = event.timestamp - prev_time;
        if gap > 1.0 {
            gap_offset += gap - 1.0;
        }
        event.timestamp -= gap_offset;
        prev_time = event.timestamp;
    }
}
