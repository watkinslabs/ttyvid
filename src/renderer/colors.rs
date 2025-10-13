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
