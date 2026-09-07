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
pub(crate) fn write_sgr(s: &mut String, c: Color, fg: bool) {
    crate::render::push_color(s, c, fg);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirtyRun {
    row: usize,
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FrameEncodeStats {
    pub(crate) bytes: usize,
    pub(crate) changed_cells: usize,
    pub(crate) runs: usize,
    pub(crate) full_repaint: bool,
}

/// Retained terminal state for fixed-grid animation.
///
/// Lifetime: one instance belongs to one interactive playback session. A size
/// change invalidates its prior frame. Pause/resume callers may explicitly call
/// `invalidate` when terminal contents could have changed while suspended.
///
/// Storage: `previous` is a contiguous copy of the last final composed frame;
/// `dirty` and `runs` are reused scratch; caller-owned `output` retains its ANSI
/// allocation. Each encode reads the current grid, writes only `output` and the
/// retained comparison state, then makes the current cells uniquely previous.
pub(crate) struct AnsiFrameEncoder {
    width: usize,
    height: usize,
    previous: Vec<Cell>,
    dirty: Vec<bool>,
    runs: Vec<DirtyRun>,
    initialized: bool,
    full_cost_hint: usize,
}

impl AnsiFrameEncoder {
    pub(crate) fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            previous: Vec::new(),
            dirty: Vec::new(),
            runs: Vec::new(),
            initialized: false,
            full_cost_hint: 0,
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.initialized = false;
    }

    /// Encode one final composed grid into `output`.
    ///
    /// Pseudocode:
    /// 1. Rebuild retained storage on a size change and force a full repaint.
    /// 2. Compare final cells, expanding changes around double-width glyphs.
    /// 3. Form row-local dirty runs and absorb gaps whose bytes cost no more
    ///    than another absolute cursor-position escape.
    /// 4. Encode dirty runs, falling back to a full frame when it is cheaper.
    /// 5. Copy current cells into contiguous previous-frame storage.
    pub(crate) fn encode(
        &mut self,
        grid: &Grid,
        force_full: bool,
        output: &mut String,
    ) -> FrameEncodeStats {
        output.clear();
        let height = grid.len();
        let width = grid.first().map_or(0, Vec::len);
        debug_assert!(grid.iter().all(|row| row.len() == width));
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.previous.resize(width * height, Cell::blank());
            self.dirty.resize(width * height, false);
            self.initialized = false;
            self.full_cost_hint = 0;
        }

        let full = force_full || !self.initialized;
        let mut changed_cells = width * height;
        self.runs.clear();

        let mut full_repaint = full;
        if full {
            encode_full_grid(grid, output);
            self.full_cost_hint = output.len();
        } else {
            changed_cells = self.collect_dirty_runs(grid);
            if width * height >= 65_536 && changed_cells > width * height / 2 {
                // Rerolling most cells needs a full repaint. Avoid constructing
                // dirty runs and speculatively encoding the same frame twice.
                encode_full_grid(grid, output);
                self.full_cost_hint = output.len();
                full_repaint = true;
            } else if changed_cells > 0 {
                encode_runs(grid, &self.runs, output);
                // Most animation diffs are far below a full frame. Use the last
                // exact full cost as a cheap gate, then scan the current frame
                // only when the result is close enough to change the decision.
                if self.full_cost_hint == 0
                    || output.len().saturating_mul(4) >= self.full_cost_hint.saturating_mul(3)
                {
                    let full_cost = full_grid_encoded_cost(grid);
                    if output.len() >= full_cost {
                        output.clear();
                        encode_full_grid(grid, output);
                        self.full_cost_hint = output.len();
                        full_repaint = true;
                    }
                }
            }
        }

        for (row_index, row) in grid.iter().enumerate() {
            let start = row_index * width;
            self.previous[start..start + width].copy_from_slice(row);
        }
        self.initialized = true;

        FrameEncodeStats {
            bytes: output.len(),
            changed_cells,
            runs: if full_repaint {
                height
            } else {
                self.runs.len()
            },
            full_repaint,
        }
    }

    fn collect_dirty_runs(&mut self, grid: &Grid) -> usize {
        self.dirty.fill(false);
        let mut changed = 0;
        for (y, row) in grid.iter().enumerate() {
            let offset = y * self.width;
            for (x, cell) in row.iter().enumerate() {
                if !cells_look_equal(*cell, self.previous[offset + x]) {
                    self.dirty[offset + x] = true;
                    changed += 1;
                }
            }
        }

        if self.width * self.height >= 65_536 && changed > self.width * self.height / 2 {
            return changed;
        }

        // A terminal-wide glyph and its reserved following cell form one visual
        // unit. Repaint both sides when either the old or new unit changes.
        for y in 0..self.height {
            let offset = y * self.width;
            for x in 0..self.width {
                if !self.dirty[offset + x] {
                    continue;
                }
                if x > 0
                    && (char_width(grid[y][x - 1].ch) == 2
                        || char_width(self.previous[offset + x - 1].ch) == 2)
                {
                    self.dirty[offset + x - 1] = true;
                }
                if (char_width(grid[y][x].ch) == 2 || char_width(self.previous[offset + x].ch) == 2)
                    && x + 1 < self.width
                {
                    self.dirty[offset + x + 1] = true;
                }
            }
        }

        for (y, row) in grid.iter().enumerate() {
            let offset = y * self.width;
            let mut x = 0;
            while x < self.width {
                while x < self.width && !self.dirty[offset + x] {
                    x += 1;
                }
                if x == self.width {
                    break;
                }
                let start = x;
                while x < self.width && self.dirty[offset + x] {
                    x += 1;
                }
                let mut end = x;

                let (mut fg, mut bg) =
                    colors_after_span(row, start, end, Color::Reset, Color::Reset);
                // Compare the exact bytes for unchanged cells in the gap with
                // the absolute cursor escape that would skip them.
                loop {
                    let mut next = end;
                    while next < self.width && !self.dirty[offset + next] {
                        next += 1;
                    }
                    if next == self.width {
                        break;
                    }
                    let mut next_end = next;
                    while next_end < self.width && self.dirty[offset + next_end] {
                        next_end += 1;
                    }
                    let gap_cost = encoded_span_cost(row, end, next, fg, bg);
                    if gap_cost > cursor_escape_len(y + 1, next + 1) {
                        break;
                    }
                    (fg, bg) = colors_after_span(row, end, next_end, fg, bg);
                    end = next_end;
                    x = next_end;
                }

                self.runs.push(DirtyRun { row: y, start, end });
            }
        }
        changed
    }
}

fn decimal_len(mut value: usize) -> usize {
    let mut len = 1;
    while value >= 10 {
        value /= 10;
        len += 1;
    }
    len
}

fn cursor_escape_len(row: usize, col: usize) -> usize {
    4 + decimal_len(row) + decimal_len(col)
}

fn sgr_len(color: Color, fg: bool) -> usize {
    match color {
        Color::Rgb { r, g, b } => {
            // ESC [ 38|48 ; 2 ; r ; g ; b m
            10 + decimal_len(r as usize) + decimal_len(g as usize) + decimal_len(b as usize)
        }
        Color::Reset => 5,
        other => {
            use crossterm::style::{SetBackgroundColor, SetForegroundColor};
            if fg {
                SetForegroundColor(other).to_string().len()
            } else {
                SetBackgroundColor(other).to_string().len()
            }
        }
    }
}

fn terminal_color(color: Color) -> Color {
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

fn encoded_span_cost(
    row: &[Cell],
    start: usize,
    end: usize,
    mut cur_fg: Color,
    mut cur_bg: Color,
) -> usize {
    let mut cost = 0;
    let mut x = start;
    while x < end {
        let cell = row[x];
        let fg = terminal_color(cell.fg);
        let bg = terminal_color(cell.bg);
        if cell.ch != ' ' && fg != cur_fg {
            cost += sgr_len(fg, true);
            cur_fg = fg;
        }
        if bg != cur_bg {
            cost += sgr_len(bg, false);
            cur_bg = bg;
        }
        cost += cell.ch.len_utf8();
        x += if char_width(cell.ch) == 2 { 2 } else { 1 };
    }
    cost
}

fn colors_after_span(
    row: &[Cell],
    start: usize,
    end: usize,
    mut cur_fg: Color,
    mut cur_bg: Color,
) -> (Color, Color) {
    let mut x = start;
    while x < end {
        let cell = row[x];
        if cell.ch != ' ' {
            cur_fg = terminal_color(cell.fg);
        }
        cur_bg = terminal_color(cell.bg);
        x += if char_width(cell.ch) == 2 { 2 } else { 1 };
    }
    (cur_fg, cur_bg)
}

fn full_grid_encoded_cost(grid: &Grid) -> usize {
    let mut cost = 4; // final SGR reset
    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;
    for (y, row) in grid.iter().enumerate() {
        cost += cursor_escape_len(y + 1, 1);
        cost += encoded_span_cost(row, 0, row.len(), cur_fg, cur_bg);
        (cur_fg, cur_bg) = colors_after_span(row, 0, row.len(), cur_fg, cur_bg);
        if cur_bg != Color::Reset {
            cost += sgr_len(Color::Reset, false);
            cur_bg = Color::Reset;
        }
    }
    cost
}

fn encode_span(
    output: &mut String,
    row: &[Cell],
    start: usize,
    end: usize,
    cur_fg: &mut Color,
    cur_bg: &mut Color,
) {
    let mut x = start;
    while x < end {
        let cell = row[x];
        let fg = terminal_color(cell.fg);
        let bg = terminal_color(cell.bg);
        // A space has no foreground pixels. Preserve the last emitted
        // foreground until a glyph makes a foreground transition visible.
        if cell.ch != ' ' && fg != *cur_fg {
            write_sgr(output, fg, true);
            *cur_fg = fg;
        }
        if bg != *cur_bg {
            write_sgr(output, bg, false);
            *cur_bg = bg;
        }
        output.push(cell.ch);
        x += if char_width(cell.ch) == 2 { 2 } else { 1 };
    }
}

fn cells_look_equal(current: Cell, previous: Cell) -> bool {
    current.ch == previous.ch
        && terminal_color(current.bg) == terminal_color(previous.bg)
        && (current.ch == ' ' || terminal_color(current.fg) == terminal_color(previous.fg))
}

fn encode_full_grid(grid: &Grid, output: &mut String) {
    use std::fmt::Write as _;
    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;
    for (y, row) in grid.iter().enumerate() {
        let _ = write!(output, "\x1b[{};1H", y + 1);
        encode_span(output, row, 0, row.len(), &mut cur_fg, &mut cur_bg);
        if cur_bg != Color::Reset {
            write_sgr(output, Color::Reset, false);
            cur_bg = Color::Reset;
        }
    }
    output.push_str("\x1b[0m");
}

fn encode_runs(grid: &Grid, runs: &[DirtyRun], output: &mut String) {
    use std::fmt::Write as _;
    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;
    let mut cursor: Option<(usize, usize)> = None;
    for run in runs {
        let row = run.row + 1;
        let col = run.start + 1;
        match cursor {
            Some((cursor_row, cursor_col)) if cursor_row == row && cursor_col == col => {}
            Some((cursor_row, cursor_col)) if cursor_row == row => {
                let delta = cursor_col.abs_diff(col);
                let relative_len = if delta == 1 {
                    3
                } else {
                    3 + decimal_len(delta)
                };
                let column_len = if col == 1 { 3 } else { 3 + decimal_len(col) };
                if relative_len < column_len {
                    let command = if col > cursor_col { 'C' } else { 'D' };
                    if delta == 1 {
                        let _ = write!(output, "\x1b[{command}");
                    } else {
                        let _ = write!(output, "\x1b[{delta}{command}");
                    }
                } else if col == 1 {
                    output.push_str("\x1b[G");
                } else {
                    let _ = write!(output, "\x1b[{col}G");
                }
            }
            _ => {
                let _ = write!(output, "\x1b[{row};{col}H");
            }
        }
        encode_span(
            output,
            &grid[run.row],
            run.start,
            run.end,
            &mut cur_fg,
            &mut cur_bg,
        );
        cursor = Some((row, run.end + 1));
    }
    output.push_str("\x1b[0m");
}

#[cfg(test)]
mod ansi_frame_tests {
    use super::*;

    fn row(text: &str) -> Grid {
        vec![text.chars().map(|ch| Cell::new(ch, Color::Reset)).collect()]
    }

    #[test]
    fn dirty_runs_coalesce_when_gap_bytes_cost_less_than_cursor_move() {
        let mut encoder = AnsiFrameEncoder::new();
        let mut output = String::new();
        encoder.encode(&row("abcdef"), true, &mut output);
        let stats = encoder.encode(&row("aXcYef"), false, &mut output);
        assert_eq!(stats.changed_cells, 2);
        assert_eq!(stats.runs, 1);
        assert_eq!(output, "\x1b[1;2HXcY\x1b[0m");
    }

    #[test]
    fn dirty_runs_stay_separate_when_gap_is_more_expensive_than_cursor_move() {
        let mut encoder = AnsiFrameEncoder::new();
        let mut output = String::new();
        let before = "a".repeat(40);
        encoder.encode(&row(&before), true, &mut output);
        let mut after = before.into_bytes();
        after[1] = b'X';
        after[35] = b'Y';
        let after = String::from_utf8(after).unwrap();
        let stats = encoder.encode(&row(&after), false, &mut output);
        assert_eq!(stats.changed_cells, 2);
        assert_eq!(stats.runs, 2);
        assert_eq!(output, "\x1b[1;2HX\x1b[36GY\x1b[0m");
    }

    #[test]
    fn retained_cursor_reduces_sparse_run_control_bytes() {
        use std::fmt::Write as _;

        let grid = vec![vec![Cell::blank(); 1_000]];
        let runs = (0..100)
            .map(|index| DirtyRun {
                row: 0,
                start: index * 10,
                end: index * 10 + 1,
            })
            .collect::<Vec<_>>();
        let mut retained = String::new();
        encode_runs(&grid, &runs, &mut retained);

        let mut absolute = String::new();
        let mut cur_fg = Color::Reset;
        let mut cur_bg = Color::Reset;
        for run in &runs {
            let _ = write!(absolute, "\x1b[{};{}H", run.row + 1, run.start + 1);
            encode_span(
                &mut absolute,
                &grid[run.row],
                run.start,
                run.end,
                &mut cur_fg,
                &mut cur_bg,
            );
        }
        absolute.push_str("\x1b[0m");

        assert_eq!((absolute.len(), retained.len()), (893, 506));
    }

    #[test]
    fn dense_change_uses_full_frame_fallback() {
        let mut encoder = AnsiFrameEncoder::new();
        let mut output = String::new();
        encoder.encode(&row("aaaaaaaaaaaaaaaa"), true, &mut output);
        let stats = encoder.encode(&row("bbbbbbbbbbbbbbbb"), false, &mut output);
        assert!(stats.full_repaint);
        assert_eq!(output, "\x1b[1;1Hbbbbbbbbbbbbbbbb\x1b[0m");
    }

    #[test]
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
        assert_eq!(output, "\x1b[1;1Hab\x1b[0m");
    }

    #[test]
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
    fn spaces_do_not_emit_invisible_foreground_sequences() {
        let red = Color::Rgb { r: 255, g: 0, b: 0 };
        let blue = Color::Rgb { r: 0, g: 0, b: 255 };
        let grid = vec![vec![Cell::new(' ', red), Cell::new('x', blue)]];
        let output = grid_to_ansi(&grid);
        assert!(!output.contains("38;2;255;0;0"));
        assert!(output.contains("38;2;0;0;255"));
    }

    #[test]
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
