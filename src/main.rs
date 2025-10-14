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

use input::{InputSource, AsciicastReader, StdinReader};
use terminal::TerminalEmulator;
use renderer::{Rasterizer, Palette, Canvas};
use encoder::{EncoderWrapper, OutputFormat};
use theme::Theme;
use theme::layers::{LayerRenderer, LayerImage};

/// Find layer file in theme directories
fn find_layer_file(layer_file: &str) -> PathBuf {
    // Try as absolute path first
    let absolute_path = PathBuf::from(layer_file);
    if absolute_path.exists() {
        return absolute_path;
    }

    // Search in theme directories
    let search_paths = vec![
        PathBuf::from("themes"),
        PathBuf::from("/usr/share/ttyvid/themes"),
        PathBuf::from("/usr/local/share/ttyvid/themes"),
    ];

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

    println!("ttyvid version [0.1.0]\n");

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
        if let Some(ref path) = args.output {
            if let Some(detected_format) = OutputFormat::from_path(path) {
                if detected_format != explicit_format {
                    eprintln!("Warning: Specified format '{:?}' doesn't match output file extension '.{}'",
                        explicit_format, path.extension().and_then(|e| e.to_str()).unwrap_or("?"));
                    eprintln!("         Using specified format: {:?}", explicit_format);
                }
            }
        }

        explicit_format
    } else if let Some(ref path) = args.output {
        // Auto-detect from output file extension
        OutputFormat::from_path(path).unwrap_or(OutputFormat::Gif)
    } else {
        // Default to GIF
        OutputFormat::Gif
    };

    // Determine output path
    let output_path = if let Some(path) = args.output {
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
    let mut input: Box<dyn InputSource> = if let Some(ref path) = args.input {
        Box::new(AsciicastReader::new(path)?)
    } else {
        Box::new(StdinReader::new(
            args.columns.unwrap_or(80),
            args.rows.unwrap_or(25),
        ))
    };

    let mut events = input.read_events()?;
    let metadata = input.metadata();

    let width = args.columns.unwrap_or(metadata.width);
    let height = args.rows.unwrap_or(metadata.height);

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

    println!(" - input: {}", args.input.as_deref().unwrap_or(std::path::Path::new("stdin")).display());
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
    let mut frame_count = ((duration * frame_rate as f64).ceil() as usize).max(1);

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

    // Create terminal emulator with theme colors
    let default_fg = theme.default_foreground;
    let default_bg = theme.default_background;
    let mut terminal = TerminalEmulator::new(width, height, !args.no_autowrap, default_fg, default_bg);

    // Create rasterizer
    let rasterizer = Rasterizer::new(args.font.as_deref());
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

    // Create encoder - use theme palette if available
    let palette = if let Some(ref theme_palette) = theme.palette {
        Palette::from_theme(theme_palette)
    } else {
        Palette::default()
    };
    let mut encoder = EncoderWrapper::new(
        &output_path,
        pixel_width,
        pixel_height,
        &palette,
        output_format,
        args.r#loop,
        frame_rate,
        args.quality.clamp(0, 100),
    )?;

    // Process frames
    let frame_duration = 1.0 / frame_rate as f64;
    let delay_centiseconds = (100.0 / frame_rate as f64).round() as u16;

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
        // During trailer frames, terminal state remains at final state

        // Render current terminal state
        let term_canvas = rasterizer.render_grid(terminal.grid());

        // Create final canvas with padding
        let mut canvas = Canvas::new(pixel_width, pixel_height, &palette);

        // Fill with theme background color
        for y in 0..pixel_height {
            for x in 0..pixel_width {
                canvas.set_pixel(x, y, theme.background);
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

        // Progress indicator
        let percent = ((frame_num + 1) as f64 / total_frame_count as f64 * 100.0) as usize;
        let status = if frame_num >= frame_count {
            format!("[TRAILER {}/{}]", frame_num - frame_count + 1, trailer_frame_count)
        } else {
            format!("{} of {:.2}s", (current_time as f32).min(duration as f32), duration)
        };
        print!("\r  {} {}% Frame: {}/{} {} FPS       ",
            status,
            percent,
            frame_num + 1,
            total_frame_count,
            frame_rate
        );
        use std::io::Write;
        std::io::stdout().flush()?;
    }

    println!();

    // Finish encoding
    encoder.finish()?;

    println!("\n✓ {:?} created: {}", output_format, output_path.display());

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
