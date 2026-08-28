//! Styled path drawing.
#![allow(unused)]
use crate::color::*;
use crate::fills::*;
use crate::scene::*;
use crate::sprites::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;
use super::*;

/// Visual style of the path connecting nodes.
#[derive(Clone, Copy)]
pub enum PathStyle {
    /// Simple box-drawing line (the default from draw_walk_path)
    Line,
    /// Dotted trail with directional hints
    Dots,
    /// Vine/branch with organic chars
    Vine,
    /// River/water flow
    River,
    /// Double-line border path
    DoubleLine,
}
impl PathStyle {
    pub fn pick(rng: &mut StdRng) -> Self {
        match rng.random_range(0..5u32) {
            0 => PathStyle::Line,
            1 => PathStyle::Dots,
            2 => PathStyle::Vine,
            3 => PathStyle::River,
            _ => PathStyle::DoubleLine,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "line" => PathStyle::Line,
            "dots" => PathStyle::Dots,
            "vine" => PathStyle::Vine,
            "river" => PathStyle::River,
            "double" => PathStyle::DoubleLine,
            _ => return Option::None,
        })
    }
}
/// Draw a styled path between waypoints on the grid.
pub fn draw_styled_path(
    grid: &mut Grid,
    stops: &[(usize, usize)],
    style: PathStyle,
    color: Color,
    rng: &mut StdRng,
) {
    match style {
        PathStyle::Line => draw_walk_path(grid, stops, color),
        PathStyle::Dots => draw_path_trail(grid, stops, color, rng),
        PathStyle::Vine => draw_vine_path(grid, stops, color, rng),
        PathStyle::River => draw_river_path(grid, stops, color, rng),
        PathStyle::DoubleLine => draw_double_path(grid, stops, color),
    }
}
/// Vine path: organic line with occasional leaf/bud chars branching off.
pub(crate) fn draw_vine_path(grid: &mut Grid, stops: &[(usize, usize)], color: Color, rng: &mut StdRng) {
    let h = grid.len();
    if h == 0 {
        return;
    }
    let w = grid[0].len();
    let vine_chars = ['╱', '╲', '│', '─', '╰', '╮', '╭', '╯'];
    let leaf_chars = ['◠', '◡', '·', '∙', '°'];

    for pair in stops.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        let dx = x1 as f32 - x0 as f32;
        let dy = y1 as f32 - y0 as f32;
        let steps = (dx.abs().max(dy.abs())) as usize;
        if steps == 0 {
            continue;
        }

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            // Slight sinuous wobble
            let wobble = (t * 8.0 * std::f32::consts::PI).sin() * 1.2;
            let perp_x = -dy / (dx.abs() + dy.abs()).max(1.0);
            let perp_y = dx / (dx.abs() + dy.abs()).max(1.0);
            let x = (x0 as f32 + dx * t + perp_x * wobble) as usize;
            let y = (y0 as f32 + dy * t + perp_y * wobble) as usize;
            if x >= w || y >= h {
                continue;
            }

            let ch = if dx.abs() < 1.0 {
                '│'
            } else if dy.abs() / dx.abs() < 0.3 {
                '─'
            } else if (dx > 0.0) == (dy > 0.0) {
                '╲'
            } else {
                '╱'
            };
            grid[y][x] = Cell::new(ch, color);

            // Branch off leaf/bud every 5-8 steps
            if i % rng.random_range(5..9) == 0 {
                let lx = (x as i32 + rng.random_range(-2..3i32)).clamp(0, w as i32 - 1) as usize;
                let ly = (y as i32 + rng.random_range(-1..2i32)).clamp(0, h as i32 - 1) as usize;
                if grid[ly][lx].ch == ' ' {
                    let lch = leaf_chars[rng.random_range(0..leaf_chars.len())];
                    grid[ly][lx] = Cell::new(lch, lighten(color, 30));
                }
            }
        }
    }
}
/// River path: wider line using water-like chars.
pub(crate) fn draw_river_path(grid: &mut Grid, stops: &[(usize, usize)], color: Color, rng: &mut StdRng) {
    let h = grid.len();
    if h == 0 {
        return;
    }
    let w = grid[0].len();
    let water_chars = ['~', '≈', '∿', '─', '╌'];

    for pair in stops.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        let dx = x1 as f32 - x0 as f32;
        let dy = y1 as f32 - y0 as f32;
        let steps = (dx.abs().max(dy.abs())) as usize;
        if steps == 0 {
            continue;
        }
        let perp_x = -dy / (dx.abs() + dy.abs()).max(1.0);
        let perp_y = dx / (dx.abs() + dy.abs()).max(1.0);

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let cx = x0 as f32 + dx * t;
            let cy = y0 as f32 + dy * t;

            // Width of 3 cells perpendicular to flow
            for offset in -1..=1i32 {
                let px = (cx + perp_x * offset as f32).round() as usize;
                let py = (cy + perp_y * offset as f32).round() as usize;
                if px >= w || py >= h {
                    continue;
                }
                let ch = water_chars[rng.random_range(0..water_chars.len())];
                let c = if offset == 0 {
                    color
                } else {
                    darken(color, 30)
                };
                grid[py][px] = Cell::new(ch, c);
            }
        }
    }
}
/// Double-line path: uses double box-drawing chars for a bolder connection.
pub(crate) fn draw_double_path(grid: &mut Grid, stops: &[(usize, usize)], color: Color) {
    let h = grid.len();
    if h == 0 {
        return;
    }
    let w = grid[0].len();

    for pair in stops.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        let dx = x1 as f32 - x0 as f32;
        let dy = y1 as f32 - y0 as f32;
        let steps = (dx.abs().max(dy.abs())) as usize;
        if steps == 0 {
            continue;
        }

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = (x0 as f32 + dx * t) as usize;
            let y = (y0 as f32 + dy * t) as usize;
            if x >= w || y >= h {
                continue;
            }

            let ch = if dx.abs() < 1.0 {
                '║'
            } else if dy.abs() / dx.abs() < 0.3 {
                '═'
            } else {
                '║'
            }; // double lines only have H/V
            grid[y][x] = Cell::new(ch, color);
        }
    }
}
