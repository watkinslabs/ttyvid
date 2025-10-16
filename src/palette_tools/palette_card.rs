use anyhow::Result;
use std::path::PathBuf;
use image::{ImageBuffer, Rgb};
use crate::theme::Theme;
use crate::font_tools::Font;
use super::Palette;

pub fn generate_palette_card(theme_name: &str, output: &PathBuf, fg_index: Option<u8>, bg_index: Option<u8>) -> Result<()> {
    // Load theme to get palette
    let theme = {
        let theme_path = std::path::Path::new(theme_name);
        if theme_path.exists() && theme_path.is_file() {
            Theme::load(theme_path)?
        } else {
            Theme::load_by_name(theme_name)?
        }
    };

    let palette = if let Some(ref theme_palette) = theme.palette {
        Palette::from_theme(theme_palette)
    } else {
        Palette::default()
    };

    // Use theme defaults or provided values
    let default_fg = fg_index.unwrap_or(theme.default_foreground);
    let default_bg = bg_index.unwrap_or(theme.default_background);

    // Load font for text rendering
    let font = Font::load(None);

    // Image dimensions
    let cell_size: usize = 40;  // Size of each color cell
    let padding: usize = 10;
    let header_height: usize = 30;  // Space for title header

    // Palette grid: 16x16 = 256 colors
    let palette_cols: usize = 16;
    let palette_rows: usize = 16;
    let palette_width = palette_cols * cell_size + padding * 2;
    let palette_height = palette_rows * cell_size + padding * 2;

    // Test pattern dimensions (classic TV test pattern)
    let pattern_width: usize = 600;
    let pattern_height = palette_height;

    // Legend height
    let legend_height: usize = 100;

    // Total dimensions (add header_height to top)
    let total_width = palette_width + pattern_width + padding;
    let total_height = header_height + palette_height.max(pattern_height) + legend_height + padding * 2;

    let mut img = ImageBuffer::from_pixel(total_width as u32, total_height as u32, Rgb([32u8, 32u8, 32u8]));

    // Draw title header at top
    let title_text = format!("Theme: {}", theme.name);
    let title_x = padding;
    let title_y = 8;

    for (char_idx, c) in title_text.chars().enumerate() {
        let char_x = title_x + char_idx * font.width() * 2;  // 2x scale for title
        let glyph = font.get_glyph_utf8(c);

        for py in 0..font.height() {
            for px in 0..font.width() {
                let glyph_idx = py * font.width() + px;
                if glyph_idx < glyph.len() && glyph[glyph_idx] > 25 {
                    // Draw 2x2 block for each pixel (scaled up title)
                    for sy in 0..2 {
                        for sx in 0..2 {
                            let img_x = (char_x + px * 2 + sx) as u32;
                            let img_y = (title_y + py * 2 + sy) as u32;
                            if img_x < total_width as u32 && img_y < total_height as u32 {
                                img.put_pixel(img_x, img_y, Rgb([255u8, 255u8, 255u8]));
                            }
                        }
                    }
                }
            }
        }
    }

    // Draw 256-color palette grid (left side)
    for i in 0..256 {
        let row = i / palette_cols;
        let col = i % palette_cols;
        let x = padding + col * cell_size;
        let y = header_height + padding + row * cell_size;

        let (r, g, b) = palette.get_rgb(i as u8);
        let color = Rgb([r, g, b]);

        // Fill cell
        for py in 0..cell_size {
            for px in 0..cell_size {
                let img_x = (x + px) as u32;
                let img_y = (y + py) as u32;
                if img_x < total_width as u32 && img_y < total_height as u32 {
                    img.put_pixel(img_x, img_y, color);
                }
            }
        }

        // Draw cell border (thin gray line)
        let border_color = Rgb([64u8, 64u8, 64u8]);
        for px in 0..cell_size {
            let img_x = (x + px) as u32;
            if img_x < total_width as u32 {
                img.put_pixel(img_x, y as u32, border_color);
                img.put_pixel(img_x, (y + cell_size - 1) as u32, border_color);
            }
        }
        for py in 0..cell_size {
            let img_y = (y + py) as u32;
            if img_y < total_height as u32 {
                img.put_pixel(x as u32, img_y, border_color);
                img.put_pixel((x + cell_size - 1) as u32, img_y, border_color);
            }
        }
    }

    // Draw base 8 colors (0-7) and bright colors (8-15) comparison on right side
    let pattern_x_offset = palette_width + padding;
    let color_bar_width = pattern_width / 2;
    let color_bar_height = cell_size;

    // Draw colors 0-7 (left column) and 8-15 (right column)
    for i in 0..8 {
        let y_pos = header_height + padding + i * color_bar_height;

        // Base color (0-7) - left side
        let (r0, g0, b0) = palette.get_rgb(i as u8);
        for y in y_pos..(y_pos + color_bar_height) {
            for x in pattern_x_offset..(pattern_x_offset + color_bar_width) {
                if x < total_width && y < total_height {
                    img.put_pixel(x as u32, y as u32, Rgb([r0, g0, b0]));
                }
            }
        }

        // Bright color (8-15) - right side
        let (r1, g1, b1) = palette.get_rgb((i + 8) as u8);
        for y in y_pos..(y_pos + color_bar_height) {
            for x in (pattern_x_offset + color_bar_width)..(pattern_x_offset + pattern_width) {
                if x < total_width && y < total_height {
                    img.put_pixel(x as u32, y as u32, Rgb([r1, g1, b1]));
                }
            }
        }

        // Draw divider line between base and bright
        for y in y_pos..(y_pos + color_bar_height) {
            let divider_x = (pattern_x_offset + color_bar_width) as u32;
            if divider_x < total_width as u32 && y < total_height {
                img.put_pixel(divider_x, y as u32, Rgb([128u8, 128u8, 128u8]));
            }
        }
    }

    // Draw SMPTE test pattern in remaining space
    let smpte_y_start = header_height + padding + 8 * color_bar_height;
    let smpte_height = pattern_height - (8 * color_bar_height) - padding;
    let bar_width = pattern_width / 7;

    let smpte_colors = [
        (192u8, 192u8, 192u8), // White
        (192, 192, 0),   // Yellow
        (0, 192, 192),   // Cyan
        (0, 192, 0),     // Green
        (192, 0, 192),   // Magenta
        (192, 0, 0),     // Red
        (0, 0, 192),     // Blue
    ];

    for (bar_idx, &(r, g, b)) in smpte_colors.iter().enumerate() {
        let x_start = pattern_x_offset + bar_idx * bar_width;
        for y in smpte_y_start..(smpte_y_start + smpte_height / 2) {
            for x in x_start..(x_start + bar_width) {
                if x < total_width && y < total_height {
                    img.put_pixel(x as u32, y as u32, Rgb([r, g, b]));
                }
            }
        }
    }

    // Gradient at bottom of right panel
    let gradient_y = smpte_y_start + smpte_height / 2;
    let gradient_height = smpte_height / 2;

    for y in gradient_y..(gradient_y + gradient_height) {
        for x in pattern_x_offset..(pattern_x_offset + pattern_width) {
            if x < total_width && y < total_height {
                let progress = (x - pattern_x_offset) as f32 / pattern_width as f32;
                let value = (progress * 255.0) as u8;
                img.put_pixel(x as u32, y as u32, Rgb([value, value, value]));
            }
        }
    }

    // Draw legend at bottom
    let legend_y = header_height + palette_height + padding * 2;
    let legend_box_width: usize = 120;
    let legend_box_height: usize = 60;
    let legend_spacing: usize = 20;

    // Helper function to draw a labeled color box with contrasting outline and text label
    let draw_legend_box = |img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, x: usize, y: usize, color_idx: u8, label: &str| {
        let (r, g, b) = palette.get_rgb(color_idx);
        let color = Rgb([r, g, b]);

        // Choose contrasting outline color (white or black based on luminance)
        let luminance = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8;
        let outline_color = if luminance > 128 {
            Rgb([0u8, 0u8, 0u8])  // Black outline for bright colors
        } else {
            Rgb([255u8, 255u8, 255u8])  // White outline for dark colors
        };
        let text_color = outline_color;  // Use same color for text

        // Draw thick outline (4px)
        for py in 0..legend_box_height {
            for px in 0..legend_box_width {
                let img_x = (x + px) as u32;
                let img_y = (y + py) as u32;
                if img_x < total_width as u32 && img_y < total_height as u32 {
                    if px < 4 || px >= legend_box_width - 4 || py < 4 || py >= legend_box_height - 4 {
                        img.put_pixel(img_x, img_y, outline_color);
                    } else {
                        img.put_pixel(img_x, img_y, color);
                    }
                }
            }
        }

        // Draw label above the box using bitmap font
        let label_y = if y >= 20 { y - 18 } else { y + legend_box_height + 2 };
        let label_text = format!("{} [{}]", label, color_idx);

        // Draw gray background for label
        let font_height = font.height();
        let text_width = label_text.len() * font.width();
        for py in 0..font_height {
            for px in 0..text_width {
                let img_x = (x + px) as u32;
                let img_y = (label_y + py) as u32;
                if img_x < total_width as u32 && img_y < total_height as u32 {
                    img.put_pixel(img_x, img_y, Rgb([48u8, 48u8, 48u8]));
                }
            }
        }

        // Draw text using bitmap font
        for (char_idx, c) in label_text.chars().enumerate() {
            let char_x = x + char_idx * font.width();
            let glyph = font.get_glyph_utf8(c);

            for py in 0..font.height() {
                for px in 0..font.width() {
                    let glyph_idx = py * font.width() + px;
                    if glyph_idx < glyph.len() && glyph[glyph_idx] > 25 {
                        let img_x = (char_x + px) as u32;
                        let img_y = (label_y + py) as u32;
                        if img_x < total_width as u32 && img_y < total_height as u32 {
                            img.put_pixel(img_x, img_y, text_color);
                        }
                    }
                }
            }
        }

        println!("  {} = RGB({}, {}, {}) - Index {}", label, r, g, b, color_idx);
    };

    // Draw the five legend boxes
    let box_y = legend_y + 20;
    draw_legend_box(&mut img, padding, box_y, default_fg, "Default FG");
    draw_legend_box(&mut img, padding + legend_box_width + legend_spacing, box_y, default_bg, "Default BG");
    draw_legend_box(&mut img, padding + (legend_box_width + legend_spacing) * 2, box_y, theme.foreground, "Theme FG");
    draw_legend_box(&mut img, padding + (legend_box_width + legend_spacing) * 3, box_y, theme.background, "Theme BG");
    draw_legend_box(&mut img, padding + (legend_box_width + legend_spacing) * 4, box_y, theme.transparent, "Transparent");

    // Save PNG
    img.save(output)?;

    println!("\n✓ Palette card image created: {}", output.display());
    println!("  Theme: {}", theme.name);
    println!("  Total colors: 256");
    println!("  Layout: Palette grid (left) | TV test pattern (right) | Color legend (bottom)");

    Ok(())
}
