use anyhow::{Context, Result};
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use fontdue::{Font as FontdueFont, FontSettings};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

/// Convert a TrueType font to .fd bitmap font format
pub struct FontConverter {
    font: FontdueFont,
    fallback_fonts: Vec<FontdueFont>, // Fallback fonts for missing glyphs
    font_name: String,
    font_size: f32,
    char_width: usize,
    char_height: usize,
    y_offset_adjustment: i32, // Offset to adjust all characters so they fit in the tight bounding box
}

impl FontConverter {
    /// Load a TrueType font from system or file path
    /// font_size is the desired font size in pixels (used for rasterization)
    pub fn load(font_path_or_name: &str, font_size: usize) -> Result<Self> {
        let font_data = if std::path::Path::new(font_path_or_name).exists() {
            // Load from file path
            std::fs::read(font_path_or_name)
                .with_context(|| format!("Failed to read font file: {}", font_path_or_name))?
        } else {
            // Try to load from system fonts
            let source = SystemSource::new();
            let handle = source
                .select_best_match(&[FamilyName::Title(font_path_or_name.to_string())], &Properties::new())
                .with_context(|| format!("Failed to find system font: {}", font_path_or_name))?;

            handle
                .load()
                .with_context(|| format!("Failed to load font: {}", font_path_or_name))?
                .copy_font_data()
                .with_context(|| "Failed to copy font data")?
                .to_vec()
        };

        let font = FontdueFont::from_bytes(font_data.as_slice(), FontSettings::default())
            .map_err(|e| anyhow::anyhow!("Failed to parse font: {}", e))?;

        let font_name = font_path_or_name
            .split('/')
            .last()
            .unwrap_or(font_path_or_name)
            .trim_end_matches(".ttf")
            .trim_end_matches(".otf")
            .to_string();

        // Calculate optimal cell dimensions by finding actual bounding box of all characters
        // Sample all printable ASCII characters plus some common extended characters
        let mut sample_chars = Vec::new();

        // ASCII printable characters (32-126)
        for i in 32..=126 {
            sample_chars.push(char::from_u32(i).unwrap());
        }

        // Common extended ASCII and Unicode characters
        let extra_chars = ['Ñ', 'ñ', 'ü', 'ö', 'ä', '§', '±', '÷', '×',
                          '─', '│', '┌', '┐', '└', '┘', '█', '▓', '▒', '░',
                          '↑', '↓', '→', '←', '•', '○', '●', '◆'];
        sample_chars.extend_from_slice(&extra_chars);

        // Get font metrics for baseline calculation
        let line_metrics = font.horizontal_line_metrics(font_size as f32)
            .ok_or_else(|| anyhow::anyhow!("Failed to get font metrics"))?;

        let padding_top = 2.0;
        let baseline_offset = padding_top + line_metrics.ascent;

        // Find the actual bounding box by rendering all characters
        let mut min_top = i32::MAX;
        let mut max_bottom = i32::MIN;
        let mut max_width = 0;

        for &ch in &sample_chars {
            let (metrics, bitmap) = font.rasterize(ch, font_size as f32);

            // Skip empty characters
            if bitmap.is_empty() {
                continue;
            }

            // Calculate where this character would be positioned
            let glyph_y_offset = baseline_offset.round() as i32 - metrics.ymin - metrics.height as i32;
            let glyph_bottom = glyph_y_offset + metrics.height as i32;

            // Track bounds
            min_top = min_top.min(glyph_y_offset);
            max_bottom = max_bottom.max(glyph_bottom);
            max_width = max_width.max(metrics.width);
        }

        // Calculate final cell dimensions from actual bounds
        let char_height = (max_bottom - min_top).max(1) as usize;
        let char_width = max_width.max(1);

        eprintln!("Font converter: font_size={}px, cell={}x{} (tight fit), ascent={:.1}px, descent={:.1}px, bounds={}..{}",
            font_size, char_width, char_height, line_metrics.ascent, line_metrics.descent, min_top, max_bottom);

        // Load fallback fonts automatically using fontconfig
        let fallback_fonts = Self::load_fallback_fonts(font_size)?;

        if !fallback_fonts.is_empty() {
            eprintln!("Loaded {} fallback fonts for missing glyphs", fallback_fonts.len());
        }

        Ok(Self {
            font,
            fallback_fonts,
            font_name,
            font_size: font_size as f32,
            char_width,
            char_height,
            y_offset_adjustment: min_top, // Store the min_top so we can adjust all glyphs
        })
    }

    /// Load fallback fonts using fontconfig to find fonts that cover missing glyphs
    fn load_fallback_fonts(font_size: usize) -> Result<Vec<FontdueFont>> {
        let mut fallback_fonts = Vec::new();

        // Common fallback fonts that provide good Unicode coverage including emoji
        let fallback_names = [
            "Symbola",           // Comprehensive symbol and emoji coverage
            "Noto Color Emoji",  // Color emoji
            "Noto Emoji",        // Black & white emoji
            "DejaVu Sans",       // Good Unicode coverage
        ];

        for fallback_name in &fallback_names {
            if let Ok(font_data) = Self::try_load_font_data(fallback_name) {
                if let Ok(font) = FontdueFont::from_bytes(font_data.as_slice(), FontSettings::default()) {
                    eprintln!("  Loaded fallback font: {}", fallback_name);
                    fallback_fonts.push(font);
                }
            }
        }

        Ok(fallback_fonts)
    }

    /// Try to load font data, returns error if font not found
    fn try_load_font_data(font_name: &str) -> Result<Vec<u8>> {
        let source = SystemSource::new();
        let handle = source
            .select_best_match(&[FamilyName::Title(font_name.to_string())], &Properties::new())
            .with_context(|| format!("Font '{}' not found", font_name))?;

        Ok(handle
            .load()
            .with_context(|| format!("Failed to load font: {}", font_name))?
            .copy_font_data()
            .with_context(|| "Failed to copy font data")?
            .to_vec())
    }

    /// Rasterize a character using primary font with fallback to other fonts
    fn rasterize_with_fallback(&self, ch: char) -> (fontdue::Metrics, Vec<u8>) {
        let ch_code = ch as u32;

        // For emoji range (U+2700-U+27BF Dingbats, U+1F300+ emoji), skip primary font and use fallback
        // Most monospace programming fonts don't have proper emoji glyphs
        let is_emoji_range = (ch_code >= 0x2700 && ch_code <= 0x27BF) ||  // Dingbats
                             (ch_code >= 0x1F300 && ch_code <= 0x1F6FF) || // Misc Symbols and Pictographs
                             (ch_code >= 0x1F600 && ch_code <= 0x1F64F) || // Emoticons
                             (ch_code >= 0x1F900 && ch_code <= 0x1F9FF) || // Supplemental Symbols
                             (ch_code >= 0x1FA00 && ch_code <= 0x1FA6F);   // Extended Pictographs

        if !is_emoji_range {
            // Try primary font first for non-emoji
            let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);

            // Check if we got a valid glyph (has pixels)
            if bitmap.iter().any(|&pixel| pixel > 10) {
                return (metrics, bitmap);
            }
        }

        // Try fallback fonts for emoji or if primary didn't have it
        for fallback_font in &self.fallback_fonts {
            let (fb_metrics, fb_bitmap) = fallback_font.rasterize(ch, self.font_size);
            if fb_bitmap.iter().any(|&pixel| pixel > 10) {
                return (fb_metrics, fb_bitmap);
            }
        }

        // No font had the glyph - return empty from primary font
        let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);
        (metrics, bitmap)
    }

    /// Convert the font to .fd format with UTF-8 character support
    ///
    /// If char_map is provided, only those characters will be included.
    /// Otherwise, generates a default set with:
    /// - Basic ASCII (0-127)
    /// - Extended ASCII / CP437 (128-255)
    /// - Common Unicode characters (box drawing, blocks, etc.)
    pub fn convert_to_fd(&self, output_path: &PathBuf, char_map: Option<Vec<char>>) -> Result<()> {
        let mut file = File::create(output_path)
            .with_context(|| format!("Failed to create output file: {:?}", output_path))?;

        // Write header
        writeln!(file, "# .fd font description generated by ttyvid font converter")?;
        writeln!(file)?;
        writeln!(file, "facename {}", self.font_name)?;
        writeln!(file, "copyright Converted from TrueType by ttyvid")?;
        writeln!(file)?;
        writeln!(file, "pointsize {}", self.char_height)?;
        writeln!(file)?;
        writeln!(file, "height {}", self.char_height)?;
        writeln!(file, "width {}", self.char_width)?;
        writeln!(file, "ascent {}", self.char_height - 2)?;
        writeln!(file, "inleading 0")?;
        writeln!(file, "exleading 0")?;
        writeln!(file)?;

        // Determine which characters to render
        let characters = if let Some(char_list) = char_map {
            char_list
        } else {
            self.generate_default_character_set()
        };

        // Calculate baseline offset using proper font metrics (same as TrueType renderer)
        let padding_top = 2.0;

        // Use fontdue's horizontal_line_metrics to get baseline position
        let line_metrics = self.font.horizontal_line_metrics(self.font_size)
            .ok_or_else(|| anyhow::anyhow!("Failed to get font metrics"))?;

        // baseline_offset is where the baseline sits in our cell
        // fontdue gives us ascent (distance from baseline to top) and descent (distance from baseline to bottom)
        let baseline_offset = padding_top + line_metrics.ascent;

        // For ASCII/CP437 compatibility, maintain the index = character code relationship
        // Characters 0-255 MUST be at their corresponding indices
        // Additional Unicode characters go at indices 256+

        let mut char_index_map = HashMap::new();
        let mut max_index = 0;

        // First pass: determine which characters to render and assign indices
        for ch in characters.iter() {
            let ch_code = *ch as u32;

            // Check if font can render this character
            let (_metrics, bitmap) = self.font.rasterize(*ch, self.font_size);
            if bitmap.is_empty() && *ch != ' ' && ch_code >= 32 {
                // Skip characters that failed to rasterize (except space and control chars)
                continue;
            }

            // Assign index based on character code
            // For 0-255: index = character code (for CP437 compatibility)
            // For 256+: assign sequential indices
            let idx = if ch_code < 256 {
                ch_code as usize
            } else {
                // Find next available index after 255
                max_index = max_index.max(255);
                max_index += 1;
                max_index
            };

            char_index_map.insert(*ch, idx);
            max_index = max_index.max(idx);
        }

        // Write charset count (max index + 1)
        writeln!(file, "charset {}", max_index + 1)?;
        writeln!(file)?;

        // Render each character in order of index
        let mut index_to_char: Vec<(usize, char)> = char_index_map.iter()
            .map(|(ch, idx)| (*idx, *ch))
            .collect();
        index_to_char.sort_by_key(|(idx, _)| *idx);

        // Track bitmap data for deduplication
        // Map from bitmap data (WITHOUT newlines) -> first character index that has this bitmap
        let mut bitmap_to_index: HashMap<String, usize> = HashMap::new();

        for (idx, ch) in index_to_char.iter() {
            // Write character header
            writeln!(file, "char {}", idx)?;
            writeln!(file, "unicode 0x{:04X}", *ch as u32)?;
            writeln!(file, "width {}", self.char_width)?;

            // Check if this is a control character or non-printable
            let is_control = (*ch as u32) < 32 || *ch == '\u{007F}';

            // Generate bitmap data as string
            let mut bitmap_data = String::new();
            let mut is_blank = true;

            if is_control {
                // Render control characters as empty (all dots)
                for _y in 0..self.char_height {
                    for _x in 0..self.char_width {
                        bitmap_data.push('.');
                    }
                    bitmap_data.push('\n');
                }
            } else {
                // Normal character rendering - try primary font first, then fallbacks
                let (metrics, bitmap) = self.rasterize_with_fallback(*ch);

                // Use the font's natural horizontal offset (xmin)
                let glyph_x_offset = metrics.xmin.max(0) as usize;

                // Vertical positioning using proper baseline formula:
                // fontdue's ymin is distance from baseline to BOTTOM of glyph
                // To get TOP of glyph: baseline - ymin - height
                // Then adjust by y_offset_adjustment to fit in tight bounding box
                let glyph_y_offset = baseline_offset.round() as i32 - metrics.ymin - metrics.height as i32 - self.y_offset_adjustment;

                // Generate bitmap with anti-aliasing support (stepped intensity levels)
                // Map grayscale values to characters:
                // . = 0% (blank/background)
                // 1 = 10%, 2 = 20%, 3 = 30%, 4 = 40%, 5 = 50%,
                // 6 = 60%, 7 = 70%, 8 = 80%, 9 = 90%
                // x = 100% (solid/foreground)
                for y in 0..self.char_height {
                    for x in 0..self.char_width {
                        // Calculate if this cell pixel maps to a source glyph pixel
                        // Using the same logic as TrueType renderer
                        let src_x_signed = x as i32 - glyph_x_offset as i32;
                        let src_y_signed = y as i32 - glyph_y_offset;

                        let pixel_value = if src_x_signed >= 0 && src_x_signed < metrics.width as i32
                                             && src_y_signed >= 0 && src_y_signed < metrics.height as i32 {
                            let src_idx = (src_y_signed as usize) * metrics.width + (src_x_signed as usize);
                            bitmap.get(src_idx).copied().unwrap_or(0)
                        } else {
                            0
                        };

                        // Map grayscale value (0-255) to intensity character
                        // Using stepped thresholds for better anti-aliasing
                        let ch = match pixel_value {
                            0..=12 => '.',      // 0-5% -> blank
                            13..=38 => '1',     // 5-15% -> 10%
                            39..=63 => '2',     // 15-25% -> 20%
                            64..=89 => '3',     // 25-35% -> 30%
                            90..=114 => '4',    // 35-45% -> 40%
                            115..=140 => '5',   // 45-55% -> 50%
                            141..=165 => '6',   // 55-65% -> 60%
                            166..=191 => '7',   // 65-75% -> 70%
                            192..=217 => '8',   // 75-85% -> 80%
                            218..=242 => '9',   // 85-95% -> 90%
                            243..=255 => 'x',   // 95-100% -> solid
                        };

                        if ch != '.' {
                            is_blank = false;
                        }
                        bitmap_data.push(ch);
                    }
                    bitmap_data.push('\n');
                }
            }

            // Check if this bitmap is blank (all dots)
            if is_blank {
                writeln!(file, "blank true")?;
            } else {
                // For deduplication, create a key WITHOUT newlines (just the pixel data)
                let bitmap_key: String = bitmap_data.chars().filter(|&c| c != '\n').collect();

                // Check if we've seen this exact bitmap before
                if let Some(&original_idx) = bitmap_to_index.get(&bitmap_key) {
                    // This is a duplicate - reference the original
                    writeln!(file, "sameas {}", original_idx)?;
                } else {
                    // New unique bitmap - store it and write the data
                    bitmap_to_index.insert(bitmap_key, *idx);
                    write!(file, "{}", bitmap_data)?;
                }
            }
            writeln!(file)?;
        }

        // Write UTF-8 character mapping section at the end
        writeln!(file, "# UTF-8 Character Mapping")?;
        writeln!(file, "# Format: char_index unicode_codepoint utf8_char description")?;
        for (ch, idx) in char_index_map.iter() {
            writeln!(
                file,
                "# {} U+{:04X} {} {}",
                idx,
                *ch as u32,
                ch,
                self.get_char_description(*ch)
            )?;
        }

        Ok(())
    }

    /// Generate a comprehensive character set including emoji and common Unicode
    /// Only includes characters that render as distinct glyphs (not empty boxes)
    fn generate_default_character_set(&self) -> Vec<char> {
        let mut chars = Vec::new();

        // All 256 ASCII/extended ASCII characters (0-255)
        // We need all of them to maintain proper indexing in .fd format
        for i in 0..256 {
            chars.push(char::from_u32(i).unwrap());
        }

        // Define specific Unicode blocks that are commonly used in terminals
        // This is much more efficient than testing all 65k+ code points
        let unicode_ranges = [
            // Latin Extended-A (U+0100-U+017F) - À, Ñ, ü, etc.
            (0x0100, 0x017F, "Latin Extended-A"),

            // Latin Extended-B (U+0180-U+024F)
            (0x0180, 0x024F, "Latin Extended-B"),

            // Greek and Coptic (U+0370-U+03FF) - α, β, γ, etc.
            (0x0370, 0x03FF, "Greek and Coptic"),

            // Cyrillic (U+0400-U+04FF) - Д, Ж, И, etc.
            (0x0400, 0x04FF, "Cyrillic"),

            // General Punctuation (U+2000-U+206F) - —, †, ‡, etc.
            (0x2000, 0x206F, "General Punctuation"),

            // Currency Symbols (U+20A0-U+20CF) - €, ¥, £, etc.
            (0x20A0, 0x20CF, "Currency Symbols"),

            // Letterlike Symbols (U+2100-U+214F) - ™, ℓ, №, etc.
            (0x2100, 0x214F, "Letterlike Symbols"),

            // Arrows (U+2190-U+21FF) - ←, ↑, →, ↓, etc.
            (0x2190, 0x21FF, "Arrows"),

            // Mathematical Operators (U+2200-U+22FF) - ∀, ∂, ∈, etc.
            (0x2200, 0x22FF, "Mathematical Operators"),

            // Box Drawing (U+2500-U+257F) - ─, │, ┌, etc.
            (0x2500, 0x257F, "Box Drawing"),

            // Block Elements (U+2580-U+259F) - █, ▓, ▒, etc.
            (0x2580, 0x259F, "Block Elements"),

            // Geometric Shapes (U+25A0-U+25FF) - ■, ●, ◆, etc.
            (0x25A0, 0x25FF, "Geometric Shapes"),

            // Miscellaneous Symbols (U+2600-U+26FF) - ☀, ☁, ☂, ★, ☆, ♠, ♣, ♥, ♦, etc.
            (0x2600, 0x26FF, "Miscellaneous Symbols"),

            // Dingbats (U+2700-U+27BF) - ✓, ✗, ✨ (SPARKLES!), ❤, etc.
            (0x2700, 0x27BF, "Dingbats"),

            // Braille Patterns (U+2800-U+28FF)
            (0x2800, 0x28FF, "Braille Patterns"),

            // Supplemental Arrows-B (U+2900-U+297F)
            (0x2900, 0x297F, "Supplemental Arrows-B"),

            // Miscellaneous Mathematical Symbols-A (U+27C0-U+27EF)
            (0x27C0, 0x27EF, "Miscellaneous Mathematical Symbols-A"),

            // Miscellaneous Mathematical Symbols-B (U+2980-U+29FF)
            (0x2980, 0x29FF, "Miscellaneous Mathematical Symbols-B"),

            // CJK Symbols and Punctuation (U+3000-U+303F)
            (0x3000, 0x303F, "CJK Symbols and Punctuation"),

            // Emoji and Pictographs (U+1F300-U+1F6FF) - 🌍, 🔥, 💻, 🚀, etc.
            (0x1F300, 0x1F6FF, "Miscellaneous Symbols and Pictographs"),

            // Emoticons (U+1F600-U+1F64F) - 😀, 😂, 😍, etc.
            (0x1F600, 0x1F64F, "Emoticons"),

            // Supplemental Symbols and Pictographs (U+1F900-U+1F9FF)
            (0x1F900, 0x1F9FF, "Supplemental Symbols and Pictographs"),

            // Extended Pictographs (U+1FA00-U+1FA6F)
            (0x1FA00, 0x1FA6F, "Extended Pictographs"),
        ];

        let mut added_count = 0;
        for (start, end, name) in unicode_ranges {
            let mut block_count = 0;
            for code_point in start..=end {
                if let Some(ch) = char::from_u32(code_point) {
                    let (_metrics, bitmap) = self.font.rasterize(ch, self.font_size);

                    // Only include if the glyph has actual pixels (not an empty box)
                    // Check if bitmap has any non-zero pixels
                    let has_pixels = bitmap.iter().any(|&pixel| pixel > 10);

                    if has_pixels {
                        chars.push(ch);
                        block_count += 1;
                    }
                }
            }
            if block_count > 0 {
                eprintln!("  Added {} glyphs from {}", block_count, name);
            }
            added_count += block_count;
        }

        eprintln!("Font enumeration: {} total characters (256 ASCII + {} Unicode)", chars.len(), added_count);

        chars
    }

    /// Get a human-readable description for a character
    fn get_char_description(&self, ch: char) -> &'static str {
        match ch {
            ' ' => "SPACE",
            '\n' => "NEWLINE",
            '\t' => "TAB",
            '─' => "BOX DRAWINGS LIGHT HORIZONTAL",
            '│' => "BOX DRAWINGS LIGHT VERTICAL",
            '┌' => "BOX DRAWINGS LIGHT DOWN AND RIGHT",
            '┐' => "BOX DRAWINGS LIGHT DOWN AND LEFT",
            '└' => "BOX DRAWINGS LIGHT UP AND RIGHT",
            '┘' => "BOX DRAWINGS LIGHT UP AND LEFT",
            '█' => "FULL BLOCK",
            '░' => "LIGHT SHADE",
            '▒' => "MEDIUM SHADE",
            '▓' => "DARK SHADE",
            _ if ch.is_ascii_alphanumeric() => "ASCII ALPHANUMERIC",
            _ if ch.is_ascii_punctuation() => "ASCII PUNCTUATION",
            _ if ch.is_ascii() => "ASCII CHARACTER",
            _ => "UNICODE CHARACTER",
        }
    }
}

/// Helper function to list available system fonts
pub fn list_system_fonts() -> Result<Vec<String>> {
    let source = SystemSource::new();
    let families = source.all_families()?;
    Ok(families)
}
