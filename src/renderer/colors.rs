use std::io::{Write, Read};
use std::time::Duration;

#[derive(Clone)]
pub struct Palette {
    colors: Vec<u8>, // RGB triplets
}

impl Palette {
    /// Create a palette from theme palette data
    pub fn from_theme(theme_palette: &crate::theme::Palette) -> Self {
        let mut colors = Vec::with_capacity(theme_palette.rgb.len() * 3);
        for rgb in &theme_palette.rgb {
            colors.extend_from_slice(rgb);
        }
        Self { colors }
    }

    /// Query the current terminal for its actual color palette
    /// Falls back to default palette for any colors that can't be queried
    /// Returns (palette, default_fg_index, default_bg_index)
    pub fn from_terminal() -> (Self, Option<u8>, Option<u8>) {
        // Check if we have a TTY
        if !crossterm::tty::IsTty::is_tty(&std::io::stderr()) {
            eprintln!("Warning: Not running in a TTY, using default palette");
            return (Self::default(), None, None);
        }

        // Start with default palette as fallback
        let mut palette = Self::default();

        eprintln!("Querying terminal colors...");

        // Try to query the first 16 colors (most important)
        // OSC 4 ; color ; ? BEL
        for color_idx in 0..16 {
            if let Some((r, g, b)) = query_terminal_color(color_idx) {
                let idx = color_idx * 3;
                palette.colors[idx] = r;
                palette.colors[idx + 1] = g;
                palette.colors[idx + 2] = b;
            }
        }

        // Query default foreground (OSC 10) and background (OSC 11) colors
        let default_fg = query_default_color(10).and_then(|(r, g, b)| {
            // Find closest palette index
            Some(palette.match_color_index(r as i32, g as i32, b as i32))
        });

        let default_bg = query_default_color(11).and_then(|(r, g, b)| {
            // Find closest palette index
            Some(palette.match_color_index(r as i32, g as i32, b as i32))
        });

        eprintln!("Terminal colors captured");
        (palette, default_fg, default_bg)
    }

    pub fn default() -> Self {
        let mut colors = Vec::with_capacity(256 * 3);

        // 16 System Colors (0-15)
        let system_colors = [
            0, 0, 0,         // 0: Black
            128, 0, 0,       // 1: Red
            0, 128, 0,       // 2: Green
            128, 128, 0,     // 3: Yellow
            0, 0, 128,       // 4: Blue
            128, 0, 128,     // 5: Magenta
            0, 128, 128,     // 6: Cyan
            192, 192, 192,   // 7: White
            128, 128, 128,   // 8: Bright Black (Gray)
            255, 0, 0,       // 9: Bright Red
            0, 255, 0,       // 10: Bright Green
            255, 255, 0,     // 11: Bright Yellow
            0, 0, 255,       // 12: Bright Blue
            255, 0, 255,     // 13: Bright Magenta
            0, 255, 255,     // 14: Bright Cyan
            255, 255, 255,   // 15: Bright White
        ];

        colors.extend_from_slice(&system_colors);

        // 216 Color Cube (16-231)
        for r in 0..6 {
            for g in 0..6 {
                for b in 0..6 {
                    let r_val = if r == 0 { 0 } else { 55 + r * 40 };
                    let g_val = if g == 0 { 0 } else { 55 + g * 40 };
                    let b_val = if b == 0 { 0 } else { 55 + b * 40 };
                    colors.push(r_val);
                    colors.push(g_val);
                    colors.push(b_val);
                }
            }
        }

        // 24 Grayscale (232-255)
        for i in 0..24 {
            let val = 8 + i * 10;
            colors.push(val);
            colors.push(val);
            colors.push(val);
        }

        Self { colors }
    }

    pub fn colors(&self) -> &[u8] {
        &self.colors
    }

    pub fn get_rgb(&self, index: u8) -> (u8, u8, u8) {
        let idx = index as usize * 3;
        if idx + 2 < self.colors.len() {
            (self.colors[idx], self.colors[idx + 1], self.colors[idx + 2])
        } else {
            (0, 0, 0)
        }
    }

    /// Get RGB colors as array of [r, g, b] triplets
    pub fn rgb_colors(&self) -> Vec<[u8; 3]> {
        let mut rgb = Vec::with_capacity(256);
        for i in (0..self.colors.len()).step_by(3) {
            if i + 2 < self.colors.len() {
                rgb.push([self.colors[i], self.colors[i + 1], self.colors[i + 2]]);
            }
        }
        // Pad to 256 colors if needed
        while rgb.len() < 256 {
            rgb.push([0, 0, 0]);
        }
        rgb
    }

    // Exact translation of graphics.pyx match_color_index lines 99-120
    pub fn match_color_index(&self, r: i32, g: i32, b: i32) -> u8 {
        let mut last_distance: i32 = -1;
        let mut mapped_color: usize = 0;

        let color_table_len = self.colors.len();
        for i in (0..color_table_len).step_by(3) {
            let mr = self.colors[i] as i32;
            let mg = self.colors[i + 1] as i32;
            let mb = self.colors[i + 2] as i32;
            let color_distance = (r - mr) * (r - mr) + (g - mg) * (g - mg) + (b - mb) * (b - mb);

            if last_distance == -1 || color_distance < last_distance {
                last_distance = color_distance;
                mapped_color = i / 3;
            }
        }

        if mapped_color > 255 {
            panic!("Color value too high");
        }

        mapped_color as u8
    }
}

/// Query default foreground (10) or background (11) color using OSC escape sequence
/// Returns None if the query fails or times out
fn query_default_color(osc_number: usize) -> Option<(u8, u8, u8)> {
    use std::io::stderr;
    use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

    // OSC 10 ; ? BEL for foreground, OSC 11 ; ? BEL for background
    let query = format!("\x1b]{};?\x1b\\", osc_number);

    // Enable raw mode to read response
    if enable_raw_mode().is_err() {
        return None;
    }

    // Send query to stderr (where terminal control sequences go)
    let mut stderr = stderr();
    if stderr.write_all(query.as_bytes()).is_err() {
        let _ = disable_raw_mode();
        return None;
    }
    if stderr.flush().is_err() {
        let _ = disable_raw_mode();
        return None;
    }

    // Read response with timeout
    let result = read_osc_response(Duration::from_millis(100));

    // Restore terminal mode
    let _ = disable_raw_mode();

    // Parse response: \x1b]N;rgb:RRRR/GGGG/BBBB\x1b\\
    if let Some(response) = result {
        parse_osc_color_response(&response)
    } else {
        None
    }
}

/// Query a single terminal color using OSC 4 escape sequence
/// Returns None if the query fails or times out
fn query_terminal_color(color_idx: usize) -> Option<(u8, u8, u8)> {
    use std::io::stderr;
    use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

    // OSC 4 ; color ; ? BEL (using ST terminator for better compatibility)
    let query = format!("\x1b]4;{};?\x1b\\", color_idx);

    // Enable raw mode to read response
    if enable_raw_mode().is_err() {
        return None;
    }

    // Send query to stderr (where terminal control sequences go)
    let mut stderr = stderr();
    if stderr.write_all(query.as_bytes()).is_err() {
        let _ = disable_raw_mode();
        return None;
    }
    if stderr.flush().is_err() {
        let _ = disable_raw_mode();
        return None;
    }

    // Read response with timeout
    let result = read_osc_response(Duration::from_millis(100));

    // Restore terminal mode
    let _ = disable_raw_mode();

    // Parse response: \x1b]4;N;rgb:RRRR/GGGG/BBBB\x1b\\
    if let Some(response) = result {
        parse_osc_color_response(&response)
    } else {
        None
    }
}

/// Read OSC response from stdin with timeout
fn read_osc_response(timeout: Duration) -> Option<String> {
    use std::io::stdin;
    use std::thread;
    use std::sync::mpsc::channel;

    let (tx, rx) = channel();

    // Spawn thread to read response
    thread::spawn(move || {
        let mut stdin = stdin();
        let mut buffer = Vec::new();
        let mut temp = [0u8; 1];

        // Read until we get ST (\x1b\\) or BEL (\x07)
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

    // Wait for response with timeout
    rx.recv_timeout(timeout).ok()
}

/// Parse OSC 4 color response: rgb:RRRR/GGGG/BBBB
fn parse_osc_color_response(response: &str) -> Option<(u8, u8, u8)> {
    // Look for "rgb:" in the response
    if let Some(rgb_start) = response.find("rgb:") {
        let rgb_part = &response[rgb_start + 4..];

        // Split by '/' to get R/G/B components
        let parts: Vec<&str> = rgb_part.split('/').collect();
        if parts.len() >= 3 {
            // Parse hex values (they're typically 4 digits: RRRR/GGGG/BBBB)
            // We want 8-bit values, so take the high byte
            let r = u16::from_str_radix(parts[0].trim_end_matches(|c: char| !c.is_ascii_hexdigit()), 16).ok()?;
            let g = u16::from_str_radix(parts[1].trim_end_matches(|c: char| !c.is_ascii_hexdigit()), 16).ok()?;
            let b = u16::from_str_radix(parts[2].trim_end_matches(|c: char| !c.is_ascii_hexdigit()), 16).ok()?;

            // Convert 16-bit to 8-bit (take high byte)
            return Some(((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8));
        }
    }

    None
}
