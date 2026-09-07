#![allow(warnings)]

use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

use crate::automata;
use crate::automata::*;
use crate::avant;
use crate::avant::*;
use crate::biomes;
use crate::biomes::*;
use crate::borders;
use crate::cli::*;
use crate::color;
use crate::color::*;
use crate::content;
use crate::content::*;
use crate::fills;
use crate::fills::*;
use crate::ink::*;
use crate::layout;
use crate::layout::*;
use crate::markdown;
use crate::markdown::*;
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::mondrian;
use crate::mondrian::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::render;
use crate::render::*;
use crate::scene;
use crate::scene::*;
use crate::sprites;
use crate::sprites::*;
use crate::tree_draw;
use crate::tree_draw::*;
use crate::types;
use crate::types::*;
use crate::walker;
use crate::walker::*;
use crate::warps::*;

/// Render path used by every mode: dump a serialized grid when ASCII_GRID_DUMP
/// is set (for the morph driver to capture), otherwise paint to the terminal.
#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn grid_color_code(c: Color) -> String {
    match c {
        Color::Rgb { r, g, b } => format!("{},{},{}", r, g, b),
        _ => "x".to_string(),
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn rgb_of(c: Color) -> Color {
    match c {
        Color::Rgb { .. } => c,
        _ => Color::Rgb {
            r: 10,
            g: 10,
            b: 12,
        },
    }
}

/// Paint a grid with each row positioned by an absolute cursor escape and NO
/// newlines, so the terminal can never scroll (the definitive anti-scrollback
/// measure for the morph player).
/// Write the SGR escape for a color directly into `s` (no per-call String alloc,
/// unlike crossterm's `SetForegroundColor(..).to_string()`). `fg` selects the
/// foreground (38/39) vs background (48/49) parameter group.
#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn write_sgr(s: &mut String, c: Color, fg: bool) {
    crate::render::push_color(s, c, fg);
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
            if cell.ch != ' ' && cell.fg != cur_fg {
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FrameEncodeStats {
    pub(crate) bytes: usize,
    pub(crate) changed_cells: usize,
    pub(crate) runs: usize,
    pub(crate) full_repaint: bool,
}

/// Adapts the art grid to Ratatui's retained buffers and Crossterm backend.
/// Ratatui owns cell comparison, wide-character handling and ANSI generation.
/// Output stays in a reusable byte buffer for the cancellable playback relay.
pub(crate) struct AnsiFrameEncoder {
    previous: ratatui::buffer::Buffer,
    current: ratatui::buffer::Buffer,
    bytes: Vec<u8>,
    initialized: bool,
}

impl AnsiFrameEncoder {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn new() -> Self {
        // This encoder creates colored art payloads even when stdout is a pipe.
        ratatui::crossterm::style::force_color_output(true);
        Self {
            previous: ratatui::buffer::Buffer::empty(ratatui::layout::Rect::default()),
            current: ratatui::buffer::Buffer::empty(ratatui::layout::Rect::default()),
            bytes: Vec::new(),
            initialized: false,
        }
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn invalidate(&mut self) {
        self.initialized = false;
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn encode(&mut self, grid: &Grid, force_full: bool, output: &mut String) -> FrameEncodeStats {
        use ratatui::backend::{Backend, TermionBackend};
        use ratatui::buffer::CellDiffOption;
        let height = grid.len();
        let width = grid.first().map_or(0, Vec::len);
        let area = ratatui::layout::Rect::new(0, 0,
            width.try_into().expect("terminal columns fit u16"),
            height.try_into().expect("terminal rows fit u16"));
        if self.current.area != area {
            self.current.resize(area);
            self.previous.resize(area);
            self.initialized = false;
        }
        let full_repaint = force_full || !self.initialized;
        let option = if full_repaint { CellDiffOption::AlwaysUpdate } else { CellDiffOption::None };
        for (source, target) in grid.iter().flatten().zip(&mut self.current.content) {
            target.set_char(source.ch)
                .set_fg(if source.ch == ' ' { ratatui::style::Color::Reset } else { ratatui_color(source.fg) })
                .set_bg(ratatui_color(source.bg))
                .set_diff_option(option);
        }
        self.bytes.clear();
        let mut changed_cells = 0;
        let mut runs = 0;
        let mut last = None;
        let mut updates = self.previous.diff_iter(&self.current).peekable();
        if updates.peek().is_some() {
            TermionBackend::new(&mut self.bytes).draw(updates.inspect(|(x, y, _)| {
                changed_cells += 1;
                if last != x.checked_sub(1).map(|x| (x, *y)) || last.is_none() {
                    runs += 1;
                }
                last = Some((*x, *y));
            })).expect("writing Ratatui output into Vec cannot fail");
        }
        output.clear();
        output.push_str(std::str::from_utf8(&self.bytes).expect("Ratatui emits UTF-8"));
        std::mem::swap(&mut self.previous, &mut self.current);
        if full_repaint {
            for cell in &mut self.previous.content {
                cell.set_diff_option(CellDiffOption::None);
            }
        }
        self.initialized = true;
        FrameEncodeStats { bytes: output.len(), changed_cells, runs, full_repaint }
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn ratatui_color(color: Color) -> ratatui::style::Color {
    use ratatui::style::Color as R;
    match terminal_color(color) {
        Color::Reset => R::Reset,
        Color::Black => R::Black,
        Color::DarkGrey => R::DarkGray,
        Color::Red => R::LightRed,
        Color::DarkRed => R::Red,
        Color::Green => R::LightGreen,
        Color::DarkGreen => R::Green,
        Color::Yellow => R::LightYellow,
        Color::DarkYellow => R::Yellow,
        Color::Blue => R::LightBlue,
        Color::DarkBlue => R::Blue,
        Color::Magenta => R::LightMagenta,
        Color::DarkMagenta => R::Magenta,
        Color::Cyan => R::LightCyan,
        Color::DarkCyan => R::Cyan,
        Color::White => R::White,
        Color::Grey => R::Gray,
        Color::AnsiValue(value) => R::Indexed(value),
        Color::Rgb {r,g,b} => R::Rgb(r,g,b),
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn terminal_color(color: Color) -> Color {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn cube_index(value: u8) -> u8 {
        match value {
            0..=47 => 0,
            48..=114 => 1,
            115..=154 => 2,
            155..=194 => 3,
            195..=234 => 4,
            235..=255 => 5,
        }
    }
    match color {
        Color::Rgb { r, g, b } => {
            Color::AnsiValue(16 + 36 * cube_index(r) + 6 * cube_index(g) + cube_index(b))
        }
        other => other,
    }
}

#[cfg(test)]
mod ansi_frame_tests {
    use super::*;

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn row(text: &str) -> Grid {
        vec![text.chars().map(|ch| Cell::new(ch, Color::Reset)).collect()]
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn library_diff_emits_only_changed_cells_and_recovers_after_invalidation() {
        let mut encoder = AnsiFrameEncoder::new();
        let mut output = String::new();
        encoder.encode(&row("abcdef"), true, &mut output);
        let stats = encoder.encode(&row("aXcYef"), false, &mut output);
        assert_eq!((stats.changed_cells, stats.runs), (2, 2));
        assert_eq!(output, "\x1b[1;2HX\x1b[1;4HY\x1b[39m\x1b[49m\x1b[m");
        encoder.invalidate();
        let stats = encoder.encode(&row("aXcYef"), false, &mut output);
        assert!(stats.full_repaint);
        assert_eq!(stats.changed_cells, 6);
        assert_eq!(output, "\x1b[1;1HaXcYef\x1b[39m\x1b[49m\x1b[m");
        encoder.encode(&row("aXcYef"), false, &mut output);
        assert_eq!(output, "");
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn library_buffers_cover_more_than_u16_cells_and_resize() {
        let mut encoder = AnsiFrameEncoder::new();
        let mut output = String::new();
        let grid = vec![vec![Cell::blank(); 400]; 200];
        let stats = encoder.encode(&grid, false, &mut output);
        assert_eq!(stats.changed_cells, 80_000);
        assert_eq!(encoder.previous.content.len(), 80_000);
        assert!(output.contains("\x1b[200;1H"));
        let stats = encoder.encode(&row("x"), false, &mut output);
        assert!(stats.full_repaint);
        assert_eq!(stats.changed_cells, 1);
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn replacing_double_width_glyph_repaints_its_reserved_cell() {
        let mut encoder = AnsiFrameEncoder::new();
        let mut output = String::new();
        let wide = vec![vec![
            Cell::new('界', Color::Reset),
            Cell::blank(),
            Cell::new('z', Color::Reset),
        ]];
        let narrow = vec![vec![
            Cell::new('a', Color::Reset),
            Cell::new('b', Color::Reset),
            Cell::new('z', Color::Reset),
        ]];
        encoder.encode(&wide, true, &mut output);
        let stats = encoder.encode(&narrow, false, &mut output);
        assert_eq!(stats.runs, 1);
        assert_eq!(output, "\x1b[1;1Hab\x1b[39m\x1b[49m\x1b[m");
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn foreground_only_changes_on_spaces_emit_no_bytes() {
        let red = Color::Rgb { r: 255, g: 0, b: 0 };
        let blue = Color::Rgb { r: 0, g: 0, b: 255 };
        let mut encoder = AnsiFrameEncoder::new();
        let mut output = String::new();
        encoder.encode(&vec![vec![Cell::new(' ', red)]], true, &mut output);
        let stats = encoder.encode(&vec![vec![Cell::new(' ', blue)]], false, &mut output);
        assert_eq!(stats.changed_cells, 0);
        assert_eq!(stats.bytes, 0);
        assert_eq!(output, "");
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn spaces_do_not_emit_invisible_foreground_sequences() {
        let red = Color::Rgb { r: 255, g: 0, b: 0 };
        let blue = Color::Rgb { r: 0, g: 0, b: 255 };
        let grid = vec![vec![Cell::new(' ', red), Cell::new('x', blue)]];
        let output = grid_to_ansi(&grid);
        assert!(!output.contains("38;2;255;0;0"));
        assert!(output.contains("38;2;0;0;255"));
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn animation_encoder_collapses_adjacent_rgb_levels() {
        let mut encoder = AnsiFrameEncoder::new();
        let mut output = String::new();
        let first = vec![vec![Cell::new(
            'x',
            Color::Rgb {
                r: 101,
                g: 149,
                b: 201,
            },
        )]];
        let adjacent = vec![vec![Cell::new(
            'x',
            Color::Rgb {
                r: 102,
                g: 150,
                b: 202,
            },
        )]];
        encoder.encode(&first, true, &mut output);
        assert!(output.contains("38;5;68m"));
        let stats = encoder.encode(&adjacent, false, &mut output);
        assert_eq!(stats.changed_cells, 0);
        assert_eq!(output, "");
    }
}
