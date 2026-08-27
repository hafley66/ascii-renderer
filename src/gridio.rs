#![allow(warnings)]

use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

use crate::automata::*;
use crate::biomes::*;
use crate::color::*;
use crate::content::*;
use crate::fills::*;
use crate::layout::*;
use crate::markdown::*;
use crate::mondrian::*;
use crate::render::*;
use crate::scene::*;
use crate::sprites::*;
use crate::tree_draw::*;
use crate::types::*;
use crate::walker::*;
use crate::avant::*;
use crate::automata; use crate::avant; use crate::biomes; use crate::borders; use crate::color; use crate::content; use crate::fills; use crate::layout; use crate::markdown; use crate::mondrian; use crate::render; use crate::scene; use crate::sprites; use crate::tree_draw; use crate::types; use crate::walker;
use crate::cli::*;
use crate::ink::*;
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;

/// Render path used by every mode: dump a serialized grid when ASCII_GRID_DUMP
/// is set (for the morph driver to capture), otherwise paint to the terminal.
pub(crate) fn emit_grid(grid: &Grid) {
    use std::io::Write;
    if std::env::var("ASCII_GRID_DUMP").is_ok() {
        let s = serialize_grid(grid);
        let mut out = io::stdout().lock();
        let _ = out.write_all(s.as_bytes());
        let _ = out.flush();
    } else {
        render_grid(grid);
    }
}

pub(crate) fn grid_color_code(c: Color) -> String {
    match c {
        Color::Rgb { r, g, b } => format!("{},{},{}", r, g, b),
        _ => "x".to_string(),
    }
}

pub(crate) fn parse_color_code(s: &str) -> Color {
    if s == "x" {
        return Color::Reset;
    }
    let mut it = s.split(',');
    let r = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let g = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let b = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    Color::Rgb { r, g, b }
}


/// Lossless text serialization: "w h" header, then one "char_u32 fg bg" line per cell.
pub(crate) fn serialize_grid(grid: &Grid) -> String {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { 0 };
    let mut s = String::with_capacity(w * h * 12 + 16);
    s.push_str(&format!("{} {}\n", w, h));
    for row in grid {
        for c in row {
            s.push_str(&format!(
                "{} {} {}\n",
                c.ch as u32,
                grid_color_code(c.fg),
                grid_color_code(c.bg)
            ));
        }
    }
    s
}

pub(crate) fn parse_grid(s: &str) -> Grid {
    let mut lines = s.lines();
    let header = lines.next().unwrap_or("0 0");
    let mut hi = header.split_whitespace();
    let w: usize = hi.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let h: usize = hi.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut grid = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        for x in 0..w {
            if let Some(line) = lines.next() {
                let mut p = line.split_whitespace();
                let ch = p
                    .next()
                    .and_then(|v| v.parse::<u32>().ok())
                    .and_then(char::from_u32)
                    .unwrap_or(' ');
                let fg = parse_color_code(p.next().unwrap_or("x"));
                let bg = parse_color_code(p.next().unwrap_or("x"));
                grid[y][x] = Cell::with_bg(ch, fg, bg);
            }
        }
    }
    grid
}


/// Force a grid to (w, h) by truncating / padding with blanks.
pub(crate) fn fit_grid(g: Grid, w: usize, h: usize) -> Grid {
    let mut out = vec![vec![Cell::blank(); w]; h];
    for y in 0..h.min(g.len()) {
        for x in 0..w.min(g[y].len()) {
            out[y][x] = g[y][x];
        }
    }
    out
}

/// Coerce any color to an Rgb so lerp_color interpolates instead of snapping.
pub(crate) fn rgb_of(c: Color) -> Color {
    match c {
        Color::Rgb { .. } => c,
        _ => Color::Rgb { r: 10, g: 10, b: 12 },
    }
}


/// Paint a grid with each row positioned by an absolute cursor escape and NO
/// newlines, so the terminal can never scroll (the definitive anti-scrollback
/// measure for the morph player).
/// Write the SGR escape for a color directly into `s` (no per-call String alloc,
/// unlike crossterm's `SetForegroundColor(..).to_string()`). `fg` selects the
/// foreground (38/39) vs background (48/49) parameter group.
pub(crate) fn write_sgr(s: &mut String, c: Color, fg: bool) {
    use std::fmt::Write as _;
    match c {
        Color::Rgb { r, g, b } => {
            let lead = if fg { 38 } else { 48 };
            let _ = write!(s, "\x1b[{};2;{};{};{}m", lead, r, g, b);
        }
        Color::Reset => s.push_str(if fg { "\x1b[39m" } else { "\x1b[49m" }),
        other => {
            // rare named/ansi variants: fall back to crossterm's formatter.
            use crossterm::style::{SetBackgroundColor, SetForegroundColor};
            if fg {
                let _ = write!(s, "{}", SetForegroundColor(other));
            } else {
                let _ = write!(s, "{}", SetBackgroundColor(other));
            }
        }
    }
}

pub(crate) fn grid_to_ansi(grid: &Grid) -> String {
    use std::fmt::Write as _;
    // preallocate roughly enough for chars + cursor escapes + some color runs.
    let approx = grid.len() * (grid.first().map_or(0, |r| r.len()) + 8) + 64;
    let mut s = String::with_capacity(approx);
    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;
    for (y, row) in grid.iter().enumerate() {
        let _ = write!(s, "\x1b[{};1H", y + 1); // home of this row (1-based)
        let mut skip = false;
        for cell in row {
            if skip {
                skip = false;
                continue;
            }
            if cell.fg != cur_fg {
                write_sgr(&mut s, cell.fg, true);
                cur_fg = cell.fg;
            }
            if cell.bg != cur_bg {
                write_sgr(&mut s, cell.bg, false);
                cur_bg = cell.bg;
            }
            s.push(cell.ch);
            if char_width(cell.ch) == 2 {
                skip = true;
            }
        }
        if cur_bg != Color::Reset {
            write_sgr(&mut s, Color::Reset, false);
            cur_bg = Color::Reset;
        }
    }
    s.push_str("\x1b[0m");
    s
}

