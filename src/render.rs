use crossterm::style::Color;
use std::io::{self, Write};
use crate::types::*;

/// Render grid to plain text (no ANSI escapes).
pub fn grid_to_plain(grid: &Grid) -> Vec<String> {
    let mut lines = Vec::with_capacity(grid.len());
    for row in grid {
        let mut line = String::with_capacity(row.len());
        let mut skip_next = false;
        for cell in row {
            if skip_next {
                skip_next = false;
                continue;
            }
            line.push(cell.ch);
            if char_width(cell.ch) == 2 {
                skip_next = true;
            }
        }
        lines.push(line);
    }
    lines
}

/// Print the grid with ANSI color escape sequences.
pub fn render_grid(grid: &Grid) {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;

    for row in grid {
        let mut skip_next = false;
        for cell in row {
            if skip_next {
                skip_next = false;
                continue;
            }
            if cell.fg != cur_fg {
                write!(out, "{}", crossterm::style::SetForegroundColor(cell.fg)).unwrap();
                cur_fg = cell.fg;
            }
            if cell.bg != cur_bg {
                write!(out, "{}", crossterm::style::SetBackgroundColor(cell.bg)).unwrap();
                cur_bg = cell.bg;
            }
            write!(out, "{}", cell.ch).unwrap();
            if char_width(cell.ch) == 2 {
                skip_next = true;
            }
        }
        if cur_bg != Color::Reset {
            write!(out, "{}", crossterm::style::SetBackgroundColor(Color::Reset)).unwrap();
            cur_bg = Color::Reset;
        }
        writeln!(out).unwrap();
    }

    write!(out, "{}", crossterm::style::ResetColor).unwrap();
    out.flush().unwrap();
}
