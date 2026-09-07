use crossterm::style::Color;
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

impl Cell {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub fn blank() -> Self {
        Cell {
            ch: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
        }
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub fn new(ch: char, fg: Color) -> Self {
        Cell {
            ch,
            fg,
            bg: Color::Reset,
        }
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub fn with_bg(ch: char, fg: Color, bg: Color) -> Self {
        Cell { ch, fg, bg }
    }
}

pub type Grid = Vec<Vec<Cell>>;

/// Display width of a string (accounts for fullwidth CJK chars).
#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub fn display_width(s: &str) -> usize {
    s.chars().map(|c| c.width().unwrap_or(0)).sum()
}

/// Display width of a single char.
#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub fn char_width(c: char) -> usize {
    c.width().unwrap_or(0)
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
