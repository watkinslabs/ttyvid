use std::collections::HashMap;
use rust_embed::RustEmbed;

// Embed all font files at compile time
#[derive(RustEmbed)]
#[folder = "themes/fonts/"]
struct EmbeddedFonts;

use super::truetype_font::TrueTypeFont;

const DEFAULT_FONT_NAME: &str = "Verite_9x16";

pub enum Font {
    /// Legacy bitmap font (FD format) with CP437 character mapping, binary (on/off) pixels
    Bitmap {
        width: usize,
        height: usize,
        glyphs: Vec<Vec<bool>>, // Bitmap data for each of 256 characters
    },
    /// Modern bitmap font with UTF-8 support and grayscale anti-aliasing
    BitmapIntensity {
        width: usize,
        height: usize,
        glyphs: HashMap<char, Vec<u8>>, // Character -> intensity map (0-255 per pixel)
    },
    /// TrueType font with full UTF-8 support
    TrueType(TrueTypeFont),
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
        '╞' => 0xC3, // BOX DRAWINGS VERTICAL SINGLE AND RIGHT DOUBLE
        '╟' => 0xC3, // BOX DRAWINGS VERTICAL DOUBLE AND RIGHT SINGLE
        '╡' => 0xB4, // BOX DRAWINGS VERTICAL SINGLE AND LEFT DOUBLE
        '╢' => 0xB4, // BOX DRAWINGS VERTICAL DOUBLE AND LEFT SINGLE
        '╤' => 0xC2, // BOX DRAWINGS DOWN SINGLE AND HORIZONTAL DOUBLE
        '╥' => 0xC2, // BOX DRAWINGS DOWN DOUBLE AND HORIZONTAL SINGLE
        '╧' => 0xC1, // BOX DRAWINGS UP SINGLE AND HORIZONTAL DOUBLE
        '╨' => 0xC1, // BOX DRAWINGS UP DOUBLE AND HORIZONTAL SINGLE
        '╪' => 0xC5, // BOX DRAWINGS VERTICAL SINGLE AND HORIZONTAL DOUBLE
        '╫' => 0xC5, // BOX DRAWINGS VERTICAL DOUBLE AND HORIZONTAL SINGLE

        // Arc box drawing (rounded corners)
        '╭' => 0xDA, // BOX DRAWINGS LIGHT ARC DOWN AND RIGHT
        '╮' => 0xBF, // BOX DRAWINGS LIGHT ARC DOWN AND LEFT
        '╯' => 0xD9, // BOX DRAWINGS LIGHT ARC UP AND LEFT
        '╰' => 0xC0, // BOX DRAWINGS LIGHT ARC UP AND RIGHT

        // Heavy box drawing
        '╱' => b'/', // BOX DRAWINGS LIGHT DIAGONAL UPPER RIGHT TO LOWER LEFT
        '╲' => b'\\', // BOX DRAWINGS LIGHT DIAGONAL UPPER LEFT TO LOWER RIGHT
        '╳' => b'X', // BOX DRAWINGS LIGHT DIAGONAL CROSS

        // Block elements (full coverage)
        '█' => 0xDB, // FULL BLOCK
        '▓' => 0xB2, // DARK SHADE
        '▒' => 0xB1, // MEDIUM SHADE
        '░' => 0xB0, // LIGHT SHADE
        '▀' => 0xDF, // UPPER HALF BLOCK
        '▄' => 0xDC, // LOWER HALF BLOCK
        '▌' => 0xDD, // LEFT HALF BLOCK
        '▐' => 0xDE, // RIGHT HALF BLOCK

        // Additional block elements (map to closest CP437 equivalents)
        '▁' => 0xDC, // LOWER ONE EIGHTH BLOCK -> LOWER HALF BLOCK
        '▂' => 0xDC, // LOWER ONE QUARTER BLOCK -> LOWER HALF BLOCK
        '▃' => 0xDC, // LOWER THREE EIGHTHS BLOCK -> LOWER HALF BLOCK
        '▅' => 0xDB, // LOWER FIVE EIGHTHS BLOCK -> FULL BLOCK
        '▆' => 0xDB, // LOWER THREE QUARTERS BLOCK -> FULL BLOCK
        '▇' => 0xDB, // LOWER SEVEN EIGHTHS BLOCK -> FULL BLOCK
        '▉' => 0xDB, // LEFT SEVEN EIGHTHS BLOCK -> FULL BLOCK
        '▊' => 0xDD, // LEFT THREE QUARTERS BLOCK -> LEFT HALF BLOCK
        '▋' => 0xDD, // LEFT FIVE EIGHTHS BLOCK -> LEFT HALF BLOCK
        '▍' => 0xDD, // LEFT THREE EIGHTHS BLOCK -> LEFT HALF BLOCK
        '▎' => 0xDD, // LEFT ONE QUARTER BLOCK -> LEFT HALF BLOCK
        '▏' => 0xDD, // LEFT ONE EIGHTH BLOCK -> LEFT HALF BLOCK
        '▕' => 0xDE, // RIGHT ONE EIGHTH BLOCK -> RIGHT HALF BLOCK
        '▔' => 0xDF, // UPPER ONE EIGHTH BLOCK -> UPPER HALF BLOCK
        '▖' => 0xDC, // QUADRANT LOWER LEFT -> LOWER HALF BLOCK
        '▗' => 0xDC, // QUADRANT LOWER RIGHT -> LOWER HALF BLOCK
        '▘' => 0xDF, // QUADRANT UPPER LEFT -> UPPER HALF BLOCK
        '▙' => 0xDB, // QUADRANT UPPER LEFT AND LOWER LEFT AND LOWER RIGHT -> FULL BLOCK
        '▚' => 0xB2, // QUADRANT UPPER LEFT AND LOWER RIGHT -> DARK SHADE
        '▛' => 0xDB, // QUADRANT UPPER LEFT AND UPPER RIGHT AND LOWER LEFT -> FULL BLOCK
        '▜' => 0xDB, // QUADRANT UPPER LEFT AND UPPER RIGHT AND LOWER RIGHT -> FULL BLOCK
        '▝' => 0xDF, // QUADRANT UPPER RIGHT -> UPPER HALF BLOCK
        '▞' => 0xB2, // QUADRANT UPPER RIGHT AND LOWER LEFT -> DARK SHADE
        '▟' => 0xDB, // QUADRANT UPPER RIGHT AND LOWER LEFT AND LOWER RIGHT -> FULL BLOCK

        // Braille patterns (for graphs - comprehensive set)
        '⠀' => b' ',  // BRAILLE PATTERN BLANK
        '⠁' => 0xB0, '⠂' => 0xB0, '⠃' => 0xB0, '⠄' => 0xB0, '⠅' => 0xB0, '⠆' => 0xB0, '⠇' => 0xB0,
        '⠈' => 0xB0, '⠉' => 0xB0, '⠊' => 0xB0, '⠋' => 0xB0, '⠌' => 0xB0, '⠍' => 0xB0, '⠎' => 0xB0, '⠏' => 0xB0,
        '⠐' => 0xB0, '⠑' => 0xB0, '⠒' => 0xB0, '⠓' => 0xB0, '⠔' => 0xB0, '⠕' => 0xB0, '⠖' => 0xB0, '⠗' => 0xB0,
        '⠘' => 0xB0, '⠙' => 0xB0, '⠚' => 0xB0, '⠛' => 0xB0, '⠜' => 0xB0, '⠝' => 0xB0, '⠞' => 0xB0, '⠟' => 0xB0,
        '⠠' => 0xB0, '⠡' => 0xB0, '⠢' => 0xB0, '⠣' => 0xB0, '⠤' => 0xB1, '⠥' => 0xB1, '⠦' => 0xB1, '⠧' => 0xB1,
        '⠨' => 0xB1, '⠩' => 0xB1, '⠪' => 0xB1, '⠫' => 0xB1, '⠬' => 0xB1, '⠭' => 0xB1, '⠮' => 0xB1, '⠯' => 0xB1,
        '⠰' => 0xB1, '⠱' => 0xB1, '⠲' => 0xB1, '⠳' => 0xB1, '⠴' => 0xB1, '⠵' => 0xB1, '⠶' => 0xB1, '⠷' => 0xB1,
        '⠸' => 0xB1, '⠹' => 0xB1, '⠺' => 0xB1, '⠻' => 0xB1, '⠼' => 0xB1, '⠽' => 0xB1, '⠾' => 0xB1, '⠿' => 0xB1,
        '⡀' => 0xB1, '⡁' => 0xB1, '⡂' => 0xB1, '⡃' => 0xB1, '⡄' => 0xB1, '⡅' => 0xB1, '⡆' => 0xB1, '⡇' => 0xB1,
        '⡈' => 0xB1, '⡉' => 0xB1, '⡊' => 0xB1, '⡋' => 0xB1, '⡌' => 0xB1, '⡍' => 0xB1, '⡎' => 0xB1, '⡏' => 0xB1,
        '⡐' => 0xB1, '⡑' => 0xB1, '⡒' => 0xB1, '⡓' => 0xB1, '⡔' => 0xB1, '⡕' => 0xB1, '⡖' => 0xB1, '⡗' => 0xB1,
        '⡘' => 0xB1, '⡙' => 0xB1, '⡚' => 0xB1, '⡛' => 0xB1, '⡜' => 0xB1, '⡝' => 0xB1, '⡞' => 0xB1, '⡟' => 0xB1,
        '⡠' => 0xB1, '⡡' => 0xB1, '⡢' => 0xB1, '⡣' => 0xB1, '⡤' => 0xB2, '⡥' => 0xB2, '⡦' => 0xB2, '⡧' => 0xB2,
        '⡨' => 0xB2, '⡩' => 0xB2, '⡪' => 0xB2, '⡫' => 0xB2, '⡬' => 0xB2, '⡭' => 0xB2, '⡮' => 0xB2, '⡯' => 0xB2,
        '⡰' => 0xB2, '⡱' => 0xB2, '⡲' => 0xB2, '⡳' => 0xB2, '⡴' => 0xB2, '⡵' => 0xB2, '⡶' => 0xB2, '⡷' => 0xB2,
        '⡸' => 0xB2, '⡹' => 0xB2, '⡺' => 0xB2, '⡻' => 0xB2, '⡼' => 0xB2, '⡽' => 0xB2, '⡾' => 0xB2, '⡿' => 0xB2,
        '⢀' => 0xB1, '⢁' => 0xB1, '⢂' => 0xB1, '⢃' => 0xB1, '⢄' => 0xB1, '⢅' => 0xB1, '⢆' => 0xB1, '⢇' => 0xB1,
        '⢈' => 0xB1, '⢉' => 0xB1, '⢊' => 0xB1, '⢋' => 0xB1, '⢌' => 0xB1, '⢍' => 0xB1, '⢎' => 0xB1, '⢏' => 0xB1,
        '⢐' => 0xB1, '⢑' => 0xB1, '⢒' => 0xB1, '⢓' => 0xB1, '⢔' => 0xB1, '⢕' => 0xB1, '⢖' => 0xB1, '⢗' => 0xB1,
        '⢘' => 0xB1, '⢙' => 0xB1, '⢚' => 0xB1, '⢛' => 0xB1, '⢜' => 0xB1, '⢝' => 0xB1, '⢞' => 0xB1, '⢟' => 0xB1,
        '⢠' => 0xB2, '⢡' => 0xB2, '⢢' => 0xB2, '⢣' => 0xB2, '⢤' => 0xB2, '⢥' => 0xB2, '⢦' => 0xB2, '⢧' => 0xB2,
        '⢨' => 0xB2, '⢩' => 0xB2, '⢪' => 0xB2, '⢫' => 0xB2, '⢬' => 0xB2, '⢭' => 0xB2, '⢮' => 0xB2, '⢯' => 0xB2,
        '⢰' => 0xB2, '⢱' => 0xB2, '⢲' => 0xB2, '⢳' => 0xB2, '⢴' => 0xB2, '⢵' => 0xB2, '⢶' => 0xB2, '⢷' => 0xB2,
        '⢸' => 0xB2, '⢹' => 0xB2, '⢺' => 0xB2, '⢻' => 0xB2, '⢼' => 0xB2, '⢽' => 0xB2, '⢾' => 0xB2, '⢿' => 0xB2,
        '⣀' => 0xB2, '⣁' => 0xB2, '⣂' => 0xB2, '⣃' => 0xB2, '⣄' => 0xB2, '⣅' => 0xB2, '⣆' => 0xB2, '⣇' => 0xB2,
        '⣈' => 0xB2, '⣉' => 0xB2, '⣊' => 0xB2, '⣋' => 0xB2, '⣌' => 0xB2, '⣍' => 0xB2, '⣎' => 0xB2, '⣏' => 0xB2,
        '⣐' => 0xB2, '⣑' => 0xB2, '⣒' => 0xB2, '⣓' => 0xB2, '⣔' => 0xB2, '⣕' => 0xB2, '⣖' => 0xB2, '⣗' => 0xB2,
        '⣘' => 0xB2, '⣙' => 0xB2, '⣚' => 0xB2, '⣛' => 0xB2, '⣜' => 0xB2, '⣝' => 0xB2, '⣞' => 0xB2, '⣟' => 0xB2,
        '⣠' => 0xB2, '⣡' => 0xB2, '⣢' => 0xB2, '⣣' => 0xB2, '⣤' => 0xB2, '⣥' => 0xB2, '⣦' => 0xB2, '⣧' => 0xB2,
        '⣨' => 0xB2, '⣩' => 0xB2, '⣪' => 0xB2, '⣫' => 0xB2, '⣬' => 0xB2, '⣭' => 0xB2, '⣮' => 0xB2, '⣯' => 0xB2,
        '⣰' => 0xB2, '⣱' => 0xB2, '⣲' => 0xB2, '⣳' => 0xB2, '⣴' => 0xDB, '⣵' => 0xDB, '⣶' => 0xDB, '⣷' => 0xDB,
        '⣸' => 0xDB, '⣹' => 0xDB, '⣺' => 0xDB, '⣻' => 0xDB, '⣼' => 0xDB, '⣽' => 0xDB, '⣾' => 0xDB, '⣿' => 0xDB,

        // Progress/graph bars
        '▪' => 0xFE, // BLACK SMALL SQUARE -> SMALL BLOCK
        '▫' => 0xB0, // WHITE SMALL SQUARE -> LIGHT SHADE
        '●' => 0x09, // BLACK CIRCLE -> WHITE CIRCLE
        '○' => 0x09, // WHITE CIRCLE
        '◆' => 0x04, // BLACK DIAMOND -> DIAMOND
        '◇' => 0x04, // WHITE DIAMOND -> DIAMOND
        '■' => 0xDB, // BLACK SQUARE -> FULL BLOCK
        '□' => 0xB0, // WHITE SQUARE -> LIGHT SHADE
        '▬' => 0x16, // BLACK RECTANGLE

        // Arrows and pointers
        '↑' => 0x18, // UPWARDS ARROW
        '↓' => 0x19, // DOWNWARDS ARROW
        '→' => 0x1A, // RIGHTWARDS ARROW
        '←' => 0x1B, // LEFTWARDS ARROW
        '↔' => 0x1D, // LEFT RIGHT ARROW
        '↕' => 0x12, // UP DOWN ARROW
        '↨' => 0x17, // UP DOWN ARROW WITH BASE
        '▲' => 0x1E, // BLACK UP-POINTING TRIANGLE
        '▼' => 0x1F, // BLACK DOWN-POINTING TRIANGLE
        '►' => 0x10, // BLACK RIGHT-POINTING POINTER
        '◄' => 0x11, // BLACK LEFT-POINTING POINTER
        '⇧' => 0x18, // UPWARDS WHITE ARROW -> UPWARDS ARROW
        '⇩' => 0x19, // DOWNWARDS WHITE ARROW -> DOWNWARDS ARROW

        // Other special characters
        '•' => 0x07, // BULLET
        '◘' => 0x08, // INVERSE BULLET
        '◙' => 0x0A, // INVERSE WHITE CIRCLE
        '♂' => 0x0B, // MALE SIGN
        '♀' => 0x0C, // FEMALE SIGN
        '♪' => 0x0D, // EIGHTH NOTE
        '♫' => 0x0E, // BEAMED EIGHTH NOTES
        '☼' => 0x0F, // WHITE SUN WITH RAYS
        '‼' => 0x13, // DOUBLE EXCLAMATION MARK
        '¶' => 0x14, // PILCROW SIGN
        '§' => 0x15, // SECTION SIGN
        '∟' => 0x1C, // RIGHT ANGLE

        // Degree and percentage symbols
        '°' => 0xF8, // DEGREE SIGN
        '±' => 0xF1, // PLUS-MINUS SIGN
        '×' => 0x9E, // MULTIPLICATION SIGN
        '÷' => 0xF6, // DIVISION SIGN
        '≈' => 0xF7, // ALMOST EQUAL TO
        '≥' => 0xF2, // GREATER-THAN OR EQUAL TO
        '≤' => 0xF3, // LESS-THAN OR EQUAL TO
        '≠' => 0xAD, // NOT EQUAL TO

        // Fractions (for statistics)
        '½' => 0xAB, // VULGAR FRACTION ONE HALF
        '¼' => 0xAC, // VULGAR FRACTION ONE QUARTER
        '¾' => 0xF3, // VULGAR FRACTION THREE QUARTERS (approximate)

        // Currency and special
        '¢' => 0x9B, // CENT SIGN
        '£' => 0x9C, // POUND SIGN
        '¥' => 0x9D, // YEN SIGN
        '₧' => 0x9E, // PESETA SIGN
        'ƒ' => 0x9F, // LATIN SMALL LETTER F WITH HOOK

        // Superscripts and subscripts (for units)
        '²' => 0xFD, // SUPERSCRIPT TWO
        '³' => 0xFD, // SUPERSCRIPT THREE (approximate)

        // Fallback for ASCII range
        _ if (ch as u32) < 256 => ch as u8,

        // Fallback for unknown characters
        _ => b'?',
    }
}

impl Font {
    /// Load a TrueType font from the system by name with smart fallbacks
    ///
    /// Special font names:
    /// - "monospace", "default", or "system" - Uses system's default monospace font
    ///
    /// Otherwise tries the specified font name, then falls back to common terminal fonts
    pub fn from_system_font(font_name: &str, char_height: usize) -> Option<Self> {
        // Try the requested font first
        if let Ok(ttf) = TrueTypeFont::from_system(font_name, char_height) {
            eprintln!("Loaded system font: {}", font_name);
            return Some(Font::TrueType(ttf));
        }

        eprintln!("Font '{}' not found, trying common terminal fonts...", font_name);

        // Fallback to common monospace/terminal fonts
        let fallback_fonts = [
            // Original request (try variations)
            font_name,
            // System default monospace (works with special keywords handled in TrueTypeFont::from_system)
            "Monospace",
            // Common terminal fonts
            "DejaVu Sans Mono",
            "Liberation Mono",
            "Consolas",
            "Courier New",
            "Monaco",
            "Menlo",
            "Ubuntu Mono",
            "Fira Mono",
            "Source Code Pro",
            "JetBrains Mono",
            "Cascadia Code",
            "SF Mono",
            "Hack",
            // Fallback to any monospace
            "monospace",
        ];

        for fallback in &fallback_fonts[1..] {  // Skip first since we already tried it
            if let Ok(ttf) = TrueTypeFont::from_system(fallback, char_height) {
                eprintln!("Using fallback font: {}", fallback);
                return Some(Font::TrueType(ttf));
            }
        }

        eprintln!("No system fonts available, will use embedded bitmap font");
        None
    }

    /// Load a bitmap font (FD format)
    pub fn load(name: Option<&str>) -> Self {
        // Use specified font or default
        let font_name = name.unwrap_or(DEFAULT_FONT_NAME);
        let font_file = format!("{}.fd", font_name);

        // Look up font in embedded fonts
        if let Some(embedded_file) = EmbeddedFonts::get(&font_file) {
            let font_data = String::from_utf8_lossy(&embedded_file.data);
            match Self::parse_fd_font(&font_data) {
                Ok(font) => return font,
                Err(e) => eprintln!("Warning: Failed to parse font '{}': {}", font_name, e),
            }
        } else {
            eprintln!("Warning: Font '{}' not found in embedded fonts", font_name);
        }

        // Fall back to default font if specified font failed
        if font_name != DEFAULT_FONT_NAME {
            let default_file = format!("{}.fd", DEFAULT_FONT_NAME);
            if let Some(embedded_file) = EmbeddedFonts::get(&default_file) {
                let font_data = String::from_utf8_lossy(&embedded_file.data);
                if let Ok(font) = Self::parse_fd_font(&font_data) {
                    return font;
                }
            }
        }

        // Last resort: fallback font
        Self::fallback_font()
    }

    /// Load a bitmap font from a file path (.fd format)
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        let font_data = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read font file: {}", e))?;
        Self::parse_fd_font(&font_data)
    }

    /// Get list of available embedded font names
    pub fn available_fonts() -> Vec<String> {
        EmbeddedFonts::iter()
            .filter_map(|path| {
                let path_str = path.as_ref();
                if path_str.ends_with(".fd") {
                    path_str.strip_suffix(".fd").map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn width(&self) -> usize {
        match self {
            Font::Bitmap { width, .. } => *width,
            Font::BitmapIntensity { width, .. } => *width,
            Font::TrueType(ttf) => ttf.width(),
        }
    }

    pub fn height(&self) -> usize {
        match self {
            Font::Bitmap { height, .. } => *height,
            Font::BitmapIntensity { height, .. } => *height,
            Font::TrueType(ttf) => ttf.height(),
        }
    }

    pub fn glyph_count(&self) -> usize {
        match self {
            Font::Bitmap { glyphs, .. } => glyphs.len(),
            Font::BitmapIntensity { glyphs, .. } => glyphs.len(),
            Font::TrueType(_) => 256, // TrueType supports all characters, but report 256 for compatibility
        }
    }

    pub fn get_glyph(&self, ch: u8) -> Vec<u8> {
        self.get_glyph_utf8(ch as char)
    }

    /// Get glyph for a character (CPU rendering)
    /// Returns the pixel data for CPU-based rendering
    pub fn get_glyph_utf8(&self, ch: char) -> Vec<u8> {
        match self {
            Font::Bitmap { glyphs, .. } => {
                // Legacy bitmap - convert bool to u8 (0 or 255)
                let mapped = map_utf8_to_cp437(ch);
                glyphs[mapped as usize]
                    .iter()
                    .map(|&b| if b { 255 } else { 0 })
                    .collect()
            }
            Font::BitmapIntensity { glyphs, width, height, .. } => {
                // Try direct UTF-8 lookup first
                if let Some(glyph) = glyphs.get(&ch) {
                    return glyph.clone();
                }

                // If not found, try ASCII fallback mapping
                let fallback_ch = map_utf8_to_cp437(ch) as char;
                if let Some(glyph) = glyphs.get(&fallback_ch) {
                    return glyph.clone();
                }

                // Character not found - return empty glyph
                vec![0u8; width * height]
            }
            Font::TrueType(ttf) => {
                // TrueType returns grayscale directly
                ttf.get_glyph_intensity(ch)
            }
        }
    }

    /// Get glyph index for a character (GPU rendering)
    /// Returns the index into the font's glyph array for GPU texture atlas
    pub fn get_glyph_index_utf8(&self, ch: char) -> Option<usize> {
        match self {
            Font::Bitmap { .. } => {
                // Legacy bitmap uses CP437 mapping
                let mapped = map_utf8_to_cp437(ch);
                Some(mapped as usize)
            }
            Font::BitmapIntensity { glyphs, .. } => {
                // Try direct UTF-8 lookup first
                // For GPU, we need to track the order/index
                // This is a simplified version - real GPU rendering would need
                // a separate index mapping structure
                if glyphs.contains_key(&ch) {
                    // For now, just indicate the character exists
                    // A proper implementation would maintain an index->char mapping
                    Some(ch as usize)
                } else {
                    // Try ASCII fallback mapping
                    let fallback_ch = map_utf8_to_cp437(ch) as char;
                    if glyphs.contains_key(&fallback_ch) {
                        Some(fallback_ch as usize)
                    } else {
                        None
                    }
                }
            }
            Font::TrueType(_) => {
                // TrueType fonts: use Unicode codepoint as index
                Some(ch as usize)
            }
        }
    }

    /// Render a string of text onto a canvas with scaling
    pub fn render_string(&self, canvas: &mut crate::renderer::Canvas, x: i32, y: i32, text: &str, fg_color: u8, bg_color: u8, size: f32) {
        let scaled_width = (self.width() as f32 * size) as i32;

        for (char_idx, ch) in text.chars().enumerate() {
            let char_x = x + (char_idx as i32 * scaled_width);
            // Render character directly (handles UTF-8 properly for both font types)
            self.render_character_utf8(canvas, char_x, y, ch, fg_color, bg_color, size);
        }
    }

    /// Render a single UTF-8 character onto a canvas with scaling
    /// Handles both bitmap fonts (with CP437 mapping) and TrueType fonts (direct UTF-8)
    fn render_character_utf8(&self, canvas: &mut crate::renderer::Canvas, x: i32, y: i32, ch: char, fg_color: u8, _bg_color: u8, size: f32) {
        // Get glyph - this handles mapping for bitmap fonts internally
        let glyph = self.get_glyph_utf8(ch);
        let char_width = self.width();
        let char_height = self.height();
        let scaled_width = (char_width as f32 * size).round() as usize;
        let scaled_height = (char_height as f32 * size).round() as usize;

        for sy in 0..scaled_height {
            for sx in 0..scaled_width {
                // Map scaled pixel back to source glyph pixel (nearest neighbor)
                let glyph_x = ((sx as f32) / size) as usize;
                let glyph_y = ((sy as f32) / size) as usize;

                if glyph_x < char_width && glyph_y < char_height {
                    let glyph_idx = glyph_y * char_width + glyph_x;
                    let intensity = glyph[glyph_idx];

                    // Only render if pixel has some intensity
                    if intensity > 0 {
                        let pixel_x = x + sx as i32;
                        let pixel_y = y + sy as i32;

                        if pixel_x >= 0 && pixel_y >= 0 &&
                           pixel_x < canvas.width() as i32 && pixel_y < canvas.height() as i32 {
                            // For now, just use full foreground color if intensity > threshold
                            // TODO: Implement alpha blending for anti-aliasing
                            if intensity > 127 {
                                canvas.set_pixel(pixel_x as usize, pixel_y as usize, fg_color);
                            }
                        }
                    }
                }
            }
        }
    }

    // Parse .fd font format (supports both legacy and new intensity formats)
    fn parse_fd_font(data: &str) -> Result<Self, String> {
        let lines: Vec<&str> = data.lines().collect();
        let mut height = 16;
        let mut width = 9;
        let mut char_start_idx = 0;
        let mut charset_size: Option<usize> = None;

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
            } else if line.starts_with("charset") {
                charset_size = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok());
            } else if line.starts_with("char") {
                // Found first character, remember where it starts
                char_start_idx = idx;
                break;
            }
        }

        // Detect if this is a new-format font (charset > 256 or has unicode mappings)
        let is_new_format = charset_size.map(|s| s > 256).unwrap_or(false) ||
                            data.contains("unicode");

        if is_new_format {
            // Parse as new format with intensities and UTF-8 support
            Self::parse_fd_font_intensity(&lines[char_start_idx..], width, height)
        } else {
            // Parse as legacy format
            Self::parse_fd_font_legacy(&lines[char_start_idx..], width, height)
        }
    }

    // Parse legacy .fd format (256 chars max, bool pixels)
    fn parse_fd_font_legacy(lines: &[&str], width: usize, height: usize) -> Result<Self, String> {
        let mut glyphs = vec![vec![false; width * height]; 256];
        let mut current_char: Option<usize> = None;
        let mut current_width = width;
        let mut bitmap_lines: Vec<String> = Vec::new();

        for line in lines {
            let line = line.trim();

            if line.starts_with("char") {
                // Save previous character if we have one
                if let Some(ch_idx) = current_char {
                    if ch_idx < 256 && bitmap_lines.len() == height {
                        glyphs[ch_idx] = Self::parse_bitmap_bool(&bitmap_lines, current_width, height);
                    }
                }

                // Start new character
                current_char = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok());
                bitmap_lines.clear();
                current_width = width;
            } else if line.starts_with("width") {
                current_width = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(width);
            } else if !line.is_empty() && !line.starts_with('#') && (line.contains('x') || line.contains('.')) {
                bitmap_lines.push(line.to_string());
            }
        }

        // Save last character
        if let Some(ch_idx) = current_char {
            if ch_idx < 256 && bitmap_lines.len() == height {
                glyphs[ch_idx] = Self::parse_bitmap_bool(&bitmap_lines, current_width, height);
            }
        }

        Ok(Font::Bitmap {
            width,
            height,
            glyphs,
        })
    }

    // Parse new .fd format with intensity values and UTF-8 support
    fn parse_fd_font_intensity(lines: &[&str], width: usize, height: usize) -> Result<Self, String> {
        let mut glyphs: HashMap<char, Vec<u8>> = HashMap::new();
        let mut current_char_idx: Option<usize> = None;
        let mut current_unicode: Option<char> = None;
        let mut current_width = width;
        let mut bitmap_lines: Vec<String> = Vec::new();

        for line in lines {
            let line = line.trim();

            if line.starts_with("char") {
                // Save previous character if we have one
                if let (Some(_idx), Some(unicode_ch)) = (current_char_idx, current_unicode) {
                    if bitmap_lines.len() == height {
                        let glyph = Self::parse_bitmap_intensity(&bitmap_lines, current_width, height);
                        glyphs.insert(unicode_ch, glyph);
                    }
                }

                // Start new character
                current_char_idx = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok());
                current_unicode = None; // Will be set by unicode line
                bitmap_lines.clear();
                current_width = width;
            } else if line.starts_with("unicode") {
                // Parse unicode value (e.g., "unicode 0x0041")
                if let Some(hex_str) = line.split_whitespace().nth(1) {
                    if let Some(hex_part) = hex_str.strip_prefix("0x").or_else(|| hex_str.strip_prefix("0X")) {
                        if let Ok(codepoint) = u32::from_str_radix(hex_part, 16) {
                            current_unicode = char::from_u32(codepoint);
                        }
                    }
                }
            } else if line.starts_with("width") {
                current_width = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(width);
            } else if !line.is_empty() && !line.starts_with('#') &&
                     (line.contains('x') || line.contains('.') || line.chars().any(|c| c.is_ascii_digit())) {
                bitmap_lines.push(line.to_string());
            }
        }

        // Save last character
        if let (Some(_idx), Some(unicode_ch)) = (current_char_idx, current_unicode) {
            if bitmap_lines.len() == height {
                let glyph = Self::parse_bitmap_intensity(&bitmap_lines, current_width, height);
                glyphs.insert(unicode_ch, glyph);
            }
        }

        Ok(Font::BitmapIntensity {
            width,
            height,
            glyphs,
        })
    }

    fn parse_bitmap_bool(lines: &[String], char_width: usize, height: usize) -> Vec<bool> {
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

    fn parse_bitmap_intensity(lines: &[String], char_width: usize, height: usize) -> Vec<u8> {
        let mut bitmap = Vec::with_capacity(char_width * height);

        for line in lines {
            for (i, ch) in line.chars().enumerate() {
                if i >= char_width {
                    break;
                }
                // Map intensity characters to u8 values
                // . = 0, 1 = 25, 2 = 51, ... 9 = 230, x = 255
                let intensity = match ch {
                    '.' => 0,
                    '1' => 25,
                    '2' => 51,
                    '3' => 76,
                    '4' => 102,
                    '5' => 127,
                    '6' => 153,
                    '7' => 178,
                    '8' => 204,
                    '9' => 230,
                    'x' | 'X' => 255,
                    _ => 0, // Unknown character, treat as blank
                };
                bitmap.push(intensity);
            }

            // Pad line if needed
            while bitmap.len() % char_width != 0 {
                bitmap.push(0);
            }
        }

        // Ensure we have exactly width * height pixels
        bitmap.resize(char_width * height, 0);
        bitmap
    }

    // Fallback font in case parsing fails (returns legacy bool format)
    fn fallback_font() -> Self {
        let width = 8;
        let height = 16;
        let mut glyphs = Vec::with_capacity(256);

        for i in 0..=255 {
            let glyph = Self::generate_fallback_glyph_bool(i, width, height);
            glyphs.push(glyph);
        }

        Font::Bitmap {
            width,
            height,
            glyphs,
        }
    }

    fn generate_fallback_glyph_bool(ch: u8, width: usize, height: usize) -> Vec<bool> {
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
