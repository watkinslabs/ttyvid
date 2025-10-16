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

        Ok(Self {
            font,
            font_name,
            font_size: font_size as f32,
            char_width,
            char_height,
            y_offset_adjustment: min_top, // Store the min_top so we can adjust all glyphs
        })
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

        for (idx, ch) in index_to_char.iter() {
            // Write character header
            writeln!(file, "char {}", idx)?;
            writeln!(file, "unicode 0x{:04X}", *ch as u32)?;
            writeln!(file, "width {}", self.char_width)?;

            // Check if this is a control character or non-printable
            let is_control = (*ch as u32) < 32 || *ch == '\u{007F}';

            if is_control {
                // Render control characters as empty (all dots)
                for _y in 0..self.char_height {
                    for _x in 0..self.char_width {
                        write!(file, ".")?;
                    }
                    writeln!(file)?;
                }
            } else {
                // Normal character rendering
                let (metrics, bitmap) = self.font.rasterize(*ch, self.font_size);

                // Calculate positioning EXACTLY like TrueType renderer
                // Horizontal: center the glyph in the cell
                let glyph_x_offset = (self.char_width.saturating_sub(metrics.width)) / 2;

                // Vertical positioning using proper baseline formula:
                // fontdue's ymin is distance from baseline to BOTTOM of glyph
                // To get TOP of glyph: baseline - ymin - height
                // Then adjust by y_offset_adjustment to fit in tight bounding box
                let glyph_y_offset = baseline_offset.round() as i32 - metrics.ymin - metrics.height as i32 - self.y_offset_adjustment;

                // Write bitmap with anti-aliasing support (stepped intensity levels)
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

                        write!(file, "{}", ch)?;
                    }
                    writeln!(file)?;
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

    /// Generate a default character set with common terminal characters
    fn generate_default_character_set(&self) -> Vec<char> {
        let mut chars = Vec::new();

        // All 256 ASCII/extended ASCII characters (0-255)
        // We need all of them to maintain proper indexing in .fd format
        for i in 0..256 {
            chars.push(char::from_u32(i).unwrap());
        }

        // Add common Unicode box drawing characters (these will get indices 256+)
        let box_drawing = [
            '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼',
            '═', '║', '╔', '╗', '╚', '╝', '╠', '╣', '╦', '╩', '╬',
            '╭', '╮', '╯', '╰',
        ];
        chars.extend_from_slice(&box_drawing);

        // Block elements
        let blocks = [
            '█', '▓', '▒', '░', '▀', '▄', '▌', '▐',
            '▁', '▂', '▃', '▅', '▆', '▇',
        ];
        chars.extend_from_slice(&blocks);

        // Arrows
        let arrows = ['↑', '↓', '→', '←', '↔', '↕', '▲', '▼', '►', '◄'];
        chars.extend_from_slice(&arrows);

        // Common symbols
        let symbols = ['•', '○', '●', '◆', '◇', '■', '□', '°', '±', '×', '÷'];
        chars.extend_from_slice(&symbols);

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
