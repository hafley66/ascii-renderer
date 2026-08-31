use crossterm::style::Color;
use rand::rngs::StdRng;

use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::pp::{pp_fbm, pp_hash2, pp_stroke};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

const TAU: f32 = std::f32::consts::TAU;

pub(super) struct HyperloomMode;

pub(super) static MODE: HyperloomMode = HyperloomMode;

const PARAMS: &[Param] = &[
    param!("THREADS", "warp threads", 4.0, 36.0, 20.0, 1.0),
    param!("LOOMS", "loom apertures", 1.0, 7.0, 4.0, 1.0),
    param!("SYMM", "knot symmetry", 3.0, 18.0, 9.0, 1.0),
    param!("DEPTH", "nested cages", 1.0, 8.0, 5.0, 1.0),
    param!("SHUTTLE", "flying shuttles", 0.0, 18.0, 7.0, 1.0),
    param!("KNOTS", "crossing jewels", 0.0, 72.0, 28.0, 2.0),
    param!("SPEED", "loom velocity", 0.05, 3.0, 0.72, 0.05),
    param!("WARP", "field curvature", 0.0, 1.5, 0.72, 0.05),
    param!("WEAVE", "harmonic weave", 1.0, 9.0, 4.0, 0.5),
    param!("MOIRE", "interference field", 0.0, 1.5, 0.82, 0.05),
    param!("TRAIL", "shuttle memory", 1.0, 24.0, 12.0, 1.0),
    param!("BLOOM", "spectral bloom", 0.0, 1.5, 0.86, 0.05),
];

impl Mode for HyperloomMode {
    fn name(&self) -> &'static str {
        "hyperloom"
    }

    fn help(&self) -> &'static str {
        "Kinetic Jacquard engine: braided warp/weft fields, nested loom apertures, Lissajous knot cages, shuttle comets [threads] [looms] [symm] [depth] [shuttles] [knots] [speed] [warp] [weave] [moire] [trail] [bloom]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = HyperloomParams::from_inputs(frame.args, frame.param_values);
        draw_hyperloom(
            frame.grid,
            frame.width,
            frame.height,
            frame.seed,
            frame.palette,
            frame.rng,
            frame.time,
            &params,
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HyperloomParams {
    pub(crate) threads: usize,
    pub(crate) looms: usize,
    pub(crate) symmetry: usize,
    pub(crate) depth: usize,
    pub(crate) shuttles: usize,
    pub(crate) knots: usize,
    pub(crate) speed: f32,
    pub(crate) warp: f32,
    pub(crate) weave: f32,
    pub(crate) moire: f32,
    pub(crate) trail: usize,
    pub(crate) bloom: f32,
}

impl Default for HyperloomParams {
    fn default() -> Self {
        Self {
            threads: 20,
            looms: 4,
            symmetry: 9,
            depth: 5,
            shuttles: 7,
            knots: 28,
            speed: 0.72,
            warp: 0.72,
            weave: 4.0,
            moire: 0.82,
            trail: 12,
            bloom: 0.86,
        }
    }
}

impl HyperloomParams {
    pub(crate) fn from_inputs(args: &[String], param_values: Option<&[f32]>) -> Self {
        let read = |index: usize, key: &str, default: f32| {
            args.get(index)
                .and_then(|value| value.parse::<f32>().ok())
                .or_else(|| {
                    param_values
                        .and_then(|values| values.get(index - 4))
                        .copied()
                })
                .unwrap_or_else(|| param_f32(key, default))
        };
        Self {
            threads: read(4, "THREADS", 20.0).round().clamp(4.0, 36.0) as usize,
            looms: read(5, "LOOMS", 4.0).round().clamp(1.0, 7.0) as usize,
            symmetry: read(6, "SYMM", 9.0).round().clamp(3.0, 18.0) as usize,
            depth: read(7, "DEPTH", 5.0).round().clamp(1.0, 8.0) as usize,
            shuttles: read(8, "SHUTTLE", 7.0).round().clamp(0.0, 18.0) as usize,
            knots: read(9, "KNOTS", 28.0).round().clamp(0.0, 72.0) as usize,
            speed: read(10, "SPEED", 0.72).clamp(0.05, 3.0),
            warp: read(11, "WARP", 0.72).clamp(0.0, 1.5),
            weave: read(12, "WEAVE", 4.0).clamp(1.0, 9.0),
            moire: read(13, "MOIRE", 0.82).clamp(0.0, 1.5),
            trail: read(14, "TRAIL", 12.0).round().clamp(1.0, 24.0) as usize,
            bloom: read(15, "BLOOM", 0.86).clamp(0.0, 1.5),
        }
    }
}

fn hash01(seed: u64, tag: u64) -> f32 {
    let mut value = seed ^ tag.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    ((value >> 40) as f32) / ((1u64 << 24) - 1) as f32
}

fn in_bounds(grid: &Grid, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[y as usize].len()
}

fn put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if in_bounds(grid, x, y) {
        let bg = grid[y as usize][x as usize].bg;
        grid[y as usize][x as usize] = Cell::with_bg(ch, fg, bg);
    }
}

fn line(grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color) {
    let ch = pp_stroke(x1 - x0, y1 - y0);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        put(grid, x0, y0, ch, fg);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice = error * 2;
        if twice >= dy {
            error += dy;
            x0 += sx;
        }
        if twice <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn stroke_curve(
    grid: &mut Grid,
    samples: usize,
    color: Color,
    mut point: impl FnMut(f32) -> (f32, f32),
) {
    let mut previous = None;
    for sample in 0..=samples.max(1) {
        let u = sample as f32 / samples.max(1) as f32;
        let (x, y) = point(u);
        let current = (x.round() as i32, y.round() as i32);
        if let Some((px, py)) = previous {
            if current != (px, py) {
                line(grid, px, py, current.0, current.1, color);
            }
        }
        previous = Some(current);
    }
}

fn polygon(grid: &mut Grid, points: &[(f32, f32)], color: Color) {
    if points.len() < 2 {
        return;
    }
    for index in 0..points.len() {
        let from = points[index];
        let to = points[(index + 1) % points.len()];
        line(
            grid,
            from.0.round() as i32,
            from.1.round() as i32,
            to.0.round() as i32,
            to.1.round() as i32,
            color,
        );
    }
}

fn thread_color(palette: &[Color; 5], index: usize, count: usize, t: f32) -> Color {
    let mix = if count <= 1 {
        0.5
    } else {
        index as f32 / (count - 1) as f32
    };
    let base = lerp_color(palette[1], palette[3], mix);
    shift_hue(base, ((t * 13.0 + index as f32 * 17.0).sin() * 24.0) as f64)
}

fn warp_thread_point(
    width: usize,
    height: usize,
    seed: u64,
    index: usize,
    count: usize,
    u: f32,
    t: f32,
    params: &HyperloomParams,
) -> (f32, f32) {
    let phase = hash01(seed, 100 + index as u64) * TAU;
    let base_y = (index as f32 + 0.5) / count as f32 * height as f32;
    let amplitude = (height as f32 / count as f32).max(1.0) * (0.8 + params.warp * 1.6);
    let primary = (u * TAU * params.weave + phase + t * (0.7 + index as f32 * 0.013)).sin();
    let secondary = (u * TAU * (params.weave * 0.5 + 1.0) - phase * 0.7 - t * 1.3).cos();
    let lens = ((u - 0.5) * TAU).sin() * (t * 0.41 + phase).cos();
    (
        u * width.saturating_sub(1) as f32,
        base_y + amplitude * (primary * 0.58 + secondary * 0.25 + lens * 0.17),
    )
}

fn weft_thread_point(
    width: usize,
    height: usize,
    seed: u64,
    index: usize,
    count: usize,
    u: f32,
    t: f32,
    params: &HyperloomParams,
) -> (f32, f32) {
    let phase = hash01(seed, 300 + index as u64) * TAU;
    let base_x = (index as f32 + 0.5) / count as f32 * width as f32;
    let amplitude = (width as f32 / count as f32).max(1.0) * (0.45 + params.warp);
    let primary = (u * TAU * (params.weave * 0.72 + 1.0) + phase - t * 0.83).sin();
    let secondary = (u * TAU * 2.0 - phase + t * 1.17).cos();
    (
        base_x + amplitude * (primary * 0.72 + secondary * 0.28),
        u * height.saturating_sub(1) as f32,
    )
}

fn draw_spectral_field(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &HyperloomParams,
) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    for y in 0..height {
        for x in 0..width {
            let nx = (x as f32 - cx) / width.max(1) as f32;
            let ny = (y as f32 - cy) / height.max(1) as f32;
            let radial = (nx * nx * 3.0 + ny * ny * 7.0).sqrt();
            let wave_a = (nx * 38.0 + ny * 17.0 + t * 0.31).sin();
            let wave_b = (ny * 29.0 - nx * 13.0 - t * 0.23).cos();
            let noise = pp_fbm(x as f32 * 0.035, y as f32 * 0.07 + t * 0.015, seed);
            let interference = (wave_a * wave_b * 0.5 + 0.5) * params.moire;
            let glow = ((1.0 - radial).max(0.0) * 0.35 + interference * 0.2 + noise * 0.15)
                .clamp(0.0, 1.0);
            let bg = lerp_color(darken(palette[0], 18), darken(palette[2], 55), glow);
            let threshold = pp_hash2(x as i32, y as i32, seed ^ 0xA11C_E5ED);
            let ch = if threshold < glow * 0.08 * params.bloom {
                if interference > 0.7 { '∙' } else { '·' }
            } else {
                ' '
            };
            let fg = darken(lerp_color(palette[2], palette[3], interference), 45);
            grid[y][x] = Cell::with_bg(ch, fg, bg);
        }
    }
}

fn loom_center(
    width: usize,
    height: usize,
    seed: u64,
    index: usize,
    count: usize,
    t: f32,
) -> (f32, f32, f32) {
    let phase = hash01(seed, 700 + index as u64) * TAU;
    let radius_x = width as f32 * (0.13 + 0.04 * hash01(seed, 900 + index as u64));
    let radius_y = height as f32 * (0.12 + 0.05 * hash01(seed, 1100 + index as u64));
    let angle = index as f32 / count.max(1) as f32 * TAU + phase * 0.22 + t * 0.08;
    (
        width as f32 * 0.5 + angle.cos() * radius_x,
        height as f32 * 0.5 + angle.sin() * radius_y,
        phase,
    )
}

fn draw_loom_apertures(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &HyperloomParams,
) {
    let base = width.min(height.saturating_mul(2)) as f32;
    for loom in 0..params.looms {
        let (cx, cy, phase) = loom_center(width, height, seed, loom, params.looms, t);
        for depth in (0..params.depth).rev() {
            let depth_t = (depth + 1) as f32 / params.depth as f32;
            let radius = base * (0.035 + depth_t * 0.055);
            let sides = 4 + (loom + depth) % 4;
            let angle = phase + t * params.speed * (0.18 + depth_t * 0.12);
            let mut points = Vec::with_capacity(sides);
            for side in 0..sides {
                let a = side as f32 / sides as f32 * TAU + angle;
                let breathe = 1.0 + (t * 0.9 + phase + side as f32).sin() * 0.08 * params.warp;
                points.push((
                    cx + a.cos() * radius * 1.8 * breathe,
                    cy + a.sin() * radius * 0.72 * breathe,
                ));
            }
            let color = darken(
                lerp_color(palette[2], palette[3], depth_t),
                ((1.0 - depth_t) * 55.0) as u8,
            );
            polygon(grid, &points, color);
        }
        put(
            grid,
            cx.round() as i32,
            cy.round() as i32,
            if loom % 2 == 0 { '◈' } else { '◇' },
            lighten(palette[3], (params.bloom * 35.0) as u8),
        );
    }
}

fn draw_threads(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &HyperloomParams,
) {
    let samples = width.max(2).min(640);
    for index in 0..params.threads {
        let color = thread_color(palette, index, params.threads, t);
        stroke_curve(grid, samples, color, |u| {
            warp_thread_point(width, height, seed, index, params.threads, u, t, params)
        });
    }

    let weft_count = (params.threads * 2 / 3).max(3);
    let samples = height.max(2).min(360);
    for index in 0..weft_count {
        let mut color = thread_color(palette, weft_count - 1 - index, weft_count, -t);
        color = darken(color, 12);
        stroke_curve(grid, samples, color, |u| {
            weft_thread_point(width, height, seed, index, weft_count, u, t, params)
        });
    }
}

fn draw_knot_cages(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &HyperloomParams,
) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let radius = width.min(height.saturating_mul(2)) as f32 * 0.34;
    let samples = (width + height).clamp(96, 900);
    for depth in 0..params.depth {
        let d = (depth + 1) as f32 / params.depth as f32;
        let frequency_a = params.symmetry as f32 + (depth % 3) as f32;
        let frequency_b = params.symmetry.saturating_sub(2).max(2) as f32 + (depth % 2) as f32;
        let phase = hash01(seed, 1700 + depth as u64) * TAU;
        let color = lighten(
            lerp_color(palette[1], palette[3], d),
            (params.bloom * d * 24.0) as u8,
        );
        stroke_curve(grid, samples, color, |u| {
            let a = u * TAU;
            let envelope = 0.28 + d * 0.72;
            let pulse = 1.0 + (t * 0.63 + phase + a * 2.0).sin() * 0.06 * params.warp;
            (
                cx + (a * frequency_a + phase + t * params.speed * 0.21).sin()
                    * radius
                    * envelope
                    * 1.65
                    * pulse,
                cy + (a * frequency_b - phase * 0.7 - t * params.speed * 0.17).sin()
                    * radius
                    * envelope
                    * 0.72,
            )
        });
    }

    for knot in 0..params.knots {
        let a = hash01(seed, 2300 + knot as u64) * TAU + t * 0.11;
        let r = hash01(seed, 2600 + knot as u64).sqrt() * radius;
        let wobble = (t * (0.4 + hash01(seed, 2900 + knot as u64)) + a).sin();
        let x = cx + a.cos() * r * 1.7 + wobble * params.warp * 2.0;
        let y = cy + a.sin() * r * 0.74 + wobble * params.warp;
        let glyph = match knot % 5 {
            0 => '╳',
            1 => '◆',
            2 => '┼',
            3 => '◇',
            _ => '✦',
        };
        let color = if knot % 2 == 0 {
            lighten(palette[3], (params.bloom * 42.0) as u8)
        } else {
            lighten(palette[1], (params.bloom * 24.0) as u8)
        };
        put(grid, x.round() as i32, y.round() as i32, glyph, color);
    }
}

fn draw_shuttles(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &HyperloomParams,
) {
    for shuttle in 0..params.shuttles {
        let thread = (shuttle * 7 + (seed as usize % params.threads)) % params.threads;
        let phase = hash01(seed, 3300 + shuttle as u64);
        let speed = 0.035 + hash01(seed, 3600 + shuttle as u64) * 0.055;
        let head_u = (phase + t * params.speed * speed).rem_euclid(1.0);
        for trail in (0..params.trail).rev() {
            let u = (head_u - trail as f32 * 0.0065).rem_euclid(1.0);
            let (x, y) =
                warp_thread_point(width, height, seed, thread, params.threads, u, t, params);
            let life = 1.0 - trail as f32 / params.trail.max(1) as f32;
            let color = lerp_color(darken(palette[2], 48), lighten(palette[4], 18), life);
            let glyph = if trail == 0 {
                if shuttle % 2 == 0 { '◀' } else { '▶' }
            } else if life > 0.6 {
                '━'
            } else {
                '·'
            };
            put(grid, x.round() as i32, y.round() as i32, glyph, color);
        }
    }
}

fn draw_jacquard_cards(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &HyperloomParams,
) {
    if width < 6 || height < 4 {
        return;
    }
    let card_width = 5usize;
    let cards = (width / card_width).max(1);
    let offset = (t * params.speed * 3.0).floor() as isize;
    for card in 0..cards {
        let logical = (card as isize + offset).rem_euclid(cards as isize) as usize;
        let x = card * card_width + 2;
        let mask = (hash01(seed, 4100 + logical as u64) * 255.0) as u8;
        let color = darken(thread_color(palette, card, cards, t), 18);
        for bit in 0..3 {
            let punched = mask & (1 << bit) != 0;
            put(
                grid,
                (x + bit) as i32,
                1,
                if punched { '●' } else { '○' },
                color,
            );
            put(
                grid,
                (width - 1 - (x + bit).min(width - 1)) as i32,
                height.saturating_sub(2) as i32,
                if punched { '∙' } else { '·' },
                color,
            );
        }
    }
}

fn draw_woven_border(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &HyperloomParams,
) {
    if width == 0 || height == 0 {
        return;
    }
    let phase = (t * params.speed * 2.0).floor() as usize;
    for x in 0..width {
        let index = (x + phase + seed as usize) % 8;
        let glyph = ['╱', '╲', '◆', '─', '╲', '╱', '◇', '─'][index];
        let color = thread_color(palette, x % params.threads, params.threads, t);
        put(grid, x as i32, 0, glyph, color);
        put(
            grid,
            x as i32,
            height.saturating_sub(1) as i32,
            glyph,
            color,
        );
    }
    for y in 0..height {
        let index = (y + phase + (seed >> 8) as usize) % 6;
        let glyph = ['│', '◇', '╳', '│', '◆', '┼'][index];
        let color = thread_color(palette, y % params.threads, params.threads, -t);
        put(grid, 0, y as i32, glyph, color);
        put(grid, width.saturating_sub(1) as i32, y as i32, glyph, color);
    }
}

pub(crate) fn draw_hyperloom(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    _rng: &mut StdRng,
    t: f32,
    params: &HyperloomParams,
) {
    if width == 0 || height == 0 {
        return;
    }
    let t = t * params.speed;

    // 1. Initialize the full spectral field from dimensions, seed, and time.
    draw_spectral_field(grid, width, height, seed, palette, t, params);
    // 2. Evaluate bounded moving apertures before the thread layers cross them.
    draw_loom_apertures(grid, width, height, seed, palette, t, params);
    // 3. Interlace analytic warp and weft curves with deterministic phases.
    draw_threads(grid, width, height, seed, palette, t, params);
    // 4. Overlay nested knot cages and seeded crossing jewels.
    draw_knot_cages(grid, width, height, seed, palette, t, params);
    // 5. Reconstruct shuttle trails directly from the current time value.
    draw_shuttles(grid, width, height, seed, palette, t, params);
    // 6. Seal the frame with moving punch cards and a woven machine border.
    draw_jacquard_cards(grid, width, height, seed, palette, t, params);
    draw_woven_border(grid, width, height, seed, palette, t, params);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use crate::render::grid_to_plain;
    use rand::SeedableRng;

    fn frame(width: usize, height: usize, seed: u64, t: f32, params: &HyperloomParams) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        draw_hyperloom(
            &mut grid, width, height, seed, &palette, &mut rng, t, params,
        );
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = HyperloomParams::default();
        let a = plain(&frame(88, 30, 42, 1.25, &params));
        let b = plain(&frame(88, 30, 42, 1.25, &params));
        let moved = plain(&frame(88, 30, 42, 3.75, &params));
        let reseeded = plain(&frame(88, 30, 43, 1.25, &params));
        let varied = plain(&frame(
            88,
            30,
            42,
            1.25,
            &HyperloomParams {
                symmetry: 14,
                threads: 30,
                weave: 7.5,
                shuttles: 14,
                ..params
            },
        ));
        assert_eq!(a, b);
        assert_ne!(a, moved);
        assert_ne!(a, reseeded);
        assert_ne!(a, varied);
    }

    #[test]
    fn parameter_values_override_and_clamp() {
        let values = [
            100.0, 0.0, 99.0, 99.0, 99.0, 999.0, 0.0, 9.0, 99.0, 9.0, 0.0, 9.0,
        ];
        let params = HyperloomParams::from_inputs(&[], Some(&values));
        insta::assert_debug_snapshot!(params, @r###"
        HyperloomParams {
            threads: 36,
            looms: 1,
            symmetry: 18,
            depth: 8,
            shuttles: 18,
            knots: 72,
            speed: 0.05,
            warp: 1.5,
            weave: 9.0,
            moire: 1.5,
            trail: 1,
            bloom: 1.5,
        }
        "###);
    }

    #[test]
    fn tiny_grid_and_extreme_inputs_terminate() {
        let params = HyperloomParams {
            threads: 36,
            looms: 7,
            symmetry: 18,
            depth: 8,
            shuttles: 18,
            knots: 72,
            speed: 3.0,
            warp: 1.5,
            weave: 9.0,
            moire: 1.5,
            trail: 24,
            bloom: 1.5,
        };
        assert_eq!(frame(1, 1, 7, 50_000.0, &params).len(), 1);
        assert_eq!(frame(5, 3, 7, 50_000.0, &params).len(), 3);
    }

    #[test]
    fn snapshot_hyperloom_t0() {
        insta::assert_snapshot!(plain(&frame(96, 32, 42, 0.0, &HyperloomParams::default())));
    }

    #[test]
    fn snapshot_hyperloom_in_motion() {
        insta::assert_snapshot!(plain(&frame(96, 32, 42, 2.75, &HyperloomParams::default())));
    }
}
