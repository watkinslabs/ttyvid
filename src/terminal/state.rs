use super::CellFlags;

pub struct TerminalState {
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub width: i32,
    pub height: i32,
    pub mode: String,
    pub reverse_video: bool,
    pub bold: bool,
    pub text_mode: bool,
    pub autowrap: bool,
    pub foreground: i32,
    pub background: i32,
    pub default_foreground: i32,
    pub default_background: i32,
    pub pending_wrap: bool,
    pub cursor_speed: i32,
    pub display_cursor: bool,

    // Scroll region
    pub scroll: i32,
    pub scroll_top: i32,
    pub scroll_bottom: i32,

    // Saved cursor
    pub saved_cursor_x: i32,
    pub saved_cursor_y: i32,

    pub flags: CellFlags,
}

impl TerminalState {
    pub fn new(width: usize, height: usize, default_fg: u8, default_bg: u8) -> Self {
        let width = width as i32;
        let height = height as i32;

        Self {
            cursor_x: 0,
            cursor_y: 0,
            width,
            height,
            mode: "linux".to_string(),
            reverse_video: false,
            bold: false,
            text_mode: false,
            autowrap: true,
            foreground: default_fg as i32,
            background: default_bg as i32,
            default_foreground: default_fg as i32,
            default_background: default_bg as i32,
            pending_wrap: false,
            cursor_speed: 0,
            display_cursor: false,
            scroll: 0,
            scroll_top: 0,
            scroll_bottom: height - 1,
            saved_cursor_x: 0,
            saved_cursor_y: 0,
            flags: CellFlags::empty(),
        }
    }

    pub fn text_mode_on(&mut self) {
        self.text_mode = true;
    }

    pub fn text_mode_off(&mut self) {
        self.text_mode = false;
    }

    pub fn autowrap_on(&mut self) {
        self.autowrap = true;
    }

    pub fn autowrap_off(&mut self) {
        self.autowrap = false;
    }

    pub fn set_scroll_region(&mut self, top: i32, bottom: i32) {
        self.scroll = 0;
        self.scroll_top = top;
        self.scroll_bottom = bottom;
    }

    pub fn show_cursor(&mut self) {
        self.display_cursor = true;
    }

    pub fn hide_cursor(&mut self) {
        self.display_cursor = false;
    }

    // Exact translation of Python check_bounds lines 58-77
    fn check_bounds(&mut self) {
        if self.pending_wrap {
            if self.cursor_x != self.width - 1 || self.cursor_y != self.height - 1 || !self.autowrap {
                self.pending_wrap = false;
            }
        }

        if self.cursor_x < 0 {
            self.cursor_x = 0;
        }

        if self.cursor_x >= self.width {
            self.cursor_x = self.width - 1;
        }

        if self.cursor_y < self.scroll_top {
            if self.text_mode {
                self.scroll -= self.scroll_top - self.cursor_y; // negative
            }
            self.cursor_y = self.scroll_top;
        }

        if self.cursor_y > self.scroll_bottom {
            if self.text_mode {
                self.scroll += self.cursor_y - self.scroll_bottom; // positive
            }
            self.cursor_y = self.scroll_bottom;
        }
    }

    // Exact translation of Python cursor_up lines 79-81
    pub fn cursor_up(&mut self, distance: i32) {
        self.cursor_y -= distance;
        self.check_bounds();
    }

    // Exact translation of Python cursor_down lines 83-85
    pub fn cursor_down(&mut self, distance: i32) {
        self.cursor_y += distance;
        self.check_bounds();
    }

    // Exact translation of Python cursor_left lines 87-89
    pub fn cursor_left(&mut self, distance: i32) {
        self.cursor_x -= distance;
        self.check_bounds();
    }

    // Exact translation of Python cursor_right lines 91-104
    pub fn cursor_right(&mut self, distance: i32) {
        // Line 93: if self.pending_wrap==None and self.autowrap and self.cursor_x==self.width-1:
        if !self.pending_wrap && self.autowrap && self.cursor_x == self.width - 1 {
            self.pending_wrap = true;
        } else {
            self.cursor_x += distance;
            if self.autowrap {
                while self.cursor_x >= self.width {
                    self.cursor_x = self.cursor_x - self.width;
                    self.cursor_down(1);
                }
            }
            self.check_bounds();
        }
    }

    // Exact translation of Python cursor_absolute_x lines 106-108
    pub fn cursor_absolute_x(&mut self, position: i32) {
        self.cursor_x = position;
        self.check_bounds();
    }

    // Exact translation of Python cursor_absolute_y lines 110-112
    pub fn cursor_absolute_y(&mut self, position: i32) {
        self.cursor_y = position;
        self.check_bounds();
    }

    // Exact translation of Python cursor_absolute lines 114-117
    pub fn cursor_absolute(&mut self, position_x: i32, position_y: i32) {
        self.cursor_x = position_x;
        self.cursor_y = position_y;
        self.check_bounds();
    }

    // Exact translation of Python cursor_save_position lines 119-121
    pub fn cursor_save_position(&mut self) {
        self.saved_cursor_x = self.cursor_x;
        self.saved_cursor_y = self.cursor_y;
    }

    // Exact translation of Python cursor_restore_position lines 123-125
    pub fn cursor_restore_position(&mut self) {
        self.cursor_x = self.saved_cursor_x;
        self.cursor_y = self.saved_cursor_y;
    }

    // Exact translation of Python cursor_get_position lines 127-128
    pub fn cursor_get_position(&self) -> (i32, i32) {
        (self.cursor_x, self.cursor_y)
    }

    // Exact translation of Python set_background lines 130-133
    pub fn set_background(&mut self, color: i32) {
        if color > 255 {
            panic!("Color over maximum value");
        }
        self.background = color;
    }

    // Exact translation of Python set_foreground lines 135-138
    pub fn set_foreground(&mut self, color: i32) {
        if color > 255 {
            panic!("Color over maximum value");
        }
        self.foreground = color;
    }
}
