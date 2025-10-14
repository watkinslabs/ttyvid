use image::{RgbaImage, Rgba};
use fontdue::{Font, FontSettings};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: font_debug <font_path> <font_size>");
        eprintln!("Example: font_debug /usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf 16");
        std::process::exit(1);
    }

    let font_path = &args[1];
    let font_size: f32 = args[2].parse().expect("Invalid font size");

    // Load font
    let font_data = fs::read(font_path).expect("Failed to read font file");
    let font = Font::from_bytes(font_data.as_slice(), FontSettings::default())
        .expect("Failed to parse font");

    // Parse font metrics
    let face = ttf_parser::Face::parse(&font_data, 0).expect("Failed to parse font metrics");
    let ascender = face.ascender() as f32;
    let descender = face.descender() as f32;
    let units_per_em = face.units_per_em() as f32;
    let scale = font_size / units_per_em;
    let ascender_px = ascender * scale;
    let descender_px = descender * scale;

    println!("=== Font Metrics ===");
    println!("Font: {}", font_path);
    println!("Font size: {}px", font_size);
    println!("Units per EM: {}", units_per_em);
    println!("Scale: {}", scale);
    println!("Ascender (font units): {}", ascender);
    println!("Descender (font units): {}", descender);
    println!("Ascender (pixels): {:.2}", ascender_px);
    println!("Descender (pixels): {:.2}", descender_px);
    println!("Total height: {:.2}", ascender_px - descender_px);
    println!();

    // Generate all printable ASCII characters
    let chars: Vec<char> = (32..127).map(|c| c as u8 as char).collect();

    // Calculate cell dimensions based on font metrics (not individual glyphs)
    let padding_top = 2.0;
    let padding_bottom = 2.0;
    let cell_height = (ascender_px - descender_px + padding_top + padding_bottom).ceil() as usize;

    // Baseline is positioned at: top_padding + ascender_height
    let baseline_offset = padding_top + ascender_px;

    // Measure max width
    let mut max_width = 0;
    for &ch in &chars {
        let (metrics, _) = font.rasterize(ch, font_size);
        max_width = max_width.max(metrics.width);
    }
    let cell_width = max_width + 4;

    println!("=== Cell Dimensions ===");
    println!("Cell width: {}", cell_width);
    println!("Cell height: {}", cell_height);
    println!("Baseline offset from top: {:.2}", baseline_offset);
    println!();

    // Layout: 16 characters per row
    let cols = 16;
    let rows = (chars.len() + cols - 1) / cols;

    let img_width = (cell_width * cols) as u32;
    let img_height = (cell_height * rows) as u32;

    let mut img = RgbaImage::from_pixel(img_width, img_height, Rgba([255, 255, 255, 255]));

    // Colors
    let black = Rgba([0, 0, 0, 255]);
    let red = Rgba([255, 0, 0, 255]);
    let green = Rgba([0, 255, 0, 255]);
    let blue = Rgba([0, 0, 255, 255]);

    println!("=== Character Details ===");

    // Render each character
    for (idx, &ch) in chars.iter().enumerate() {
        let col = idx % cols;
        let row = idx / cols;

        let cell_x = (col * cell_width) as i32;
        let cell_y = (row * cell_height) as i32;

        // Rasterize the character
        let (metrics, bitmap) = font.rasterize(ch, font_size);

        // Print metrics
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == 'g' || ch == 'y' || ch == 'p' {
            println!("'{}' (0x{:02x}): width={}, height={}, ymin={}, ymax={}",
                     ch, ch as u32, metrics.width, metrics.height,
                     metrics.ymin, metrics.ymin + metrics.height as i32);
        }

        // Calculate position
        // X: center horizontally in cell
        let glyph_x = cell_x + ((cell_width - metrics.width) / 2) as i32;

        // Y: position relative to baseline
        // baseline_offset is the baseline position from top of cell
        // metrics.ymin is the distance from baseline to BOTTOM of glyph
        // negative ymin = bottom extends below baseline
        // positive ymin = bottom is above baseline
        // TOP of glyph = baseline - ymin - height
        let glyph_y = cell_y + baseline_offset as i32 - metrics.ymin - metrics.height as i32;

        // Draw the glyph
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let px = glyph_x + x as i32;
                let py = glyph_y + y as i32;

                if px >= 0 && py >= 0 && (px as u32) < img_width && (py as u32) < img_height {
                    let alpha = bitmap[y * metrics.width + x];
                    if alpha > 128 {
                        img.put_pixel(px as u32, py as u32, black);
                    }
                }
            }
        }

        // Draw guide lines for each row (only once per row)
        if col == 0 {
            let top_y = cell_y as u32;
            let baseline_y = (cell_y + baseline_offset as i32) as u32;
            let bottom_y = (cell_y + cell_height as i32 - 1) as u32;

            // Red line at top
            for x in 0..img_width {
                img.put_pixel(x, top_y, red);
            }

            // Blue line at baseline
            for x in 0..img_width {
                if baseline_y < img_height {
                    img.put_pixel(x, baseline_y, blue);
                }
            }

            // Green line at bottom
            for x in 0..img_width {
                if bottom_y < img_height {
                    img.put_pixel(x, bottom_y, green);
                }
            }
        }

        // Draw vertical separator between characters (light gray)
        let sep_color = Rgba([200, 200, 200, 255]);
        for y in 0..cell_height {
            let px = (cell_x + cell_width as i32) as u32;
            let py = (cell_y + y as i32) as u32;
            if px < img_width && py < img_height {
                img.put_pixel(px, py, sep_color);
            }
        }
    }

    // Save the image
    let output_path = "font_debug.png";
    img.save(output_path).expect("Failed to save image");
    println!();
    println!("Image saved to: {}", output_path);
    println!();
    println!("Legend:");
    println!("  RED line = Top of cell");
    println!("  BLUE line = Baseline");
    println!("  GREEN line = Bottom of cell");
}
