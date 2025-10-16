use super::Font;
use crate::renderer::Palette;
use anyhow::Result;
use image::{ImageBuffer, Rgb};
use std::path::Path;

/// Generate a font card showing all characters with their indices and unicode values
/// Shows characters in both white-on-black and black-on-white
pub fn generate_font_card(font: &Font, output_path: &Path, font_name: &str) -> Result<()> {
    let palette = Palette::default();

    // Font dimensions
    let char_width = font.width();
    let char_height = font.height();

    // Get the actual number of characters in the font
    let num_chars = font.glyph_count();

    // Layout: 16 columns, calculate rows based on character count
    let cols = 16;
    let rows = (num_chars + cols - 1) / cols; // Round up

    // Space for labels below each character (index + unicode)
    let label_height = 36;  // More room for labels
    let box_padding_top = 5;
    let box_padding_bottom = 5;
    let cell_height = box_padding_top + char_height + label_height + 10 + box_padding_bottom + 4; // Extra spacing + padding
    let cell_width = char_width + 60; // Extra horizontal spacing for labels and padding

    // Padding and margins
    let margin = 20;
    let section_gap = 40;
    let title_height = 40;
    let sample_height = 120; // Space for sample text (4 lines)

    // Calculate canvas dimensions
    // Two sections: white-on-black and black-on-white
    let grid_width = cols * cell_width;
    let grid_height = rows * cell_height;
    let canvas_width = grid_width + 2 * margin;
    let canvas_height = title_height + sample_height + margin + 2 * grid_height + section_gap + margin;

    let mut img = ImageBuffer::from_pixel(canvas_width as u32, canvas_height as u32, Rgb([235u8, 235u8, 235u8]));

    // Render title
    let title = format!("Font Card: {} ({} characters)", font_name, num_chars);
    render_text(font, &mut img, margin, 10, &title, Rgb([0, 0, 0]), Rgb([235, 235, 235]), 2.0);

    // Add sample text section showing the font in use
    let sample_y = title_height + 8;

    // Line 1: Standard pangram
    let sample_text1 = "The quick brown fox jumps over the lazy dog 0123456789";
    render_text(font, &mut img, margin, sample_y, sample_text1, Rgb([64, 64, 64]), Rgb([235, 235, 235]), 1.0);

    // Line 2: Special characters with box drawing (more spacing)
    let sample_y2 = sample_y + char_height + 8;
    let sample_text2 = "┌─────────┐  Symbols: ←↑→↓ █▓▒░ •○●◆ ±×÷≈";
    render_text(font, &mut img, margin, sample_y2, sample_text2, Rgb([64, 64, 64]), Rgb([235, 235, 235]), 1.0);

    // Line 3: Box with text (more spacing)
    let sample_y3 = sample_y2 + char_height + 8;
    let sample_text3 = "│ Example │  Blocks: ▀▄▌▐  Arrows: ▲▼►◄";
    render_text(font, &mut img, margin, sample_y3, sample_text3, Rgb([64, 64, 64]), Rgb([235, 235, 235]), 1.0);

    // Line 4: Close box (more spacing)
    let sample_y4 = sample_y3 + char_height + 8;
    let sample_text4 = "└─────────┘  Math: °±×÷≈≥≤≠  Currency: $¢£¥";
    render_text(font, &mut img, margin, sample_y4, sample_text4, Rgb([64, 64, 64]), Rgb([235, 235, 235]), 1.0);

    // Section 1: White on Black
    let section1_y = title_height + sample_height + margin + 40;
    let section_title = "White on Black";
    render_text(font, &mut img, margin, section1_y - 25, section_title, Rgb([0, 0, 0]), Rgb([235, 235, 235]), 1.0);
    render_font_section(
        font,
        &mut img,
        &palette,
        margin,
        section1_y,
        cols,
        num_chars,
        cell_width,
        cell_height,
        char_width,
        char_height,
        Rgb([255, 255, 255]), // white foreground
        Rgb([0, 0, 0]),       // black background
    );

    // Section 2: Black on White
    let section2_y = section1_y + grid_height + section_gap;
    let section_title2 = "Black on White";
    render_text(font, &mut img, margin, section2_y - 25, section_title2, Rgb([0, 0, 0]), Rgb([235, 235, 235]), 1.0);
    render_font_section(
        font,
        &mut img,
        &palette,
        margin,
        section2_y,
        cols,
        num_chars,
        cell_width,
        cell_height,
        char_width,
        char_height,
        Rgb([0, 0, 0]),       // black foreground
        Rgb([255, 255, 255]), // white background
    );

    // Save to PNG
    img.save(output_path)?;

    println!("\n✓ Font card created: {}", output_path.display());
    println!("  Font: {}", font_name);
    println!("  Cell size: {}x{}", char_width, char_height);
    println!("  Total characters: {}", num_chars);

    Ok(())
}

fn render_font_section(
    font: &Font,
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    _palette: &Palette,
    base_x: usize,
    base_y: usize,
    cols: usize,
    num_chars: usize,
    cell_width: usize,
    cell_height: usize,
    char_width: usize,
    char_height: usize,
    fg_color: Rgb<u8>,
    bg_color: Rgb<u8>,
) {
    for idx in 0..num_chars {
        let col = idx % cols;
        let row = idx / cols;

        let cell_x = base_x + col * cell_width;
        let cell_y = base_y + row * cell_height;

        // Gap between cells (grid spacing)
        let cell_gap = 4;

        // Padding for top and bottom of character box
        let box_padding_top = 5;
        let box_padding_bottom = 5;

        // Calculate the character box size (excluding the gap)
        let box_width = cell_width.saturating_sub(cell_gap);
        let label_height = 36; // Space for labels (index + unicode)
        let box_height = box_padding_top + char_height + label_height + 10 + box_padding_bottom;

        // Draw cell background (individual box with gap)
        for y in 0..box_height {
            for x in 0..box_width {
                let img_x = (cell_x + x) as u32;
                let img_y = (cell_y + y) as u32;
                if img_x < img.width() && img_y < img.height() {
                    img.put_pixel(img_x, img_y, bg_color);
                }
            }
        }

        // Position character in the cell with top padding
        let char_x = cell_x + (cell_width - char_width) / 2;
        let char_y = cell_y + box_padding_top;

        // Draw box around the character
        let box_color = Rgb([128, 128, 128]); // Gray border
        // Top border
        for x in (char_x.saturating_sub(2))..(char_x + char_width + 2) {
            let img_y = (char_y.saturating_sub(2)) as u32;
            if (x as u32) < img.width() && img_y < img.height() {
                img.put_pixel(x as u32, img_y, box_color);
            }
        }
        // Bottom border
        for x in (char_x.saturating_sub(2))..(char_x + char_width + 2) {
            let img_y = (char_y + char_height + 1) as u32;
            if (x as u32) < img.width() && img_y < img.height() {
                img.put_pixel(x as u32, img_y, box_color);
            }
        }
        // Left border
        for y in (char_y.saturating_sub(2))..(char_y + char_height + 2) {
            let img_x = (char_x.saturating_sub(2)) as u32;
            if img_x < img.width() && (y as u32) < img.height() {
                img.put_pixel(img_x, y as u32, box_color);
            }
        }
        // Right border
        for y in (char_y.saturating_sub(2))..(char_y + char_height + 2) {
            let img_x = (char_x + char_width + 1) as u32;
            if img_x < img.width() && (y as u32) < img.height() {
                img.put_pixel(img_x, y as u32, box_color);
            }
        }

        // Render the character
        let ch = idx as u8 as char;
        let glyph = font.get_glyph_utf8(ch);

        for gy in 0..char_height {
            for gx in 0..char_width {
                let pixel_x = (char_x + gx) as u32;
                let pixel_y = (char_y + gy) as u32;

                if pixel_x < img.width() && pixel_y < img.height() {
                    let intensity = glyph[gy * char_width + gx];

                    // Blend foreground and background based on intensity (0-255)
                    let color = if intensity == 0 {
                        bg_color
                    } else if intensity == 255 {
                        fg_color
                    } else {
                        // Anti-aliasing: blend foreground and background
                        let alpha = intensity as f32 / 255.0;
                        let r = (fg_color.0[0] as f32 * alpha + bg_color.0[0] as f32 * (1.0 - alpha)) as u8;
                        let g = (fg_color.0[1] as f32 * alpha + bg_color.0[1] as f32 * (1.0 - alpha)) as u8;
                        let b = (fg_color.0[2] as f32 * alpha + bg_color.0[2] as f32 * (1.0 - alpha)) as u8;
                        Rgb([r, g, b])
                    };

                    img.put_pixel(pixel_x, pixel_y, color);
                }
            }
        }

        // Render labels below the character with large gap
        let label_y = cell_y + box_padding_top + char_height + 10;

        // Index label (decimal)
        let idx_label = format!("{}", idx);
        render_text_small(font, img, cell_x + 4, label_y, &idx_label, fg_color, bg_color, 1.0);

        // Unicode label (hex) - larger gap between the two labels
        let unicode_label = format!("U+{:02X}", idx);
        render_text_small(font, img, cell_x + 4, label_y + 18, &unicode_label, fg_color, bg_color, 0.9);
    }
}

fn render_text(
    font: &Font,
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    x: usize,
    y: usize,
    text: &str,
    fg_color: Rgb<u8>,
    _bg_color: Rgb<u8>,
    scale: f32,
) {
    let char_width = font.width();
    let char_height = font.height();
    let scaled_width = (char_width as f32 * scale) as usize;

    for (char_idx, c) in text.chars().enumerate() {
        let char_x = x + char_idx * scaled_width;
        let glyph = font.get_glyph_utf8(c);

        for py in 0..char_height {
            for px in 0..char_width {
                let glyph_idx = py * char_width + px;
                if glyph_idx < glyph.len() {
                    let intensity = glyph[glyph_idx];
                    if intensity > 0 {
                        // Blend color based on intensity for anti-aliasing
                        let color = if intensity == 255 {
                            fg_color
                        } else {
                            let alpha = intensity as f32 / 255.0;
                            let bg = Rgb([235u8, 235u8, 235u8]); // Background color
                            let r = (fg_color.0[0] as f32 * alpha + bg.0[0] as f32 * (1.0 - alpha)) as u8;
                            let g = (fg_color.0[1] as f32 * alpha + bg.0[1] as f32 * (1.0 - alpha)) as u8;
                            let b = (fg_color.0[2] as f32 * alpha + bg.0[2] as f32 * (1.0 - alpha)) as u8;
                            Rgb([r, g, b])
                        };

                        // Scale the pixel
                        for sy in 0..(scale as usize) {
                            for sx in 0..(scale as usize) {
                                let img_x = (char_x + px * (scale as usize) + sx) as u32;
                                let img_y = (y + py * (scale as usize) + sy) as u32;
                                if img_x < img.width() && img_y < img.height() {
                                    img.put_pixel(img_x, img_y, color);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_text_small(
    font: &Font,
    img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    x: usize,
    y: usize,
    text: &str,
    fg_color: Rgb<u8>,
    bg_color: Rgb<u8>,
    scale: f32,
) {
    let char_width = font.width();
    let char_height = font.height();
    let scaled_width = ((char_width as f32 * scale) as usize).max(1);
    let scaled_height = ((char_height as f32 * scale) as usize).max(1);

    for (char_idx, c) in text.chars().enumerate() {
        let char_x = x + char_idx * scaled_width;
        let glyph = font.get_glyph_utf8(c);

        // Draw background
        for py in 0..scaled_height {
            for px in 0..scaled_width {
                let img_x = (char_x + px) as u32;
                let img_y = (y + py) as u32;
                if img_x < img.width() && img_y < img.height() {
                    img.put_pixel(img_x, img_y, bg_color);
                }
            }
        }

        // Draw character with scaling and anti-aliasing
        for py in 0..char_height {
            for px in 0..char_width {
                let glyph_idx = py * char_width + px;
                if glyph_idx < glyph.len() {
                    let intensity = glyph[glyph_idx];
                    if intensity > 0 {
                        // Blend color based on intensity for anti-aliasing
                        let color = if intensity == 255 {
                            fg_color
                        } else {
                            let alpha = intensity as f32 / 255.0;
                            let r = (fg_color.0[0] as f32 * alpha + bg_color.0[0] as f32 * (1.0 - alpha)) as u8;
                            let g = (fg_color.0[1] as f32 * alpha + bg_color.0[1] as f32 * (1.0 - alpha)) as u8;
                            let b = (fg_color.0[2] as f32 * alpha + bg_color.0[2] as f32 * (1.0 - alpha)) as u8;
                            Rgb([r, g, b])
                        };

                        let scaled_x = (px as f32 * scale) as usize;
                        let scaled_y = (py as f32 * scale) as usize;

                        if scaled_x < scaled_width && scaled_y < scaled_height {
                            let img_x = (char_x + scaled_x) as u32;
                            let img_y = (y + scaled_y) as u32;
                            if img_x < img.width() && img_y < img.height() {
                                img.put_pixel(img_x, img_y, color);
                            }
                        }
                    }
                }
            }
        }
    }
}
