use fontdue::{Font as FontdueFont, FontSettings};
use font_kit::family_name::FamilyName;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// TrueType font wrapper using fontdue for rendering
pub struct TrueTypeFont {
    font: Arc<FontdueFont>,
    font_size: f32,
    char_width: usize,
    char_height: usize,
    /// Font metrics for baseline calculation
    ascender: f32,
    descender: f32,
    units_per_em: f32,
    baseline_offset: f32,
    /// Cache for rendered glyphs (using Mutex for interior mutability)
    glyph_cache: Arc<Mutex<HashMap<char, Vec<bool>>>>,
}

impl TrueTypeFont {
    /// Load a TrueType font from system by name or file path
    pub fn from_system(font_name: &str, char_height: usize) -> Result<Self, String> {
        // Check if this is a file path first
        use std::path::Path;
        let path = Path::new(font_name);
        if path.exists() && path.is_file() {
            eprintln!("Loading font from file: {}", font_name);
            return Self::from_file(font_name, char_height);
        }

        // Not a file, try to load by font name
        let source = SystemSource::new();

        // Special handling for requesting system default monospace
        let font_families = if font_name.eq_ignore_ascii_case("monospace")
            || font_name.eq_ignore_ascii_case("default")
            || font_name.eq_ignore_ascii_case("system") {
            eprintln!("Requesting system default monospace font");
            // Prioritize system monospace font
            vec![FamilyName::Monospace]
        } else {
            eprintln!("Looking for font by name: {}", font_name);
            // Try the specific font name, then fall back to system monospace
            vec![
                FamilyName::Title(font_name.to_string()),
                FamilyName::Monospace,
            ]
        };

        // Try to find the font by family name
        let font_handle = source
            .select_best_match(
                &font_families,
                &Properties::default(),
            )
            .map_err(|e| {
                eprintln!("Font lookup failed: {}", e);
                eprintln!("\nAvailable system fonts:");
                if let Ok(fonts) = Self::list_system_fonts() {
                    for (i, font) in fonts.iter().take(20).enumerate() {
                        eprintln!("  {}: {}", i + 1, font);
                    }
                    if fonts.len() > 20 {
                        eprintln!("  ... and {} more", fonts.len() - 20);
                    }
                }
                format!("Failed to find font '{}': {}", font_name, e)
            })?;

        // Load the font data
        let font_data = font_handle
            .load()
            .map_err(|e| format!("Failed to load font data: {}", e))?;

        // Get the font bytes - copy_font_data returns Option<Vec<u8>>
        let font_bytes = font_data
            .copy_font_data()
            .ok_or_else(|| "Failed to copy font data".to_string())?;

        Self::from_bytes(&font_bytes, char_height)
    }

    /// Load a TrueType font from a file path
    pub fn from_file(path: &str, char_height: usize) -> Result<Self, String> {
        use std::fs;
        let font_data = fs::read(path)
            .map_err(|e| format!("Failed to read font file '{}': {}", path, e))?;
        Self::from_bytes(&font_data, char_height)
    }

    /// List all available system fonts
    pub fn list_system_fonts() -> Result<Vec<String>, String> {
        use font_kit::handle::Handle;

        let source = SystemSource::new();
        let mut font_names = Vec::new();

        // Get all font families
        let families = source.all_families()
            .map_err(|e| format!("Failed to enumerate fonts: {}", e))?;

        for family_name in families {
            font_names.push(family_name);
        }

        font_names.sort();
        Ok(font_names)
    }

    /// Load a TrueType font from bytes
    pub fn from_bytes(font_data: &[u8], font_size: usize) -> Result<Self, String> {
        let font = FontdueFont::from_bytes(font_data, FontSettings::default())
            .map_err(|e| format!("Failed to parse font: {}", e))?;

        // Parse font metrics using ttf-parser
        let face = ttf_parser::Face::parse(font_data, 0)
            .map_err(|e| format!("Failed to parse font metrics: {:?}", e))?;

        let ascender = face.ascender() as f32;
        let descender = face.descender() as f32;
        let units_per_em = face.units_per_em() as f32;

        // Calculate scale to achieve desired font size
        let scale = font_size as f32 / units_per_em;
        let ascender_px = ascender * scale;
        let descender_px = descender * scale;

        // Cell height needs to fit ascenders and descenders with padding
        let padding_top = 2.0;
        let padding_bottom = 2.0;
        let char_height = (ascender_px - descender_px + padding_top + padding_bottom).ceil() as usize;

        // Baseline is positioned at: top_padding + ascender_height
        let baseline_offset = padding_top + ascender_px;

        // Calculate character width
        let char_width = Self::calculate_char_width(&font, font_size as f32);

        eprintln!("Font metrics: font_size={}px, ascender={:.1}px, descender={:.1}px, cell={}x{}, baseline={:.1}px",
            font_size, ascender_px, descender_px, char_width, char_height, baseline_offset);

        Ok(Self {
            font: Arc::new(font),
            font_size: font_size as f32,
            char_width,
            char_height,
            ascender,
            descender,
            units_per_em,
            baseline_offset,
            glyph_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Calculate average character width for monospace rendering
    fn calculate_char_width(font: &FontdueFont, font_size: f32) -> usize {
        // Measure width of 'M' which is typically the widest character in monospace fonts
        let (metrics, _) = font.rasterize('M', font_size);
        metrics.width.max(1)
    }

    pub fn width(&self) -> usize {
        self.char_width
    }

    pub fn height(&self) -> usize {
        self.char_height
    }

    /// Get glyph bitmap for a character (cached)
    pub fn get_glyph(&self, ch: char) -> Vec<bool> {
        // Check cache first
        {
            let cache = self.glyph_cache.lock().unwrap();
            if let Some(glyph) = cache.get(&ch) {
                return glyph.clone();
            }
        }

        // Not in cache, render it
        let glyph = self.rasterize_char(ch);

        // Store in cache
        {
            let mut cache = self.glyph_cache.lock().unwrap();
            cache.insert(ch, glyph.clone());
        }

        glyph
    }

    /// Rasterize a character to a boolean bitmap
    fn rasterize_char(&self, ch: char) -> Vec<bool> {
        let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);

        // Create a bitmap that fits our character cell
        let mut cell_bitmap = vec![false; self.char_width * self.char_height];

        // Center horizontally
        let offset_x = (self.char_width.saturating_sub(metrics.width)) / 2;

        // For vertical positioning, use proper font baseline
        // fontdue's ymin is distance from baseline to BOTTOM of glyph
        // negative ymin = bottom extends below baseline
        // positive ymin = bottom is above baseline
        // To get TOP of glyph: baseline - ymin - height
        let offset_y = self.baseline_offset.round() as i32 - metrics.ymin - metrics.height as i32;

        // Copy the rasterized glyph into the cell bitmap
        for y in 0..metrics.height {
            for x in 0..metrics.width {
                let src_idx = y * metrics.width + x;
                let dst_x = offset_x + x;

                // Calculate destination y with bounds checking for negative offsets
                let dst_y_signed = offset_y + y as i32;
                if dst_y_signed < 0 {
                    continue; // Skip pixels above the cell
                }
                let dst_y = dst_y_signed as usize;

                if dst_x < self.char_width && dst_y < self.char_height {
                    let dst_idx = dst_y * self.char_width + dst_x;
                    // Consider pixel "on" if alpha > threshold
                    cell_bitmap[dst_idx] = bitmap[src_idx] > 128;
                }
            }
        }

        cell_bitmap
    }
}

/// Query terminal for its font name
pub fn query_terminal_font() -> Option<String> {
    use std::io::{Write, Read};
    use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
    use std::time::Duration;
    use std::sync::mpsc::channel;
    use std::thread;

    // OSC 50 ; ? ST - Query font
    let query = "\x1b]50;?\x1b\\";

    if !crossterm::tty::IsTty::is_tty(&std::io::stderr()) {
        return None;
    }

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
                // Check for ST terminator: ESC \
                if buffer.len() >= 2 && buffer[buffer.len() - 2] == 0x1b && buffer[buffer.len() - 1] == b'\\' {
                    break;
                }
                // Check for BEL terminator
                if temp[0] == 0x07 {
                    break;
                }
                // Safety limit
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

    // Parse response: OSC 50 ; font_name ST
    if let Some(resp) = response {
        // Response format: \x1b]50;fontname\x1b\ or \x1b]50;fontname\x07
        if let Some(start) = resp.find("]50;") {
            let font_part = &resp[start + 4..];
            // Remove trailing escape sequences
            let font_name = font_part
                .trim_end_matches('\x1b')
                .trim_end_matches('\\')
                .trim_end_matches('\x07')
                .trim();

            if !font_name.is_empty() {
                return Some(font_name.to_string());
            }
        }
    }

    None
}
