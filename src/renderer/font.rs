// Include the auto-generated embedded fonts
include!(concat!(env!("OUT_DIR"), "/embedded_fonts.rs"));

pub struct Font {
    width: usize,
    height: usize,
    glyphs: Vec<Vec<bool>>, // Bitmap data for each of 256 characters
}

/// Map UTF-8 characters to their Code Page 437 / ASCII equivalents
fn map_utf8_to_cp437(ch: char) -> u8 {
    match ch {
        // Box-drawing characters (single line)
        '─' => 0xC4, // BOX DRAWINGS LIGHT HORIZONTAL
        '│' => 0xB3, // BOX DRAWINGS LIGHT VERTICAL
        '┌' => 0xDA, // BOX DRAWINGS LIGHT DOWN AND RIGHT
        '┐' => 0xBF, // BOX DRAWINGS LIGHT DOWN AND LEFT
        '└' => 0xC0, // BOX DRAWINGS LIGHT UP AND RIGHT
        '┘' => 0xD9, // BOX DRAWINGS LIGHT UP AND LEFT
        '├' => 0xC3, // BOX DRAWINGS LIGHT VERTICAL AND RIGHT
        '┤' => 0xB4, // BOX DRAWINGS LIGHT VERTICAL AND LEFT
        '┬' => 0xC2, // BOX DRAWINGS LIGHT DOWN AND HORIZONTAL
        '┴' => 0xC1, // BOX DRAWINGS LIGHT UP AND HORIZONTAL
        '┼' => 0xC5, // BOX DRAWINGS LIGHT VERTICAL AND HORIZONTAL

        // Box-drawing characters (double line)
        '═' => 0xCD, // BOX DRAWINGS DOUBLE HORIZONTAL
        '║' => 0xBA, // BOX DRAWINGS DOUBLE VERTICAL
        '╔' => 0xC9, // BOX DRAWINGS DOUBLE DOWN AND RIGHT
        '╗' => 0xBB, // BOX DRAWINGS DOUBLE DOWN AND LEFT
        '╚' => 0xC8, // BOX DRAWINGS DOUBLE UP AND RIGHT
        '╝' => 0xBC, // BOX DRAWINGS DOUBLE UP AND LEFT
        '╠' => 0xCC, // BOX DRAWINGS DOUBLE VERTICAL AND RIGHT
        '╣' => 0xB9, // BOX DRAWINGS DOUBLE VERTICAL AND LEFT
        '╦' => 0xCB, // BOX DRAWINGS DOUBLE DOWN AND HORIZONTAL
        '╩' => 0xCA, // BOX DRAWINGS DOUBLE UP AND HORIZONTAL
        '╬' => 0xCE, // BOX DRAWINGS DOUBLE VERTICAL AND HORIZONTAL

        // Mixed single/double box drawing
        '╒' => 0xD5, // BOX DRAWINGS DOWN SINGLE AND RIGHT DOUBLE
        '╓' => 0xD6, // BOX DRAWINGS DOWN DOUBLE AND RIGHT SINGLE
        '╕' => 0xB7, // BOX DRAWINGS DOWN SINGLE AND LEFT DOUBLE
        '╖' => 0xB6, // BOX DRAWINGS DOWN DOUBLE AND LEFT SINGLE
        '╘' => 0xD4, // BOX DRAWINGS UP SINGLE AND RIGHT DOUBLE
        '╙' => 0xD3, // BOX DRAWINGS UP DOUBLE AND RIGHT SINGLE
        '╛' => 0xBE, // BOX DRAWINGS UP SINGLE AND LEFT DOUBLE
        '╜' => 0xBD, // BOX DRAWINGS UP DOUBLE AND LEFT SINGLE

        // Block elements
        '█' => 0xDB, // FULL BLOCK
        '▓' => 0xB2, // DARK SHADE
        '▒' => 0xB1, // MEDIUM SHADE
        '░' => 0xB0, // LIGHT SHADE
        '▀' => 0xDF, // UPPER HALF BLOCK
        '▄' => 0xDC, // LOWER HALF BLOCK
        '▌' => 0xDD, // LEFT HALF BLOCK
        '▐' => 0xDE, // RIGHT HALF BLOCK

        // Other special characters
        '•' => 0x07, // BULLET
        '◘' => 0x08, // INVERSE BULLET
        '○' => 0x09, // WHITE CIRCLE
        '◙' => 0x0A, // INVERSE WHITE CIRCLE
        '♂' => 0x0B, // MALE SIGN
        '♀' => 0x0C, // FEMALE SIGN
        '♪' => 0x0D, // EIGHTH NOTE
        '♫' => 0x0E, // BEAMED EIGHTH NOTES
        '☼' => 0x0F, // WHITE SUN WITH RAYS
        '►' => 0x10, // BLACK RIGHT-POINTING POINTER
        '◄' => 0x11, // BLACK LEFT-POINTING POINTER
        '↕' => 0x12, // UP DOWN ARROW
        '‼' => 0x13, // DOUBLE EXCLAMATION MARK
        '¶' => 0x14, // PILCROW SIGN
        '§' => 0x15, // SECTION SIGN
        '▬' => 0x16, // BLACK RECTANGLE
        '↨' => 0x17, // UP DOWN ARROW WITH BASE
        '↑' => 0x18, // UPWARDS ARROW
        '↓' => 0x19, // DOWNWARDS ARROW
        '→' => 0x1A, // RIGHTWARDS ARROW
        '←' => 0x1B, // LEFTWARDS ARROW
        '∟' => 0x1C, // RIGHT ANGLE
        '↔' => 0x1D, // LEFT RIGHT ARROW
        '▲' => 0x1E, // BLACK UP-POINTING TRIANGLE
        '▼' => 0x1F, // BLACK DOWN-POINTING TRIANGLE

        // Fallback for ASCII range
        _ if (ch as u32) < 256 => ch as u8,

        // Fallback for unknown characters
        _ => b'?',
    }
}

impl Font {
    pub fn load(name: Option<&str>) -> Self {
        // Use specified font or default
        let font_name = name.unwrap_or(DEFAULT_FONT_NAME);

        // Look up font in embedded fonts
        if let Some(&font_data) = EMBEDDED_FONTS.get(font_name) {
            match Self::parse_fd_font(font_data) {
                Ok(font) => return font,
                Err(e) => eprintln!("Warning: Failed to parse font '{}': {}", font_name, e),
            }
        } else {
            eprintln!("Warning: Font '{}' not found. Available fonts: {:?}", font_name, FONT_NAMES);
        }

        // Fall back to default font if specified font failed
        if font_name != DEFAULT_FONT_NAME {
            if let Some(&font_data) = EMBEDDED_FONTS.get(DEFAULT_FONT_NAME) {
                if let Ok(font) = Self::parse_fd_font(font_data) {
                    return font;
                }
            }
        }

        // Last resort: fallback font
        Self::fallback_font()
    }

    /// Get list of available embedded font names
    pub fn available_fonts() -> &'static [&'static str] {
        FONT_NAMES
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn get_glyph(&self, ch: u8) -> &[bool] {
        &self.glyphs[ch as usize]
    }

    /// Get glyph for a character, mapping UTF-8 to CP437 if needed
    pub fn get_glyph_utf8(&self, ch: char) -> &[bool] {
        let mapped = map_utf8_to_cp437(ch);
        &self.glyphs[mapped as usize]
    }

    /// Render a string of text onto a canvas with scaling
    pub fn render_string(&self, canvas: &mut crate::renderer::Canvas, x: i32, y: i32, text: &str, fg_color: u8, bg_color: u8, size: f32) {
        let scaled_width = (self.width as f32 * size) as i32;

        for (char_idx, ch) in text.chars().enumerate() {
            let ch_code = map_utf8_to_cp437(ch);
            let char_x = x + (char_idx as i32 * scaled_width);

            self.render_character(canvas, char_x, y, ch_code, fg_color, bg_color, size);
        }
    }

    /// Render a single character onto a canvas with scaling
    /// Only renders foreground pixels, leaving background transparent
    fn render_character(&self, canvas: &mut crate::renderer::Canvas, x: i32, y: i32, ch: u8, fg_color: u8, _bg_color: u8, size: f32) {
        let glyph = self.get_glyph(ch);
        let scaled_width = (self.width as f32 * size).round() as usize;
        let scaled_height = (self.height as f32 * size).round() as usize;

        for sy in 0..scaled_height {
            for sx in 0..scaled_width {
                // Map scaled pixel back to source glyph pixel (nearest neighbor)
                let glyph_x = ((sx as f32) / size) as usize;
                let glyph_y = ((sy as f32) / size) as usize;

                if glyph_x < self.width && glyph_y < self.height {
                    let glyph_idx = glyph_y * self.width + glyph_x;
                    let is_foreground = glyph[glyph_idx];

                    // Only render foreground pixels (the actual character)
                    if is_foreground {
                        let pixel_x = x + sx as i32;
                        let pixel_y = y + sy as i32;

                        if pixel_x >= 0 && pixel_y >= 0 &&
                           pixel_x < canvas.width() as i32 && pixel_y < canvas.height() as i32 {
                            canvas.set_pixel(pixel_x as usize, pixel_y as usize, fg_color);
                        }
                    }
                }
            }
        }
    }

    // Parse .fd font format
    fn parse_fd_font(data: &str) -> Result<Self, String> {
        let lines: Vec<&str> = data.lines().collect();
        let mut height = 16;
        let mut width = 9;
        let mut char_start_idx = 0;

        // Parse header
        for (idx, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if line.starts_with("height") {
                height = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .ok_or("Invalid height")?;
            } else if line.starts_with("char") {
                // Found first character, remember where it starts
                char_start_idx = idx;
                break;
            }
        }

        // Now parse characters starting from char_start_idx
        let mut glyphs = vec![vec![false; width * height]; 256];
        let mut current_char: Option<usize> = None;
        let mut current_width = width;
        let mut bitmap_lines: Vec<String> = Vec::new();

        for line in &lines[char_start_idx..] {
            let line = line.trim();

            if line.starts_with("char") {
                // Save previous character if we have one
                if let Some(ch_idx) = current_char {
                    if ch_idx < 256 && bitmap_lines.len() == height {
                        glyphs[ch_idx] = Self::parse_bitmap(&bitmap_lines, current_width, height);
                    }
                }

                // Start new character
                current_char = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok());
                bitmap_lines.clear();
                current_width = width; // Reset to default
            } else if line.starts_with("width") {
                current_width = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(width);
            } else if !line.is_empty() && !line.starts_with('#') && (line.contains('x') || line.contains('.')) {
                // This is a bitmap line
                bitmap_lines.push(line.to_string());
            }
        }

        // Save last character
        if let Some(ch_idx) = current_char {
            if ch_idx < 256 && bitmap_lines.len() == height {
                glyphs[ch_idx] = Self::parse_bitmap(&bitmap_lines, current_width, height);
            }
        }

        Ok(Self {
            width,
            height,
            glyphs,
        })
    }

    fn parse_bitmap(lines: &[String], char_width: usize, height: usize) -> Vec<bool> {
        let mut bitmap = Vec::with_capacity(char_width * height);

        for line in lines {
            for (i, ch) in line.chars().enumerate() {
                if i >= char_width {
                    break;
                }
                bitmap.push(ch == 'x');
            }

            // Pad line if needed
            while bitmap.len() % char_width != 0 {
                bitmap.push(false);
            }
        }

        // Ensure we have exactly width * height pixels
        bitmap.resize(char_width * height, false);
        bitmap
    }

    // Fallback font in case parsing fails
    fn fallback_font() -> Self {
        let width = 8;
        let height = 16;
        let mut glyphs = Vec::with_capacity(256);

        for i in 0..=255 {
            let glyph = Self::generate_fallback_glyph(i, width, height);
            glyphs.push(glyph);
        }

        Self {
            width,
            height,
            glyphs,
        }
    }

    fn generate_fallback_glyph(ch: u8, width: usize, height: usize) -> Vec<bool> {
        let mut bitmap = vec![false; width * height];

        if ch >= 32 && ch < 127 {
            match ch {
                b' ' => {}
                b'#' => {
                    for pixel in &mut bitmap {
                        *pixel = true;
                    }
                }
                _ => {
                    for y in 2..height - 2 {
                        bitmap[y * width + 1] = true;
                        bitmap[y * width + width - 2] = true;
                    }
                    for x in 1..width - 1 {
                        bitmap[2 * width + x] = true;
                        bitmap[(height - 3) * width + x] = true;
                    }
                }
            }
        }

        bitmap
    }
}
