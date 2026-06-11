use crate::content::*;
use crate::sprites::draw_fret_border;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;

pub enum BorderStyle {
    Light,
    Heavy,
    Double,
    Rounded,
    Fret,
}

pub fn border_glyphs(style: &BorderStyle) -> (char, char, char, char, char, char) {
    match style {
        BorderStyle::Light => ('┌', '┐', '└', '┘', '─', '│'),
        BorderStyle::Heavy => ('┏', '┓', '┗', '┛', '━', '┃'),
        BorderStyle::Double => ('╔', '╗', '╚', '╝', '═', '║'),
        BorderStyle::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
        BorderStyle::Fret => unreachable!(),
    }
}

/// Draw a box border around rect, preserving existing bg colors.
/// No-op if rect < 3x3. Fret falls back to Light if rect < 8x6.
pub fn draw_box_border(grid: &mut Grid, rect: &Rect, style: &BorderStyle, color: Color) {
    if rect.w < 3 || rect.h < 3 {
        return;
    }

    if matches!(style, BorderStyle::Fret) {
        if rect.w < 8 || rect.h < 6 {
            return draw_box_border(grid, rect, &BorderStyle::Light, color);
        }
        let band = 2;
        for edge in 0..4 {
            draw_fret_border(grid, rect.x, rect.y, rect.w, rect.h, band, edge, color);
        }
        return;
    }

    let (tl, tr, bl, br, horiz, vert) = border_glyphs(style);
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.w - 1;
    let y1 = rect.y + rect.h - 1;

    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() {
            let bg = grid[y][x].bg;
            grid[y][x] = Cell::with_bg(ch, fg, bg);
        }
    };

    set(grid, x0, y0, tl, color);
    set(grid, x1, y0, tr, color);
    set(grid, x0, y1, bl, color);
    set(grid, x1, y1, br, color);

    for x in (x0 + 1)..x1 {
        set(grid, x, y0, horiz, color);
        set(grid, x, y1, horiz, color);
    }
    for y in (y0 + 1)..y1 {
        set(grid, x0, y, vert, color);
        set(grid, x1, y, vert, color);
    }
}

/// Draw decorative corner embellishments on a bordered rect.
/// Overlays triangle/block motifs at each corner, preserving bg.
pub fn draw_corner_embellishments(grid: &mut Grid, rect: &Rect, style: usize, color: Color) {
    if rect.w < 5 || rect.h < 4 {
        return;
    }

    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.w - 1;
    let y1 = rect.y + rect.h - 1;

    let set = |grid: &mut Grid, x: usize, y: usize, ch: char| {
        if y < grid.len() && x < grid[0].len() {
            let bg = grid[y][x].bg;
            grid[y][x] = Cell::with_bg(ch, color, bg);
        }
    };

    match style % 6 {
        0 => {
            // Fan: quarter-triangle + half-block extensions
            set(grid, x0, y0, '◤');
            set(grid, x0 + 1, y0, '▀');
            set(grid, x0, y0 + 1, '▌');
            set(grid, x1, y0, '◥');
            set(grid, x1 - 1, y0, '▀');
            set(grid, x1, y0 + 1, '▐');
            set(grid, x0, y1, '◣');
            set(grid, x0 + 1, y1, '▄');
            set(grid, x0, y1 - 1, '▌');
            set(grid, x1, y1, '◢');
            set(grid, x1 - 1, y1, '▄');
            set(grid, x1, y1 - 1, '▐');
        }
        1 => {
            // Double triangle: two quarter-triangles per corner
            set(grid, x0, y0, '◤');
            set(grid, x0 + 1, y0, '◥');
            set(grid, x0, y0 + 1, '◣');
            set(grid, x1, y0, '◥');
            set(grid, x1 - 1, y0, '◤');
            set(grid, x1, y0 + 1, '◢');
            set(grid, x0, y1, '◣');
            set(grid, x0 + 1, y1, '◢');
            set(grid, x0, y1 - 1, '◤');
            set(grid, x1, y1, '◢');
            set(grid, x1 - 1, y1, '◣');
            set(grid, x1, y1 - 1, '◥');
        }
        2 => {
            // Block: three-quadrant block chars
            set(grid, x0, y0, '▛');
            set(grid, x0 + 1, y0, '▀');
            set(grid, x0, y0 + 1, '▌');
            set(grid, x1, y0, '▜');
            set(grid, x1 - 1, y0, '▀');
            set(grid, x1, y0 + 1, '▐');
            set(grid, x0, y1, '▙');
            set(grid, x0 + 1, y1, '▄');
            set(grid, x0, y1 - 1, '▌');
            set(grid, x1, y1, '▟');
            set(grid, x1 - 1, y1, '▄');
            set(grid, x1, y1 - 1, '▐');
        }
        3 => {
            // Arrow: triangle + directional pointer
            set(grid, x0, y0, '◤');
            set(grid, x0 + 1, y0, '▶');
            set(grid, x0, y0 + 1, '▼');
            set(grid, x1, y0, '◥');
            set(grid, x1 - 1, y0, '◀');
            set(grid, x1, y0 + 1, '▼');
            set(grid, x0, y1, '◣');
            set(grid, x0 + 1, y1, '▶');
            set(grid, x0, y1 - 1, '▲');
            set(grid, x1, y1, '◢');
            set(grid, x1 - 1, y1, '◀');
            set(grid, x1, y1 - 1, '▲');
        }
        4 if rect.w >= 7 && rect.h >= 5 => {
            // Layered: 3-deep triangle cascade
            set(grid, x0, y0, '◤');
            set(grid, x0 + 1, y0, '◥');
            set(grid, x0 + 2, y0, '▀');
            set(grid, x0, y0 + 1, '◣');
            set(grid, x0 + 1, y0 + 1, '◤');
            set(grid, x0, y0 + 2, '▌');

            set(grid, x1, y0, '◥');
            set(grid, x1 - 1, y0, '◤');
            set(grid, x1 - 2, y0, '▀');
            set(grid, x1, y0 + 1, '◢');
            set(grid, x1 - 1, y0 + 1, '◥');
            set(grid, x1, y0 + 2, '▐');

            set(grid, x0, y1, '◣');
            set(grid, x0 + 1, y1, '◢');
            set(grid, x0 + 2, y1, '▄');
            set(grid, x0, y1 - 1, '◤');
            set(grid, x0 + 1, y1 - 1, '◣');
            set(grid, x0, y1 - 2, '▌');

            set(grid, x1, y1, '◢');
            set(grid, x1 - 1, y1, '◣');
            set(grid, x1 - 2, y1, '▄');
            set(grid, x1, y1 - 1, '◥');
            set(grid, x1 - 1, y1 - 1, '◢');
            set(grid, x1, y1 - 2, '▐');
        }
        _ => {
            // Bracket: half-bracket corners with side accents
            set(grid, x0, y0, '⌜');
            set(grid, x0 + 1, y0, '▀');
            set(grid, x0, y0 + 1, '▏');
            set(grid, x1, y0, '⌝');
            set(grid, x1 - 1, y0, '▀');
            set(grid, x1, y0 + 1, '▕');
            set(grid, x0, y1, '⌞');
            set(grid, x0 + 1, y1, '▄');
            set(grid, x0, y1 - 1, '▏');
            set(grid, x1, y1, '⌟');
            set(grid, x1 - 1, y1, '▄');
            set(grid, x1, y1 - 1, '▕');
        }
    }
}

pub fn pick_border_style(rng: &mut StdRng, w: usize, h: usize) -> BorderStyle {
    let area = w * h;
    if area < 100 {
        if rng.random_range(0..2) == 0 {
            BorderStyle::Light
        } else {
            BorderStyle::Rounded
        }
    } else {
        match rng.random_range(0..5) {
            0 => BorderStyle::Light,
            1 => BorderStyle::Heavy,
            2 => BorderStyle::Double,
            3 => BorderStyle::Rounded,
            _ => {
                if w >= 8 && h >= 6 {
                    BorderStyle::Fret
                } else {
                    BorderStyle::Light
                }
            }
        }
    }
}

pub fn border_inset(style: &BorderStyle) -> usize {
    match style {
        BorderStyle::Fret => 3,
        _ => 1,
    }
}

/// Wrapper: draw a decorative border, then render content in the inset area.
/// If `clear` is true, clears the rect before drawing (use for truchet bg).
/// If false, preserves existing bg (use for mondrian color fills).
pub fn render_block_with_border(
    grid: &mut Grid,
    block: &ContentBlock,
    rect: &Rect,
    fg: Color,
    bar_fg: Color,
    clear: bool,
    rng: &mut StdRng,
) {
    if clear {
        for y in rect.y..rect.y + rect.h {
            for x in rect.x..rect.x + rect.w {
                if y < grid.len() && x < grid[0].len() {
                    grid[y][x] = Cell::blank();
                }
            }
        }
    }

    let style = pick_border_style(rng, rect.w, rect.h);
    let inset = border_inset(&style);

    // fall back to borderless if rect too small for border + content
    if rect.w <= inset * 2 + 4 || rect.h <= inset * 2 + 2 {
        render_block_preserve_bg(grid, block, rect, fg, bar_fg);
        return;
    }

    draw_box_border(grid, rect, &style, fg);

    // corner embellishments on non-fret, non-rounded borders (50% chance, needs space)
    if !matches!(style, BorderStyle::Fret | BorderStyle::Rounded) && rect.w >= 7 && rect.h >= 5 {
        if rng.random_range(0..2) == 0 {
            let corner_style = rng.random_range(0..6);
            draw_corner_embellishments(grid, rect, corner_style, fg);
        }
    }

    let inner = Rect {
        x: rect.x + inset,
        y: rect.y + inset,
        w: rect.w - inset * 2,
        h: rect.h - inset * 2,
    };
    render_block_preserve_bg(grid, block, &inner, fg, bar_fg);
}
