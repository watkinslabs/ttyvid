use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CellFlags: u8 {
        const BOLD = 0b00000001;
        const ITALIC = 0b00000010;
        const UNDERLINE = 0b00000100;
        const REVERSE = 0b00001000;
        const BLINK = 0b00010000;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Cell {
    pub character: char,
    pub fg_color: u8,
    pub bg_color: u8,
    pub flags: CellFlags,
}

impl Cell {
    pub fn new(character: char, fg_color: u8, bg_color: u8, flags: CellFlags) -> Self {
        Self {
            character,
            fg_color,
            bg_color,
            flags,
        }
    }

    pub fn empty(fg_color: u8, bg_color: u8) -> Self {
        Self {
            character: ' ',
            fg_color,
            bg_color,
            flags: CellFlags::empty(),
        }
    }
}
