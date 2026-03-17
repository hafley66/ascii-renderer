use crossterm::style::Color;
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

impl Cell {
    pub fn blank() -> Self {
        Cell {
            ch: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
        }
    }

    pub fn new(ch: char, fg: Color) -> Self {
        Cell {
            ch,
            fg,
            bg: Color::Reset,
        }
    }

    pub fn with_bg(ch: char, fg: Color, bg: Color) -> Self {
        Cell { ch, fg, bg }
    }
}

pub type Grid = Vec<Vec<Cell>>;

/// Display width of a string (accounts for fullwidth CJK chars).
pub fn display_width(s: &str) -> usize {
    s.chars().map(|c| c.width().unwrap_or(0)).sum()
}

/// Display width of a single char.
pub fn char_width(c: char) -> usize {
    c.width().unwrap_or(0)
}

pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
