use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;

use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::pp::{pp_fbm, pp_line, pp_put, pp_stroke};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct IlluminariumMode;

pub(super) static MODE: IlluminariumMode = IlluminariumMode;

const PARAMS: &[Param] = &[
    param!("SYMM", "symmetry", 4.0, 28.0, 12.0, 1.0),
    param!("RINGS", "rose rings", 3.0, 14.0, 7.0, 1.0),
    param!("FILI", "filigree", 0.0, 1.0, 0.72, 0.04),
    param!("ORBITS", "orbiters", 1.0, 24.0, 9.0, 1.0),
    param!("SPEED", "rotation", 0.05, 3.0, 0.65, 0.05),
    param!("WARP", "rose warp", 0.0, 1.0, 0.35, 0.05),
    param!("TRAIL", "comet trail", 0.0, 18.0, 7.0, 1.0),
    param!("SPARKS", "light motes", 0.0, 320.0, 90.0, 10.0),
    param!("DEPTH", "branch depth", 1.0, 6.0, 4.0, 1.0),
    param!("BLOOM", "bloom", 0.0, 1.5, 0.72, 0.05),
];

impl Mode for IlluminariumMode {
    fn name(&self) -> &'static str {
        "illuminarium"
    }

    fn help(&self) -> &'static str {
        "Rotating rose vault, guilloche, recursive filigree, comet jewels [symm] [rings] [fili] [orbits] [speed] [warp] [trail] [sparks] [depth] [bloom]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = IlluminariumParams::from_args(frame.args);
        draw_illuminarium(
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
pub(crate) struct IlluminariumParams {
    pub(crate) symmetry: usize,
    pub(crate) rings: usize,
    pub(crate) filigree: f32,
    pub(crate) orbiters: usize,
    pub(crate) speed: f32,
    pub(crate) warp: f32,
    pub(crate) trails: usize,
    pub(crate) sparks: usize,
    pub(crate) depth: usize,
    pub(crate) bloom: f32,
}

impl Default for IlluminariumParams {
    fn default() -> Self {
        Self {
            symmetry: 12,
            rings: 7,
            filigree: 0.72,
            orbiters: 9,
            speed: 0.65,
            warp: 0.35,
            trails: 7,
            sparks: 90,
            depth: 4,
            bloom: 0.72,
        }
    }
}

impl IlluminariumParams {
    pub(crate) fn from_args(args: &[String]) -> Self {
        let read = |index: usize, key: &str, default: f32| {
            args.get(index)
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or_else(|| param_f32(key, default))
        };
        Self {
            symmetry: read(4, "SYMM", 12.0).round().clamp(4.0, 28.0) as usize,
            rings: read(5, "RINGS", 7.0).round().clamp(3.0, 14.0) as usize,
            filigree: read(6, "FILI", 0.72).clamp(0.0, 1.0),
            orbiters: read(7, "ORBITS", 9.0).round().clamp(1.0, 24.0) as usize,
            speed: read(8, "SPEED", 0.65).clamp(0.05, 3.0),
            warp: read(9, "WARP", 0.35).clamp(0.0, 1.0),
            trails: read(10, "TRAIL", 7.0).round().clamp(0.0, 18.0) as usize,
            sparks: read(11, "SPARKS", 90.0).round().clamp(0.0, 320.0) as usize,
            depth: read(12, "DEPTH", 4.0).round().clamp(1.0, 6.0) as usize,
            bloom: read(13, "BLOOM", 0.72).clamp(0.0, 1.5),
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

fn line(grid: &mut Grid, a: (i32, i32), b: (i32, i32), color: Color) {
    pp_line(grid, a.0, a.1, b.0, b.1, color);
}

fn color_wheel(palette: &[Color; 5], index: usize, phase: f32) -> Color {
    let base = palette[1 + index % 3];
    let shift = index as f64 * 37.0 + phase as f64 * 24.0;
    shift_hue(lighten(base, 18), shift)
}

fn trace_curve<F>(
    grid: &mut Grid,
    samples: usize,
    connect: bool,
    glyph: char,
    color: Color,
    mut point: F,
) where
    F: FnMut(f32) -> (f32, f32),
{
    let mut previous: Option<(i32, i32)> = None;
    for sample in 0..=samples {
        let u = sample as f32 / samples.max(1) as f32;
        let p = point(u);
        let here = (p.0.round() as i32, p.1.round() as i32);
        if connect {
            if let Some(prior) = previous {
                if prior != here {
                    line(grid, prior, here, color);
                }
            }
        } else if sample % 2 == 0 {
            put(grid, here.0, here.1, glyph, color);
        }
        previous = Some(here);
    }
}

fn cubic_point(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    u: f32,
) -> (f32, f32) {
    let v = 1.0 - u;
    let a = v * v * v;
    let b = 3.0 * v * v * u;
    let c = 3.0 * v * u * u;
    let d = u * u * u;
    (
        p0.0 * a + p1.0 * b + p2.0 * c + p3.0 * d,
        p0.1 * a + p1.1 * b + p2.1 * c + p3.1 * d,
    )
}

fn draw_background(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    bloom: f32,
) {
    let top = darken(palette[0], 18);
    let bottom = darken(shift_hue(palette[2], 24.0), 54);
    let span = height.saturating_sub(1).max(1) as f32;
    for y in 0..height {
        let vertical = y as f32 / span;
        let bg = lerp_color(top, bottom, vertical * vertical * 0.82);
        for x in 0..width {
            grid[y][x] = Cell::with_bg(' ', bg, bg);
        }
    }

    let star_color = darken(palette[4], 38);
    let dust_color = darken(shift_hue(palette[3], 35.0), 58);
    for y in 0..height {
        for x in 0..width {
            let field = pp_fbm(x as f32 * 0.075, y as f32 * 0.13, seed ^ 0x51A7);
            let pick = hash01(seed, x as u64 * 131 + y as u64 * 977);
            if field > 0.72 && pick > 0.82 - bloom * 0.08 {
                let twinkle = (t * 0.9 + pick * std::f32::consts::TAU * 2.0).sin();
                let ch = if twinkle > 0.84 {
                    '✦'
                } else if twinkle > 0.18 {
                    '∙'
                } else {
                    '·'
                };
                put(grid, x as i32, y as i32, ch, star_color);
            } else if field > 0.63 && pick > 0.91 {
                put(grid, x as i32, y as i32, '·', dust_color);
            }
        }
    }
}

fn draw_arch_frame(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    symmetry: usize,
) {
    if width < 8 || height < 6 {
        return;
    }
    let edge = darken(lighten(palette[1], 24), 18);
    let inner = darken(shift_hue(lighten(palette[3], 18), 42.0), 26);
    let phase = (t * 2.0 + seed as f32 * 0.013).floor() as usize;

    for inset in 0..2usize {
        let left = inset as i32;
        let right = width as i32 - 1 - inset as i32;
        let top = inset as i32;
        let bottom = height as i32 - 1 - inset as i32;
        let color = if inset == 0 { edge } else { inner };
        for x in left..=right {
            let pattern = (x as usize + phase + inset * 3) % 8;
            let ch = match pattern {
                0 => '◆',
                4 => '◇',
                _ => '─',
            };
            put(grid, x, top, ch, color);
            put(grid, x, bottom, ch, color);
        }
        for y in top..=bottom {
            let pattern = (y as usize * 2 + phase + inset) % 9;
            let ch = if pattern == 0 { '○' } else { '│' };
            put(grid, left, y, ch, color);
            put(grid, right, y, ch, color);
        }
    }

    let corners = [
        (1, 1, 1.0, 1.0),
        (width as i32 - 2, 1, -1.0, 1.0),
        (1, height as i32 - 2, 1.0, -1.0),
        (width as i32 - 2, height as i32 - 2, -1.0, -1.0),
    ];
    for (corner_index, &(x, y, sx, sy)) in corners.iter().enumerate() {
        for arm in 0..3 {
            let length = 3.0 + arm as f32 * 1.8;
            let bend = (corner_index as f32 * 1.7 + arm as f32 + t * 0.21).sin();
            let end = (
                x as f32 + sx * length * 2.0,
                y as f32 + sy * length * (0.55 + bend.abs() * 0.18),
            );
            let control = (x as f32 + sx * length, y as f32 + sy * (1.0 + arm as f32));
            trace_curve(grid, 18, true, '·', inner, |u| {
                cubic_point(
                    (x as f32, y as f32),
                    control,
                    (end.0 - sx * 2.0, end.1 + sy * bend),
                    end,
                    u,
                )
            });
        }
        put(
            grid,
            x,
            y,
            ['╭', '╮', '╰', '╯'][corner_index],
            lighten(edge, 12),
        );
    }

    let runes = ['◆', '◇', '○', '⊙', '✦', '∙'];
    for i in 0..symmetry {
        let x = 3 + i * width.saturating_sub(6) / symmetry.max(1);
        if x < width.saturating_sub(2) {
            put(
                grid,
                x as i32,
                2,
                runes[(i + seed as usize) % runes.len()],
                darken(inner, 4),
            );
        }
    }
}

fn draw_guilloche(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    warp: f32,
    rings: usize,
) {
    let tau = std::f32::consts::TAU;
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let rx = width as f32 * 0.46;
    let ry = height as f32 * 0.43;
    let samples = (width + height).max(40) * 6;
    for ribbon in 0..3usize {
        let a_freq = 2.0 + ((seed as usize + ribbon * 3) % 5) as f32;
        let b_freq = a_freq + 1.0 + (rings % 3) as f32;
        let phase = hash01(seed, 800 + ribbon as u64) * tau;
        let color = darken(
            color_wheel(palette, ribbon + 1, t * 0.2),
            48 - ribbon as u8 * 6,
        );
        trace_curve(grid, samples, false, '·', color, |u| {
            let a = u * tau;
            let breathe = 0.82 + 0.08 * (a * 3.0 - t * 0.32).sin();
            let x = cx
                + rx * breathe
                    * (a * a_freq + phase + t * 0.07 * (ribbon as f32 + 1.0)).sin()
                    * (0.72 + warp * 0.18 * (a * b_freq - t * 0.11).cos());
            let y = cy
                + ry * breathe
                    * (a * b_freq - phase * 0.37 - t * 0.05 * (ribbon as f32 + 1.0)).sin();
            (x, y)
        });
    }
}

fn draw_rose_lattice(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &IlluminariumParams,
) {
    let tau = std::f32::consts::TAU;
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_rx = width as f32 * 0.43;
    let max_ry = height as f32 * 0.40;
    let samples = (width + height).max(48) * 5;

    for ring in 0..params.rings {
        let fraction = (ring + 1) as f32 / (params.rings + 1) as f32;
        let petals = params.symmetry + (ring % 3) * 2;
        let direction = if ring % 2 == 0 { 1.0 } else { -1.0 };
        let seed_phase = hash01(seed, 1000 + ring as u64) * tau;
        let rotation = seed_phase + direction * t * params.speed * (0.035 + ring as f32 * 0.006);
        let pulse = 1.0 + params.bloom * 0.035 * (t * 0.8 + seed_phase).sin();
        let color = darken(
            color_wheel(palette, ring, t * params.speed * 0.12),
            (params.rings.saturating_sub(ring) * 2).min(20) as u8,
        );
        trace_curve(grid, samples, true, '·', color, |u| {
            let angle = u * tau;
            let rose =
                1.0 + (0.08 + params.warp * 0.13) * (petals as f32 * angle + rotation * 2.0).cos();
            let interlace = 1.0 + 0.035 * ((petals - 1) as f32 * angle - rotation).sin();
            let r = fraction * rose * interlace * pulse;
            (
                cx + max_rx * r * (angle + rotation).cos(),
                cy + max_ry * r * (angle + rotation).sin(),
            )
        });

        for petal in 0..petals {
            let angle = petal as f32 * tau / petals as f32 + rotation;
            let rose = 1.0 + (0.08 + params.warp * 0.13) * (petals as f32 * angle).cos();
            let x = cx + max_rx * fraction * rose * angle.cos();
            let y = cy + max_ry * fraction * rose * angle.sin();
            let glyph = if ring + 1 == params.rings {
                if petal % 2 == 0 { '◆' } else { '◇' }
            } else if petal % 3 == 0 {
                '○'
            } else {
                '∙'
            };
            put(
                grid,
                x.round() as i32,
                y.round() as i32,
                glyph,
                lighten(color, 10),
            );
        }
    }
}

fn draw_branch(
    grid: &mut Grid,
    start: (f32, f32),
    angle: f32,
    length: f32,
    depth: usize,
    branch_id: u64,
    seed: u64,
    color: Color,
    t: f32,
    filigree: f32,
) {
    if depth == 0 || length < 0.7 {
        put(
            grid,
            start.0.round() as i32,
            start.1.round() as i32,
            '✦',
            lighten(color, 8),
        );
        return;
    }

    let bend = (hash01(seed, branch_id * 11 + depth as u64) - 0.5) * 0.9;
    let sway = (t * 0.18 + branch_id as f32 * 0.31).sin() * 0.08 * filigree;
    let final_angle = angle + bend * filigree + sway;
    let end = (
        start.0 + final_angle.cos() * length * 2.0,
        start.1 + final_angle.sin() * length,
    );
    let normal = (-final_angle.sin(), final_angle.cos());
    let control1 = (
        start.0 + final_angle.cos() * length * 0.65 + normal.0 * bend * 1.8,
        start.1 + final_angle.sin() * length * 0.32 + normal.1 * bend,
    );
    let control2 = (
        end.0 - final_angle.cos() * length * 0.55 - normal.0 * bend * 1.4,
        end.1 - final_angle.sin() * length * 0.28 - normal.1 * bend,
    );
    trace_curve(grid, 12, true, '·', color, |u| {
        cubic_point(start, control1, control2, end, u)
    });
    put(
        grid,
        end.0.round() as i32,
        end.1.round() as i32,
        if depth % 2 == 0 { '◇' } else { '∙' },
        lighten(color, 5),
    );

    let split = 0.38 + hash01(seed, branch_id * 19 + 5) * 0.34;
    let next = length * (0.63 + hash01(seed, branch_id * 23 + 7) * 0.09);
    draw_branch(
        grid,
        end,
        final_angle - split,
        next,
        depth - 1,
        branch_id * 2 + 1,
        seed,
        darken(color, 2),
        t,
        filigree,
    );
    draw_branch(
        grid,
        end,
        final_angle + split,
        next * 0.92,
        depth - 1,
        branch_id * 2 + 2,
        seed,
        shift_hue(color, 8.0),
        t,
        filigree,
    );
}

fn draw_filigree(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &IlluminariumParams,
) {
    if params.filigree <= 0.01 {
        return;
    }
    let tau = std::f32::consts::TAU;
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let arms = params.symmetry.min(18);
    let base_rx = width as f32 * 0.27;
    let base_ry = height as f32 * 0.25;
    let length = (width as f32 / arms.max(4) as f32 * 0.65)
        .min(height as f32 * 0.11)
        .max(1.1)
        * (0.7 + params.filigree * 0.5);
    let rotation = t * params.speed * -0.025 + hash01(seed, 4040) * tau;
    for arm in 0..arms {
        let angle = arm as f32 * tau / arms as f32 + rotation;
        let start = (cx + base_rx * angle.cos(), cy + base_ry * angle.sin());
        let tangent = angle + std::f32::consts::FRAC_PI_2 * if arm % 2 == 0 { 1.0 } else { -1.0 };
        let color = darken(color_wheel(palette, arm, t * 0.08), 14);
        draw_branch(
            grid,
            start,
            tangent,
            length,
            params.depth,
            5000 + arm as u64 * 97,
            seed,
            color,
            t,
            params.filigree,
        );
    }
}

fn draw_orbits(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &IlluminariumParams,
) {
    let tau = std::f32::consts::TAU;
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let heads = ['◆', '●', '⊙', '✦'];
    let tails = ['·', '∙', '○', '◇'];
    for orbiter in 0..params.orbiters {
        let phase = hash01(seed, 7000 + orbiter as u64 * 13) * tau;
        let band = orbiter % params.rings.max(1);
        let radius = 0.16 + 0.69 * (band + 1) as f32 / (params.rings + 1) as f32;
        let eccentricity = 0.70 + hash01(seed, 7100 + orbiter as u64) * 0.26;
        let direction = if orbiter % 2 == 0 { 1.0 } else { -1.0 };
        let rate = 0.18 + hash01(seed, 7200 + orbiter as u64) * 0.38;
        let color = color_wheel(palette, orbiter + 2, t * params.speed * 0.18);
        for trail in (0..=params.trails).rev() {
            let age = trail as f32 * 0.055;
            let angle = phase + direction * (t * params.speed - age) * rate;
            let wobble = 1.0 + params.warp * 0.08 * (angle * 3.0 + phase).sin();
            let x = cx + width as f32 * 0.44 * radius * wobble * angle.cos();
            let y = cy
                + height as f32
                    * 0.40
                    * radius
                    * eccentricity
                    * wobble
                    * (angle + params.warp * 0.13 * (angle * 2.0).sin()).sin();
            let ch = if trail == 0 {
                heads[orbiter % heads.len()]
            } else {
                tails[(trail + orbiter) % tails.len()]
            };
            let fade = ((trail as f32 / params.trails.max(1) as f32) * 54.0) as u8;
            put(
                grid,
                x.round() as i32,
                y.round() as i32,
                ch,
                darken(color, fade),
            );
        }
    }
}

fn draw_sparks(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &IlluminariumParams,
) {
    let tau = std::f32::consts::TAU;
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let glyphs = ['·', '∙', '*', '+', '✧', '✦'];
    for spark in 0..params.sparks {
        let base = spark as u64 * 41;
        let angle0 = hash01(seed, 9000 + base) * tau;
        let radius0 = hash01(seed, 9001 + base).sqrt();
        let rate = (hash01(seed, 9002 + base) - 0.5) * 0.20;
        let wave = hash01(seed, 9003 + base) * tau;
        let angle = angle0 + t * params.speed * rate;
        let radius = (radius0 + 0.035 * (t * 0.37 + wave).sin()).clamp(0.04, 0.98);
        let x = cx + width as f32 * 0.45 * radius * angle.cos();
        let y = cy + height as f32 * 0.41 * radius * angle.sin();
        let pulse = (t * (0.6 + hash01(seed, 9004 + base)) + wave).sin();
        let glyph_index = ((pulse + 1.0) * 0.5 * (glyphs.len() - 1) as f32).round() as usize;
        let color = color_wheel(palette, spark, t * 0.05);
        put(
            grid,
            x.round() as i32,
            y.round() as i32,
            glyphs[glyph_index.min(glyphs.len() - 1)],
            darken(color, if pulse > 0.55 { 4 } else { 28 }),
        );
    }
}

fn draw_rosette(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &IlluminariumParams,
) {
    let tau = std::f32::consts::TAU;
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let petal_rx = (width as f32 * 0.14).clamp(4.0, 15.0);
    let petal_ry = (height as f32 * 0.16).clamp(2.5, 7.0);
    let rotation = t * params.speed * 0.08 + hash01(seed, 10001) * tau;
    let petals = params.symmetry;

    for petal in 0..petals {
        let angle = petal as f32 * tau / petals as f32 + rotation;
        let tangent = (-angle.sin(), angle.cos());
        let tip = (cx + petal_rx * angle.cos(), cy + petal_ry * angle.sin());
        let shoulder = petal_rx * (0.36 + params.bloom * 0.12);
        let color = color_wheel(palette, petal, t * params.speed * 0.16);
        for side in [-1.0f32, 1.0] {
            let control1 = (
                cx + tangent.0 * shoulder * side,
                cy + tangent.1 * shoulder * 0.55 * side,
            );
            let control2 = (
                tip.0 + tangent.0 * shoulder * 0.58 * side,
                tip.1 + tangent.1 * shoulder * 0.34 * side,
            );
            trace_curve(grid, 20, true, '·', darken(color, 2), |u| {
                cubic_point((cx, cy), control1, control2, tip, u)
            });
        }
        put(
            grid,
            tip.0.round() as i32,
            tip.1.round() as i32,
            if petal % 2 == 0 { '◆' } else { '◇' },
            lighten(color, 16),
        );
    }

    let core_rx = (petal_rx * 0.43).max(2.0);
    let core_ry = (petal_ry * 0.48).max(1.4);
    for ring in 0..4usize {
        let fraction = (ring + 1) as f32 / 4.0;
        let color = color_wheel(palette, ring + petals, t * 0.2);
        trace_curve(grid, 90, true, '·', color, |u| {
            let angle = u * tau - rotation * (0.4 + ring as f32 * 0.1);
            (
                cx + core_rx * fraction * angle.cos(),
                cy + core_ry * fraction * angle.sin(),
            )
        });
    }
    let center_color = lighten(shift_hue(palette[4], t as f64 * 18.0), 20);
    put(
        grid,
        cx.round() as i32,
        cy.round() as i32,
        '◉',
        center_color,
    );
    put(
        grid,
        cx.round() as i32 - 1,
        cy.round() as i32,
        '⊙',
        lighten(palette[3], 20),
    );
    put(
        grid,
        cx.round() as i32 + 1,
        cy.round() as i32,
        '⊙',
        lighten(palette[1], 20),
    );
}

pub(crate) fn draw_illuminarium(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    params: &IlluminariumParams,
) {
    if width == 0 || height == 0 {
        return;
    }
    let t = t + rng.random_range(0.0..std::f32::consts::TAU) * 0.03;

    // Establish the colored night field and seeded stellar dust.
    draw_background(grid, width, height, seed, palette, t, params.bloom);
    // Lay quiet full-frame harmonographs beneath the radial architecture.
    draw_guilloche(
        grid,
        width,
        height,
        seed,
        palette,
        t,
        params.warp,
        params.rings,
    );
    // Build the nested rotating rose vault and its indexed jewels.
    draw_rose_lattice(grid, width, height, seed, palette, t, params);
    // Grow mirrored recursive ornament from the middle rings.
    draw_filigree(grid, width, height, seed, palette, t, params);
    // Thread analytically reconstructed comet trails through the vault.
    draw_orbits(grid, width, height, seed, palette, t, params);
    // Add seeded time-varying light points before the opaque central medallion.
    draw_sparks(grid, width, height, seed, palette, t, params);
    // Seal the composition with a rotating bezier rosette and luminous core.
    draw_rosette(grid, width, height, seed, palette, t, params);
    // Draw the animated architectural border last so it remains legible.
    draw_arch_frame(grid, width, height, seed, palette, t, params.symmetry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use crate::render::grid_to_plain;
    use rand::SeedableRng;

    fn frame(width: usize, height: usize, seed: u64, t: f32, params: &IlluminariumParams) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        draw_illuminarium(
            &mut grid, width, height, seed, &palette, &mut rng, t, params,
        );
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = IlluminariumParams::default();
        let a = plain(&frame(80, 36, 42, 1.25, &params));
        let b = plain(&frame(80, 36, 42, 1.25, &params));
        let c = plain(&frame(80, 36, 42, 3.75, &params));
        let different_seed = plain(&frame(80, 36, 43, 1.25, &params));
        let tuned = IlluminariumParams {
            symmetry: 17,
            rings: 10,
            filigree: 0.4,
            orbiters: 15,
            warp: 0.8,
            ..params
        };
        let different_inputs = plain(&frame(80, 36, 42, 1.25, &tuned));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, different_seed);
        assert_ne!(a, different_inputs);
        assert_eq!(
            a.lines()
                .map(str::chars)
                .map(Iterator::count)
                .collect::<Vec<_>>(),
            vec![80; 36]
        );
    }

    #[test]
    fn tiny_grid_and_extreme_inputs_terminate() {
        let params = IlluminariumParams {
            symmetry: 28,
            rings: 14,
            filigree: 1.0,
            orbiters: 24,
            speed: 3.0,
            warp: 1.0,
            trails: 18,
            sparks: 320,
            depth: 6,
            bloom: 1.5,
        };
        let output = frame(12, 6, 7, 8.0, &params);
        assert_eq!(output.len(), 6);
        assert_eq!(output.iter().map(Vec::len).collect::<Vec<_>>(), vec![12; 6]);
    }

    #[test]
    fn snapshot_illuminarium_in_motion() {
        let params = IlluminariumParams::default();
        insta::assert_snapshot!(
            "illuminarium_t2_75",
            plain(&frame(80, 36, 42, 2.75, &params))
        );
    }
}
