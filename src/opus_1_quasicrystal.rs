//! opus-1-quasicrystal -- de Bruijn's multigrid dual. N families of parallel lines
//! become a quasiperiodic rhombic tiling that reconfigures as the offsets drift.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::PI;

const SYMS: [u32; 6] = [4, 5, 5, 6, 7, 9];
const RAMP: [char; 8] = [' ', '.', ',', ':', ';', '%', '#', '@'];
const LEVELS: usize = 8;
const CLASS_MAX: usize = 8;
const WORM_SEQ: usize = 64;
const WORM_LIFE: f32 = 13.0;
const BREATH_PERIOD: f32 = 37.0;
const WAVE_PERIOD: f32 = 26.0;
const MAX_RHOMBS: usize = 260_000;

pub(crate) struct Opus1QuasicrystalKnobs {
    pub speed: f32,
    pub hue: f32,
    pub sym: f32,
    pub scale: f32,
    pub drift: f32,
    pub spin: f32,
    pub shade: f32,
    pub wave: f32,
    pub worms: f32,
    pub pulse: f32,
    pub stars: f32,
    pub breath: f32,
}

impl Opus1QuasicrystalKnobs {
    pub(crate) fn from_env() -> Self {
        Opus1QuasicrystalKnobs {
            speed: param_f32("SPEED", 1.0),
            hue: param_f32("HUE", 0.0),
            sym: param_f32("SYM", 0.0),
            scale: param_f32("SCALE", 12.0),
            drift: param_f32("DRIFT", 0.03),
            spin: param_f32("SPIN", 1.5),
            shade: param_f32("SHADE", 0.85),
            wave: param_f32("WAVE", 1.8),
            worms: param_f32("WORMS", 2.0),
            pulse: param_f32("PULSE", 0.06),
            stars: param_f32("STARS", 0.3),
            breath: param_f32("BREATH", 0.12),
        }
    }
}

#[derive(Clone, Copy)]
struct Rhomb {
    ax: f32,
    ay: f32,
    j: u8,
    l: u8,
    level: u8,
    class: u8,
    worm: f32,
    star: u8,
}

struct Cached {
    key: (u64, u32),
    n: usize,
    gamma0: Vec<f32>,
    grate: Vec<f32>,
    rot0: f32,
    hue0: f32,
    wave_ang: f32,
    fv: Vec<(f32, f32)>,
    seq: Vec<(u8, f32, f32)>,
    eu: Vec<(f32, f32)>,
    ev: Vec<(f32, f32)>,
    gam: Vec<f32>,
    glyph: Vec<char>,
    lit: Vec<i32>,
    head: Vec<f32>,
    rhombs: Vec<Rhomb>,
    shade_rows: Vec<Vec<usize>>,
}

thread_local! {
    static CACHE: RefCell<Option<Cached>> = const { RefCell::new(None) };
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn choose_sym(seed: u64, knob: f32) -> u32 {
    let forced = knob.round() as i32;
    if forced >= 3 {
        return (forced as u32).min(13);
    }
    let mut rng = StdRng::seed_from_u64(seed ^ 0x5177_A51C_0DE1);
    SYMS[rng.random_range(0..SYMS.len())]
}

fn build(seed: u64, n: u32) -> Cached {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x9B1D_E7F0_2C55);
    let nn = n as usize;
    let mut gamma0 = Vec::with_capacity(nn);
    let mut grate = Vec::with_capacity(nn);
    for _ in 0..nn {
        gamma0.push(rng.random::<f32>() * 0.86 + 0.07);
        grate.push(rng.random::<f32>() * 2.0 - 1.0);
    }
    let mut ax = 0.0f32;
    let mut ay = 0.0f32;
    for (m, r) in grate.iter().enumerate() {
        let a = PI * m as f32 / n as f32;
        ax += r * a.cos();
        ay += r * a.sin();
    }
    let mut peak = 1e-4f32;
    for (m, r) in grate.iter_mut().enumerate() {
        let a = PI * m as f32 / n as f32;
        *r -= 2.0 / n as f32 * (ax * a.cos() + ay * a.sin());
        peak = peak.max(r.abs());
    }
    for r in grate.iter_mut() {
        *r /= peak;
    }

    let mut qs: Vec<u32> = (3..(2 * n).max(4))
        .step_by(2)
        .filter(|q| *q != 2 * n - 1 && gcd(*q, 2 * n) == 1)
        .collect();
    if qs.is_empty() {
        qs.push(3);
    }
    let q = qs[rng.random_range(0..qs.len())];
    let mut fv = Vec::with_capacity(nn);
    for m in 0..nn {
        let a = PI * (m as u32 * q) as f32 / n as f32;
        fv.push((a.cos(), a.sin()));
    }

    let mut seq = Vec::with_capacity(WORM_SEQ);
    for _ in 0..WORM_SEQ {
        let fam = rng.random_range(0..nn) as u8;
        let where_on = rng.random::<f32>() * 1.6 - 0.8;
        let phase = rng.random::<f32>();
        seq.push((fam, where_on, phase));
    }

    Cached {
        key: (seed, n),
        n: nn,
        gamma0,
        grate,
        rot0: rng.random::<f32>() * PI,
        hue0: rng.random::<f32>() * 360.0,
        wave_ang: rng.random::<f32>() * 2.0 * PI,
        fv,
        seq,
        eu: vec![(0.0, 0.0); nn],
        ev: vec![(0.0, 0.0); nn],
        gam: vec![0.0; nn],
        glyph: vec!['-'; nn],
        lit: vec![i32::MIN; nn],
        head: vec![0.0; nn],
        rhombs: Vec::new(),
        shade_rows: Vec::new(),
    }
}

fn hash2(x: i32, y: i32, salt: u32) -> f32 {
    let mut h = (x as u64 & 0xFFFF).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64 & 0xFFFF).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (salt as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 29;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 32;
    (h & 0xFF_FFFF) as f32 / 16_777_216.0
}

fn slope_glyph(dx: f32, dy: f32) -> char {
    if dx.abs() < 1e-3 {
        return '|';
    }
    let s = dy / dx;
    if s.abs() < 0.42 {
        '-'
    } else if s.abs() > 2.3 {
        '|'
    } else if s > 0.0 {
        '\\'
    } else {
        '/'
    }
}

fn keep_bg(grid: &Grid, x: usize, y: usize) -> Color {
    grid[y][x].bg
}

fn stroke(grid: &mut Grid, w: usize, h: usize, ax: f32, ay: f32, d: (f32, f32), ch: char, fg: Color) {
    let steps = d.0.abs().max(d.1.abs()).ceil().max(1.0);
    let inv = 1.0 / steps;
    let n = steps as i32;
    for i in 0..n {
        let f = i as f32 * inv;
        let px = ax + d.0 * f;
        let py = ay + d.1 * f;
        if px < 0.0 || py < 0.0 {
            continue;
        }
        let (x, y) = (px as usize, py as usize);
        if x < w && y < h {
            let bg = keep_bg(grid, x, y);
            grid[y][x] = Cell::with_bg(ch, fg, bg);
        }
    }
}

fn fill_par(
    grid: &mut Grid,
    w: usize,
    h: usize,
    ax: f32,
    ay: f32,
    u: (f32, f32),
    v: (f32, f32),
    a: Cell,
    b: Cell,
    mix: f32,
    salt: u32,
) {
    let det = u.0 * v.1 - u.1 * v.0;
    if det.abs() < 1e-3 {
        return;
    }
    let inv = 1.0 / det;
    let ys = [ay, ay + u.1, ay + v.1, ay + u.1 + v.1];
    let mut ymin = ys[0];
    let mut ymax = ys[0];
    for y in ys.iter().skip(1) {
        ymin = ymin.min(*y);
        ymax = ymax.max(*y);
    }
    let y0 = (ymin - 0.5).ceil().max(0.0) as usize;
    let y1t = (ymax - 0.5).floor();
    if y1t < 0.0 || y0 >= h {
        return;
    }
    let y1 = (y1t as usize).min(h - 1);
    let au = v.1 * inv;
    let aw = -u.1 * inv;
    for y in y0..=y1 {
        let dy = y as f32 + 0.5 - ay;
        let bu = -ax * v.1 * inv - dy * v.0 * inv;
        let bw = (u.0 * dy + u.1 * ax) * inv;
        let mut lo = 0.0f32;
        let mut hi = w as f32;
        let mut dead = false;
        for (s, c) in [(au, bu), (aw, bw)] {
            if s > 1e-6 {
                lo = lo.max(-c / s);
                hi = hi.min((1.0 - c) / s);
            } else if s < -1e-6 {
                lo = lo.max((1.0 - c) / s);
                hi = hi.min(-c / s);
            } else if !(0.0..=1.0).contains(&c) {
                dead = true;
            }
        }
        if dead || hi <= lo {
            continue;
        }
        let x0 = (lo - 0.5).ceil().max(0.0) as usize;
        let x1t = (hi - 0.5).floor();
        if x1t < 0.0 || x0 >= w {
            continue;
        }
        let x1 = (x1t as usize).min(w - 1);
        if x0 > x1 {
            continue;
        }
        let row = &mut grid[y];
        if mix <= 0.0 {
            row[x0..=x1].fill(a);
        } else {
            for (x, cell) in row[x0..=x1].iter_mut().enumerate() {
                *cell = if hash2((x0 + x) as i32, y as i32, salt) < mix { b } else { a };
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_par_row(
    row: &mut [Cell], y: usize, ax: f32, ay: f32, u: (f32, f32), v: (f32, f32),
    a: Cell, b: Cell, mix: f32, salt: u32,
) {
    let det = u.0 * v.1 - u.1 * v.0;
    if det.abs() < 1e-3 { return; }
    let inv = 1.0 / det;
    let dy = y as f32 + 0.5 - ay;
    let au = v.1 * inv;
    let aw = -u.1 * inv;
    let bu = -ax * v.1 * inv - dy * v.0 * inv;
    let bw = (u.0 * dy + u.1 * ax) * inv;
    let mut lo = 0.0f32;
    let mut hi = row.len() as f32;
    for (s, c) in [(au, bu), (aw, bw)] {
        if s > 1e-6 {
            lo = lo.max(-c / s);
            hi = hi.min((1.0 - c) / s);
        } else if s < -1e-6 {
            lo = lo.max((1.0 - c) / s);
            hi = hi.min(-c / s);
        } else if !(0.0..=1.0).contains(&c) { return; }
    }
    if hi <= lo { return; }
    let x0 = (lo - 0.5).ceil().max(0.0) as usize;
    let x1t = (hi - 0.5).floor();
    if x1t < 0.0 || x0 >= row.len() { return; }
    let x1 = (x1t as usize).min(row.len() - 1);
    if x0 > x1 { return; }
    if mix <= 0.0 {
        row[x0..=x1].fill(a);
    } else {
        for (x, cell) in row[x0..=x1].iter_mut().enumerate() {
            *cell = if hash2((x0 + x) as i32, y as i32, salt) < mix { b } else { a };
        }
    }
}

pub(crate) fn draw_opus_1_quasicrystal(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Opus1QuasicrystalKnobs,
) {
    let n = choose_sym(seed, k.sym).max(3);
    CACHE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let stale = slot.as_ref().map(|c| c.key != (seed, n)).unwrap_or(true);
        if stale {
            *slot = Some(build(seed, n));
        }
        render(grid, w, h, palette, t, k, slot.as_mut().unwrap());
    });
}

fn render(grid: &mut Grid, w: usize, h: usize, palette: &[Color; 5], t: f32, k: &Opus1QuasicrystalKnobs, c: &mut Cached) {
    let bg = palette[0];
    measure_layer("opus-1-quasicrystal", "clear", || {
        let blank = Cell::with_bg(' ', palette[4], bg);
        for row in grid.iter_mut().take(h) {
            let end = w.min(row.len());
            row[..end].fill(blank);
        }
    });
    if w < 8 || h < 5 {
        return;
    }

    let tt = if t > 0.0 { t * k.speed.clamp(0.0, 8.0) } else { 0.0 };
    let n = c.n;
    let nf = n as f32;
    let breath = k.breath.clamp(0.0, 0.35);
    let zoom = 1.0 + breath * 0.5 * (1.0 - (2.0 * PI * tt / BREATH_PERIOD).cos());
    let mut sx = k.scale.clamp(5.0, 80.0) * zoom;
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;

    let mut rd = ((cx / sx).powi(2) + (cy / (sx * 0.5)).powi(2)).sqrt() + 2.0;
    if 4.0 * rd * rd > MAX_RHOMBS as f32 {
        sx *= (4.0 * rd * rd / MAX_RHOMBS as f32).sqrt();
        rd = ((cx / sx).powi(2) + (cy / (sx * 0.5)).powi(2)).sqrt() + 2.0;
    }
    let sy = sx * 0.5;
    let rp = 2.0 * rd / nf + 2.0;

    let rot = c.rot0 + (k.spin.clamp(0.0, 24.0) * tt).to_radians();
    let drift = k.drift.clamp(0.0, 0.4);
    let mut gsx = 0.0f32;
    let mut gsy = 0.0f32;
    let mut fpx = 0.0f32;
    let mut fpy = 0.0f32;
    for m in 0..n {
        let a = PI * m as f32 / nf + rot;
        let (ca, sa) = (a.cos(), a.sin());
        c.eu[m] = (ca, sa);
        c.ev[m] = (ca * sx, sa * sy);
        c.glyph[m] = slope_glyph(ca * sx, sa * sy);
        let g = c.gamma0[m] + c.grate[m] * drift * tt;
        c.gam[m] = g;
        gsx += g * ca;
        gsy += g * sa;
        fpx += g * c.fv[m].0;
        fpy += g * c.fv[m].1;
        c.lit[m] = i32::MIN;
    }

    let worms = (k.worms.round().clamp(0.0, 6.0)) as usize;
    let epoch = (tt / WORM_LIFE).floor() as i64;
    for wi in 0..worms.min(n) {
        let idx = (epoch.rem_euclid(WORM_SEQ as i64) as usize + wi * 11) % WORM_SEQ;
        let (fam, at, ph) = c.seq[idx];
        let m = fam as usize % n;
        c.lit[m] = (at * rp * 0.75 - c.gam[m]).round() as i32;
        let travel = (tt * k.pulse.clamp(0.0, 2.0) + ph).fract();
        c.head[m] = (travel * 2.0 - 1.0) * rp;
    }

    let nclass = (n / 2).max(1).min(CLASS_MAX);
    let hue_base = c.hue0 + k.hue;
    let mut fill_a = [[Cell::blank(); LEVELS]; CLASS_MAX];
    let mut fill_b = [[Cell::blank(); LEVELS]; CLASS_MAX];
    let mut edge_fg = [palette[4]; CLASS_MAX];
    for ci in 0..nclass {
        let hue = (hue_base + 34.0 * ci as f32).rem_euclid(360.0) as f64;
        edge_fg[ci] = lerp_color(hsl_to_rgb(hue, 0.5, 0.8), palette[4], 0.42);
        for lv in 0..LEVELS {
            let f = lv as f32 / (LEVELS - 1) as f32;
            let fg = lerp_color(hsl_to_rgb(hue, 0.5, (0.2 + 0.42 * f) as f64), palette[1], 0.22);
            let tint = lerp_color(hsl_to_rgb(hue, 0.55, (0.04 + 0.05 * f) as f64), bg, 0.42);
            fill_a[ci][lv] = Cell::with_bg(RAMP[lv], fg, tint);
            let lo = lv.saturating_sub(1);
            fill_b[ci][lv] = Cell::with_bg(RAMP[lo], fg, tint);
        }
    }
    let star_fg = lighten(palette[3], 60);
    let worm_fg = lighten(palette[3], 30);
    let worm_hot = lerp_color(palette[3], palette[4], 0.65);

    let wave = k.wave.clamp(0.6, 60.0);
    let wk = 2.0 * PI / wave;
    let (wcx, wcy) = ((c.wave_ang + rot).cos(), (c.wave_ang + rot).sin());
    let wphase = 2.0 * PI * tt / WAVE_PERIOD;
    let shade = k.shade.clamp(0.0, 1.0);
    let star_r = k.stars.clamp(0.0, 1.0) * 1.25;
    let pulse_w = (rp * 0.22).max(0.5);
    let core_k = 1.0 / (rd * 0.3).powi(2).max(0.5);
    let vig_x = 1.0 / (cx * cx).max(1.0);
    let vig_y = 1.0 / (cy * cy).max(1.0);

    measure_layer("opus-1-quasicrystal", "solve", || {
        c.rhombs.clear();
        for j in 0..n {
            for l in (j + 1)..n {
                let det = c.eu[j].0 * c.eu[l].1 - c.eu[j].1 * c.eu[l].0;
                if det.abs() < 5e-3 {
                    continue;
                }
                let dot = c.eu[j].0 * c.eu[l].0 + c.eu[j].1 * c.eu[l].1;
                let d = (l - j).min(n - (l - j));
                let class = (d - 1).min(nclass - 1) as u8;
                let kj_lo = (-rp - c.gam[j]).ceil() as i32;
                let kj_hi = (rp - c.gam[j]).floor() as i32;
                for kj in kj_lo..=kj_hi {
                    let cj = kj as f32 + c.gam[j];
                    let rad2 = rp * rp - cj * cj;
                    if rad2 <= 0.0 {
                        continue;
                    }
                    let smax = rad2.sqrt();
                    let span = (smax * det).abs();
                    let mid = cj * dot;
                    let kl_lo = (mid - span - c.gam[l]).ceil() as i32;
                    let kl_hi = (mid + span - c.gam[l]).floor() as i32;
                    for kl in kl_lo..=kl_hi {
                        if c.rhombs.len() >= MAX_RHOMBS {
                            return;
                        }
                        let cl = kl as f32 + c.gam[l];
                        let px = (cj * c.eu[l].1 - cl * c.eu[j].1) / det;
                        let py = (c.eu[j].0 * cl - c.eu[l].0 * cj) / det;
                        let mut vx = gsx;
                        let mut vy = gsy;
                        let mut qx = fpx;
                        let mut qy = fpy;
                        for m in 0..n {
                            let km = if m == j {
                                kj
                            } else if m == l {
                                kl
                            } else {
                                (px * c.eu[m].0 + py * c.eu[m].1 - c.gam[m]).ceil() as i32
                            } as f32;
                            vx += km * c.eu[m].0;
                            vy += km * c.eu[m].1;
                            qx += km * c.fv[m].0;
                            qy += km * c.fv[m].1;
                        }
                        let ax = cx + vx * sx;
                        let ay = cy + vy * sy;
                        let (ux, uy) = c.ev[j];
                        let (wx, wy) = c.ev[l];
                        let lox = ax + ux.min(0.0) + wx.min(0.0);
                        let hix = ax + ux.max(0.0) + wx.max(0.0);
                        let loy = ay + uy.min(0.0) + wy.min(0.0);
                        let hiy = ay + uy.max(0.0) + wy.max(0.0);
                        if hix < 0.0 || hiy < 0.0 || lox >= w as f32 || loy >= h as f32 {
                            continue;
                        }
                        let mcx = vx + (c.eu[j].0 + c.eu[l].0) * 0.5;
                        let mcy = vy + (c.eu[j].1 + c.eu[l].1) * 0.5;
                        let rr = (mcx * mcx + mcy * mcy).sqrt() + 0.36 * (mcx * wcx + mcy * wcy);
                        let lightw = 0.5 + 0.5 * (rr * wk - wphase).cos();
                        let ddx = ax + (ux + wx) * 0.5 - cx;
                        let ddy = ay + (uy + wy) * 0.5 - cy;
                        let vig = (1.0 - 0.66 * (ddx * ddx * vig_x + ddy * ddy * vig_y)).clamp(0.0, 1.0);
                        let bias = 0.34 * (class as f32 / (nclass - 1).max(1) as f32) - 0.17;
                        let grain = (qx * 1.618 + qy * 2.414).rem_euclid(1.0) - 0.5;
                        let mass = lightw * lightw * (3.0 - 2.0 * lightw);
                        let core = (-(mcx * mcx + mcy * mcy) * core_k).exp();
                        let amt = (((0.96 * mass + 0.44 * grain + bias * mass) * vig + 0.78 * core) * shade)
                            .clamp(0.0, 1.0);
                        let level = (amt * (LEVELS - 1) as f32).round() as u8;
                        let mut star = 0u8;
                        let corners = [
                            (qx, qy),
                            (qx + c.fv[j].0, qy + c.fv[j].1),
                            (qx + c.fv[j].0 + c.fv[l].0, qy + c.fv[j].1 + c.fv[l].1),
                            (qx + c.fv[l].0, qy + c.fv[l].1),
                        ];
                        for (bit, p) in corners.iter().enumerate() {
                            if p.0 * p.0 + p.1 * p.1 < star_r * star_r {
                                star |= 1 << bit;
                            }
                        }
                        let mut worm = 0.0f32;
                        if c.lit[j] == kj || c.lit[l] == kl {
                            let m = if c.lit[j] == kj { j } else { l };
                            let s = -px * c.eu[m].1 + py * c.eu[m].0;
                            let g = ((s - c.head[m]) / pulse_w).abs().min(6.0);
                            worm = 0.34 + 0.66 * (-g * g).exp();
                        }
                        c.rhombs.push(Rhomb {
                            ax,
                            ay,
                            j: j as u8,
                            l: l as u8,
                            level,
                            class,
                            worm,
                            star,
                        });
                    }
                }
            }
        }
    });

    let ev = &c.ev;
    let glyph = &c.glyph;
    let rhombs = &c.rhombs;

    measure_layer("opus-1-quasicrystal", "shade", || {
        if w.saturating_mul(h) < 100_000 {
            for r in rhombs.iter() {
                let (ci, lv) = (r.class as usize, r.level as usize);
                fill_par(grid, w, h, r.ax, r.ay, ev[r.j as usize], ev[r.l as usize],
                    fill_a[ci][lv], fill_b[ci][lv], if lv >= 5 { 0.3 } else { 0.0 },
                    (lv as u32) << 3 | ci as u32);
            }
            return;
        }
        c.shade_rows.resize_with(h, Vec::new);
        for bin in &mut c.shade_rows { bin.clear(); }
        for (ri, r) in rhombs.iter().enumerate() {
            let u = ev[r.j as usize];
            let v = ev[r.l as usize];
            let ymin = r.ay.min(r.ay + u.1).min(r.ay + v.1).min(r.ay + u.1 + v.1);
            let ymax = r.ay.max(r.ay + u.1).max(r.ay + v.1).max(r.ay + u.1 + v.1);
            let y0 = (ymin - 0.5).ceil().max(0.0) as usize;
            let y1t = (ymax - 0.5).floor();
            if y1t < 0.0 || y0 >= h { continue; }
            let y1 = (y1t as usize).min(h - 1);
            if y0 > y1 { continue; }
            for bin in &mut c.shade_rows[y0..=y1] { bin.push(ri); }
        }
        grid[..h].par_iter_mut().zip(c.shade_rows[..h].par_iter()).enumerate()
            .for_each(|(y, (row, bin))| {
                for &ri in bin {
                    let r = &rhombs[ri];
                    let (ci, lv) = (r.class as usize, r.level as usize);
                    fill_par_row(&mut row[..w], y, r.ax, r.ay, ev[r.j as usize], ev[r.l as usize],
                        fill_a[ci][lv], fill_b[ci][lv], if lv >= 5 { 0.3 } else { 0.0 },
                        (lv as u32) << 3 | ci as u32);
                }
            });
    });

    measure_layer("opus-1-quasicrystal", "edges", || {
        for r in rhombs.iter() {
            let (j, l) = (r.j as usize, r.l as usize);
            let fg = edge_fg[r.class as usize];
            let (ux, uy) = ev[j];
            let (wx, wy) = ev[l];
            stroke(grid, w, h, r.ax, r.ay, ev[j], glyph[j], fg);
            stroke(grid, w, h, r.ax, r.ay, ev[l], glyph[l], fg);
            stroke(grid, w, h, r.ax + wx, r.ay + wy, ev[j], glyph[j], fg);
            stroke(grid, w, h, r.ax + ux, r.ay + uy, ev[l], glyph[l], fg);
        }
    });

    measure_layer("opus-1-quasicrystal", "worms", || {
        for r in rhombs.iter().filter(|r| r.worm > 0.0) {
            let (j, l) = (r.j as usize, r.l as usize);
            let heat = ((r.worm - 0.34) / 0.66).clamp(0.0, 1.0);
            let tint = lerp_color(bg, palette[3], 0.08 + 0.3 * heat);
            if heat > 0.3 {
                let fg = lerp_color(worm_fg, worm_hot, heat);
                let a = Cell::with_bg(if heat > 0.7 { '#' } else { '~' }, fg, tint);
                let b = Cell::with_bg(if heat > 0.7 { '%' } else { ',' }, darken(fg, 40), tint);
                fill_par(grid, w, h, r.ax, r.ay, ev[j], ev[l], a, b, 0.4, 0x5A);
            }
            let edge = lerp_color(worm_fg, worm_hot, heat);
            let (ux, uy) = ev[j];
            let (wx, wy) = ev[l];
            stroke(grid, w, h, r.ax, r.ay, ev[j], glyph[j], edge);
            stroke(grid, w, h, r.ax, r.ay, ev[l], glyph[l], edge);
            stroke(grid, w, h, r.ax + wx, r.ay + wy, ev[j], glyph[j], edge);
            stroke(grid, w, h, r.ax + ux, r.ay + uy, ev[l], glyph[l], edge);
        }
    });

    measure_layer("opus-1-quasicrystal", "stars", || {
        for r in rhombs.iter().filter(|r| r.star != 0) {
            let (ux, uy) = ev[r.j as usize];
            let (wx, wy) = ev[r.l as usize];
            let pts = [
                (r.ax, r.ay),
                (r.ax + ux, r.ay + uy),
                (r.ax + ux + wx, r.ay + uy + wy),
                (r.ax + wx, r.ay + wy),
            ];
            for (bit, p) in pts.iter().enumerate() {
                if r.star & (1 << bit) == 0 || p.0 < 0.0 || p.1 < 0.0 {
                    continue;
                }
                let (x, y) = (p.0 as usize, p.1 as usize);
                if x < w && y < h {
                    let keep = grid[y][x].bg;
                    let ch = if r.level >= 4 { '*' } else { 'o' };
                    grid[y][x] = Cell::with_bg(ch, star_fg, keep);
                }
            }
        }
    });
}

pub(crate) fn cli_opus_1_quasicrystal(
    mut grid: Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: [Color; 5],
    rng: StdRng,
    t_anim: f32,
    term_w: u16,
    term_h: u16,
    args: &[String],
    mode: &str,
    theme_name: &str,
) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = Opus1QuasicrystalKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 12] = [
        &mut k.speed,
        &mut k.hue,
        &mut k.sym,
        &mut k.scale,
        &mut k.drift,
        &mut k.spin,
        &mut k.shade,
        &mut k.wave,
        &mut k.worms,
        &mut k.pulse,
        &mut k.stars,
        &mut k.breath,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_opus_1_quasicrystal(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = Opus1QuasicrystalKnobs::from_env();
        draw_opus_1_quasicrystal(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_quasicrystal_small() {
        insta::assert_snapshot!("opus_1_quasicrystal_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_quasicrystal_wide_t18() {
        insta::assert_snapshot!("opus_1_quasicrystal_110x36_t18", run(110, 36, 7, 18.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 3, 0.0));
    }

    #[test]
    fn time_moves_the_tiling() {
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 9.0));
        assert_ne!(run(90, 30, 42, 9.0), run(90, 30, 42, 21.0));
    }

    #[test]
    fn every_symmetry_terminates() {
        for n in 3..=13 {
            let mut g = vec![vec![Cell::blank(); 60]; 20];
            let p = crate::color::make_palette(n as u64);
            let mut k = Opus1QuasicrystalKnobs::from_env();
            k.sym = n as f32;
            draw_opus_1_quasicrystal(&mut g, 60, 20, 5, &p, 4.0, &k);
        }
    }

    #[test]
    fn tiny_grid_is_safe() {
        let mut g = vec![vec![Cell::blank(); 4]; 3];
        let p = crate::color::make_palette(1);
        let k = Opus1QuasicrystalKnobs::from_env();
        draw_opus_1_quasicrystal(&mut g, 4, 3, 1, &p, 2.0, &k);
    }

    #[test]
    fn row_fill_matches_serial_with_clipping_dither_and_overwrite() {
        let (w, h) = (37, 19);
        let mut serial = vec![vec![Cell::blank(); w]; h];
        let mut rows = serial.clone();
        let fills = [
            (-5.4, 2.2, (13.7, 4.3), (-3.1, 8.8), Cell::with_bg('#', Color::Red, Color::Blue), Cell::with_bg('%', Color::Yellow, Color::Blue), 0.3, 43),
            (11.8, -4.5, (8.2, 16.4), (14.7, 2.1), Cell::with_bg('x', Color::Green, Color::Black), Cell::with_bg('o', Color::Cyan, Color::Black), 0.0, 7),
            (20.1, 8.7, (-12.5, 6.4), (9.3, 7.8), Cell::with_bg('@', Color::Magenta, Color::DarkGrey), Cell::with_bg('.', Color::White, Color::DarkGrey), 0.3, 61),
        ];
        for &(ax, ay, u, v, a, b, mix, salt) in &fills {
            fill_par(&mut serial, w, h, ax, ay, u, v, a, b, mix, salt);
            for (y, row) in rows.iter_mut().enumerate() {
                fill_par_row(row, y, ax, ay, u, v, a, b, mix, salt);
            }
        }
        assert_eq!(rows, serial);
    }

    #[test]
    fn parallel_shading_preserves_full_colored_grid() {
        let render = |threads| rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap().install(|| {
            let (w, h) = (400, 260);
            let mut g = vec![vec![Cell::blank(); w]; h];
            let p = crate::color::make_palette(73);
            let mut k = Opus1QuasicrystalKnobs::from_env();
            k.sym = 13.0; k.shade = 1.0; k.stars = 0.83;
            draw_opus_1_quasicrystal(&mut g, w, h, 73, &p, 19.75, &k);
            g
        });
        assert_eq!(render(1), render(4));
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = Opus1QuasicrystalKnobs::from_env();
        draw_opus_1_quasicrystal(&mut g, w, h, 42, &p, 0.0, &k);
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..200 {
            let t0 = std::time::Instant::now();
            draw_opus_1_quasicrystal(&mut g, w, h, 42, &p, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("opus-1-quasicrystal frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }
}
