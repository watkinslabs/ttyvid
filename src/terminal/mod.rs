mod cell;
mod grid;
mod state;
pub mod parser;

pub use cell::{Cell, CellFlags};
pub use grid::Grid;
pub use state::TerminalState;

use parser::{Event, Command, EscapeType, parse_ansi_stream};
use crate::renderer::Palette;

pub struct TerminalEmulator {
    grid: Grid,
    state: TerminalState,
    alt_grid: Grid,
    alt_state: TerminalState,
    display_alt_screen: Option<bool>,  // None=main, Some(true)=alt
    extra_text: String,  // Buffer for partial escape sequences across events
    palette: Palette,
}

impl TerminalEmulator {
    pub fn new(width: usize, height: usize, auto_wrap: bool, default_fg: u8, default_bg: u8) -> Self {
        let mut state = TerminalState::new(width, height, default_fg, default_bg);
        let mut alt_state = TerminalState::new(width, height, default_fg, default_bg);
        if !auto_wrap {
            state.autowrap_off();
            alt_state.autowrap_off();
        }

        Self {
            grid: Grid::new(width, height, default_fg, default_bg),
            state,
            alt_grid: Grid::new(width, height, default_fg, default_bg),
            alt_state,
            display_alt_screen: None,
            extra_text: String::new(),
            palette: Palette::default(),
        }
    }

    // Exact translation of parser.pyx has_escape lines 367-371
    fn has_escape(&self, text: &str) -> bool {
        for ch in text.chars() {
            if ch as u32 == 0x1B {
                return true;
            }
        }
        false
    }

    // Exact translation of parser.pyx add_event + stream_2_sequence lines 373-360
    pub fn feed_bytes(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        // Line 386: self.stream_2_sequence(self.extra_text+event_io,timestamp,0)
        let full_text = format!("{}{}", self.extra_text, text);

        // Use parser module but we need to track the last parsed position
        // We'll use parse_ansi_stream_with_position which we need to create
        let (events, last_pos) = parser::parse_ansi_stream_with_position(&full_text);

        // Process all parsed events
        for event in events {
            match event {
                Event::Text(chars) => self.cmd_render_text(&chars),
                Event::Command(cmd) => self.process_command(cmd),
            }
        }

        // Lines 355-360: Check if remaining text contains escape
        let remaining = &full_text[last_pos..];

        if self.has_escape(remaining) {
            // Line 356: self.extra_text=text[cursor:]
            self.extra_text = remaining.to_string();
        } else {
            // Line 359-360
            self.extra_text.clear();
            if !remaining.is_empty() {
                let chars: Vec<char> = remaining.chars().collect();
                self.cmd_render_text(&chars);
            }
        }
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn state(&self) -> &TerminalState {
        &self.state
    }

    // Exact translation of terminal_graphics.pyx alternate_screen_on lines 77-89
    fn alternate_screen_on(&mut self) {
        if self.display_alt_screen.is_none() {
            self.display_alt_screen = Some(true);
            // Swap grids
            std::mem::swap(&mut self.grid, &mut self.alt_grid);
            // Swap states
            std::mem::swap(&mut self.state, &mut self.alt_state);
        }
    }

    // Exact translation of terminal_graphics.pyx alternate_screen_off lines 91-103
    fn alternate_screen_off(&mut self) {
        if self.display_alt_screen == Some(true) {
            self.display_alt_screen = None;
            // Swap grids back
            std::mem::swap(&mut self.grid, &mut self.alt_grid);
            // Swap states back
            std::mem::swap(&mut self.state, &mut self.alt_state);
        }
    }

    // Exact translation of parser.pyx cmd_render_text lines 59-90
    fn cmd_render_text(&mut self, data: &[char]) {
        const BS: u32 = 8;   // Backspace
        const FI: u32 = 9;   // Forward Index
        const LF: u32 = 10;  // Line feed
        const CR: u32 = 13;  // Carriage return

        self.state.text_mode_on();

        for &character in data {
            // Line 68-70: while self.g.state.scroll!=0:
            while self.state.scroll != 0 {
                self.scroll_buffer();
            }

            let char_ord = character as u32;

            // Line 73-84: if char_ord<32 and self.no_codes==None:
            if char_ord < 32 {
                if char_ord == BS {
                    self.state.cursor_left(1);
                } else if char_ord == FI {
                    self.state.cursor_right(1);
                } else if char_ord == LF {
                    self.state.cursor_down(1);
                    if self.state.mode == "linux" {
                        self.state.cursor_absolute_x(0);
                    }
                } else if char_ord == CR {
                    self.state.cursor_absolute_x(0);
                }
            } else {
                // Line 86-89
                if self.state.pending_wrap {
                    self.state.cursor_right(1);
                }
                self.write(char_ord);
                self.state.cursor_right(1);
            }
        }

        self.state.text_mode_off();
    }

    // Write a character - like terminal_graphics.write()
    fn write(&mut self, char_ord: u32) {
        let mut fg = self.state.foreground;
        let mut bg = self.state.background;

        // Apply reverse video
        if self.state.reverse_video {
            std::mem::swap(&mut fg, &mut bg);
        }

        let character = if char_ord <= 0xFFFF {
            std::char::from_u32(char_ord).unwrap_or(' ')
        } else {
            ' '
        };

        let cell = Cell::new(character, fg as u8, bg as u8, self.state.flags);
        self.grid.write_cell(self.state.cursor_x as usize, self.state.cursor_y as usize, cell);
    }

    fn scroll_buffer(&mut self) {
        let amount = self.state.scroll.abs() as usize;
        let fg = self.state.foreground as u8;
        let bg = self.state.background as u8;

        if self.state.scroll > 0 {
            // Scroll up
            self.grid.scroll_region_up(
                self.state.scroll_top as usize,
                self.state.scroll_bottom as usize,
                amount,
                fg,
                bg,
            );
        } else {
            // Scroll down
            self.grid.scroll_region_down(
                self.state.scroll_top as usize,
                self.state.scroll_bottom as usize,
                amount,
                fg,
                bg,
            );
        }

        self.state.scroll = 0;
    }

    fn process_command(&mut self, cmd: Command) {
        match cmd.esc_type {
            EscapeType::Single => self.process_single(&cmd.command),
            EscapeType::Csi => self.process_csi(&cmd.command, &cmd.params),
            EscapeType::CharSet | EscapeType::G0 | EscapeType::G1 | EscapeType::Osc | EscapeType::Title => {
                // Ignore
            }
            EscapeType::BracketPaste => {
                // Ignore
            }
        }
    }

    // Exact translation of parser.pyx process_SINGLE lines 241-247
    fn process_single(&mut self, command: &str) {
        if command == "7" {
            self.state.cursor_save_position();
        }
        if command == "8" {
            self.state.cursor_restore_position();
        }
    }

    // Exact translation of parser.pyx process_CSI lines 260-317
    fn process_csi(&mut self, command: &str, params: &[i32]) {
        let param_len = params.len();
        let mut value1 = 0;
        let mut value2 = 0;

        // Exact translation of lines 266-278
        if param_len > 0 {
            value1 = params[0];
        } else if command == "r" {
            value1 = 1;
        }

        if param_len > 1 {
            value2 = params[1];
        } else if command == "r" {
            value2 = self.state.height - 1;
        }

        // Exact translation of lines 285-317
        if command == "A" {
            self.cmd_cuu(value1);
        } else if command == "B" {
            self.cmd_cud(value1);
        } else if command == "C" {
            self.cmd_cuf(value1);
        } else if command == "D" {
            self.cmd_cub(value1);
        } else if command == "E" {
            self.cmd_cnl(value1);
        } else if command == "F" {
            self.cmd_cpl(value1);
        } else if command == "G" {
            self.cmd_cha(value1 - 1);
        } else if command == "H" {
            self.cmd_cup(value2 - 1, value1 - 1);
        } else if command == "J" {
            self.cmd_ed(value1);
        } else if command == "K" {
            self.cmd_el(value1);
        } else if command == "P" {
            self.cmd_dch(value1);
        } else if command == "X" {
            self.cmd_ech(value1);
        } else if command == "d" {
            self.cmd_vpa(value1 - 1);
        } else if command == "`" {
            self.cmd_hpa(value1 - 1);
        } else if command == "f" {
            self.cmd_hvp(value2 - 1, value1 - 1);
        } else if command == "l" {
            self.cmd_reset_mode(value1);
        } else if command == "m" {
            self.cmd_process_colors(params);
        } else if command == "r" {
            self.cmd_decstbm(value1 - 1, value2 - 1);
        } else if command == "s" {
            self.cmd_scp();
        } else if command == "u" {
            self.cmd_rcp();
        } else if command == "~" {
            self.cmd_bracketed_paste(value1);
        } else if command == "?h" {
            self.cmd_decset(value1);
        } else if command == "?l" {
            self.cmd_decrst(value1);
        }
    }

    // Exact translation of cmd_DECSET lines 320-329 and 36-45
    fn cmd_decset(&mut self, code: i32) {
        if code == 7 {
            self.state.autowrap_on();
        } else if code == 25 {
            self.state.show_cursor();
        } else if code == 1049 {
            self.alternate_screen_on();
        } else if code == 2004 {
            // bracketed paste - ignore
        }
    }

    // Exact translation of cmd_DECRST lines 331-340 and 47-56
    fn cmd_decrst(&mut self, code: i32) {
        if code == 7 {
            self.state.autowrap_off();
        } else if code == 25 {
            self.state.hide_cursor();
        } else if code == 1049 {
            self.alternate_screen_off();
        } else if code == 2004 {
            // bracketed paste - ignore
        }
    }

    // Exact translation of cmd_bracketed_paste lines 348-356
    fn cmd_bracketed_paste(&mut self, _value: i32) {
        // bracketed paste handling - ignore for now
    }

    // Exact translation of cmd_set_mode lines 359-392
    fn cmd_set_mode(&mut self, cmd: i32) {
        if cmd == 0 {
            self.state.set_foreground(self.state.default_foreground);
            self.state.set_background(self.state.default_background);
            self.state.bold = false;
            self.state.reverse_video = false;
        } else if cmd == 1 {
            self.state.bold = true;
        } else if cmd == 7 {
            self.state.reverse_video = true;
        } else if cmd == 22 {
            self.state.bold = false;
        } else if cmd == 27 {
            self.state.reverse_video = false;
        } else if cmd >= 30 && cmd <= 37 {
            if self.state.bold {
                self.set_foreground(cmd - 30 + 8);
            } else {
                self.set_foreground(cmd - 30);
            }
        } else if cmd == 39 {
            self.state.set_foreground(self.state.default_foreground);
        } else if cmd >= 40 && cmd <= 47 {
            if self.state.bold {
                self.set_background(cmd - 40 + 8);
            } else {
                self.set_background(cmd - 40);
            }
        } else if cmd == 49 {
            self.state.set_background(self.state.default_background);
        } else if cmd >= 90 && cmd <= 97 {
            self.set_foreground(cmd - 90 + 8);
        } else if cmd >= 100 && cmd <= 107 {
            self.set_background(cmd - 100 + 8);
        }
    }

    // Exact translation of cmd_reset_mode lines 394-403
    fn cmd_reset_mode(&mut self, cmd: i32) {
        if cmd == 0 {
            self.state.set_foreground(self.state.default_foreground);
            self.state.set_background(self.state.default_background);
            self.state.bold = false;
            self.state.reverse_video = false;
        } else if cmd == 1 {
            self.state.bold = false;
        } else if cmd == 7 {
            self.state.reverse_video = false;
        }
    }

    fn set_foreground(&mut self, color: i32) {
        if color >= 256 {
            self.state.set_foreground(self.state.default_foreground);
        } else {
            self.state.set_foreground(color);
        }
    }

    fn set_background(&mut self, color: i32) {
        if color >= 256 {
            self.state.set_background(self.state.default_background);
        } else {
            self.state.set_background(color);
        }
    }

    // Exact translation of cmd_process_colors lines 419-437
    fn cmd_process_colors(&mut self, params: &[i32]) {
        if params.is_empty() {
            return;
        }

        if params[0] == 38 {
            if params.len() > 1 && params[1] == 2 && params.len() > 4 {
                // RGB foreground - ESC[38;2;R;G;Bm
                let r = params[2];
                let g = params[3];
                let b = params[4];
                let color = self.palette.match_color_index(r, g, b);
                self.set_foreground(color as i32);
            }
            if params.len() > 2 && params[1] == 5 {
                self.set_foreground(params[2]);
            }
        } else if params[0] == 48 {
            if params.len() > 1 && params[1] == 2 && params.len() > 4 {
                // RGB background - ESC[48;2;R;G;Bm
                let r = params[2];
                let g = params[3];
                let b = params[4];
                let color = self.palette.match_color_index(r, g, b);
                self.set_background(color as i32);
            }
            if params.len() > 2 && params[1] == 5 {
                self.set_background(params[2]);
            }
        } else {
            for &cmd in params {
                self.cmd_set_mode(cmd);
            }
        }
    }

    // Exact translation of cmd_DECSTBM lines 441-442
    fn cmd_decstbm(&mut self, top: i32, bottom: i32) {
        self.state.set_scroll_region(top, bottom);
    }

    // Exact translation of cmd_CUU lines 444-445
    fn cmd_cuu(&mut self, distance: i32) {
        self.state.cursor_up(distance);
    }

    // Exact translation of cmd_CUD lines 447-448
    fn cmd_cud(&mut self, distance: i32) {
        self.state.cursor_down(distance);
    }

    // Exact translation of cmd_CUB lines 450-451
    fn cmd_cub(&mut self, distance: i32) {
        self.state.cursor_left(distance);
    }

    // Exact translation of cmd_CUF lines 453-454
    fn cmd_cuf(&mut self, distance: i32) {
        self.state.cursor_right(distance);
    }

    // Exact translation of cmd_CPL lines 456-458
    fn cmd_cpl(&mut self, distance: i32) {
        self.state.cursor_absolute_x(0);
        self.state.cursor_up(distance);
    }

    // Exact translation of cmd_CNL lines 460-462
    fn cmd_cnl(&mut self, distance: i32) {
        self.state.cursor_absolute_x(0);
        self.state.cursor_up(distance); // NOTE: Python has bug here, says cursor_up but should be cursor_down
    }

    // Exact translation of cmd_CHA lines 464-465
    fn cmd_cha(&mut self, x: i32) {
        self.state.cursor_absolute_x(x);
    }

    // Exact translation of cmd_CUP lines 467-468
    fn cmd_cup(&mut self, x: i32, y: i32) {
        self.state.cursor_absolute(x, y);
    }

    // Exact translation of cmd_ED lines 470-495
    fn cmd_ed(&mut self, mode: i32) {
        if mode == 1 {
            let cp = self.state.cursor_get_position();
            for x in 0..=self.state.cursor_x {
                self.state.cursor_absolute_x(x);
                self.write(0);
            }
            for y in 0..self.state.cursor_y - 1 {
                for x in 0..self.state.width {
                    self.state.cursor_absolute(x, y);
                    self.write(0);
                }
            }
            self.state.cursor_absolute(cp.0, cp.1);
        }
        if mode == 0 {
            let cp = self.state.cursor_get_position();
            for x in self.state.cursor_x..self.state.width {
                self.state.cursor_absolute_x(x);
                self.write(0);
            }
            for y in (self.state.cursor_y + 1)..self.state.height {
                for x in 0..self.state.width {
                    self.state.cursor_absolute(x, y);
                    self.write(0);
                }
            }
            self.state.cursor_absolute(cp.0, cp.1);
        }
        if mode == 2 {
            self.grid.clear(self.state.foreground as u8, self.state.background as u8);
        }
    }

    // Exact translation of cmd_EL lines 497-512
    fn cmd_el(&mut self, mode: i32) {
        let cp = self.state.cursor_get_position();
        if mode == 0 {
            for x in self.state.cursor_x..self.state.width {
                self.state.cursor_absolute_x(x);
                self.write(0);
            }
        } else if mode == 1 {
            for x in 0..=self.state.cursor_x {
                self.state.cursor_absolute_x(x);
                self.write(0);
            }
        } else if mode == 2 {
            for x in 0..self.state.width {
                self.state.cursor_absolute_x(x);
                self.write(0);
            }
        }
        self.state.cursor_absolute(cp.0, cp.1);
    }

    // Exact translation of cmd_DCH lines 514-528
    fn cmd_dch(&mut self, distance: i32) {
        let x = self.state.cursor_x;
        let y = self.state.cursor_y;
        let width = self.state.width;

        // Copy elements to buffer
        for x2 in (x + distance)..width {
            if let Some(cell) = self.grid.get_cell(x2 as usize, y as usize).cloned() {
                self.grid.write_cell((x2 - distance) as usize, y as usize, cell);
            }
        }

        // Clear the end of the line
        for x2 in (width - distance)..width {
            let cell = Cell::empty(self.state.foreground as u8, self.state.background as u8);
            self.grid.write_cell(x2 as usize, y as usize, cell);
        }
    }

    // Exact translation of cmd_ECH lines 530-535
    fn cmd_ech(&mut self, distance: i32) {
        let cp = self.state.cursor_get_position();
        for x in self.state.cursor_x..(self.state.cursor_x + distance) {
            self.state.cursor_absolute_x(x);
            self.write(0);
        }
        self.state.cursor_absolute(cp.0, cp.1);
    }

    // Exact translation of cmd_HVP lines 537-538
    fn cmd_hvp(&mut self, x: i32, y: i32) {
        self.state.cursor_absolute(x, y);
    }

    // Exact translation of cmd_HPA lines 540-541
    fn cmd_hpa(&mut self, x: i32) {
        self.state.cursor_absolute_x(x);
    }

    // Exact translation of cmd_SCP lines 543-544
    fn cmd_scp(&mut self) {
        self.state.cursor_save_position();
    }

    // Exact translation of cmd_RCP lines 546-547
    fn cmd_rcp(&mut self) {
        self.state.cursor_restore_position();
    }

    // Exact translation of cmd_VPA lines 549-550
    fn cmd_vpa(&mut self, position: i32) {
        self.state.cursor_absolute(0, position);
    }
}
