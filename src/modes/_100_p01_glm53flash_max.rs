//! fairy-ring: a mycelial colony lives one cycle per period: spores drift,
//! hyphae colonize the soil by space colonization, mushrooms rise on the ring.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};

pub(super) struct FairyRing;
pub(super) static MODE: FairyRing = FairyRing;

const NAME: &str = "fairy-ring";
const KNOBS: usize = 11;
const HELP: &str = "fairy-ring: a mycelial colony lives its cycle on a soil cross-section [period] [frontier] [squash] [colony] [weave] [flow] [feed] [ground] [hue] [glow] [aspect]";

const PARAMS: &[Param] = &[
    param!("PERIOD", "life cycle seconds", 8.0, 90.0, 36.0, 1.0),
    param!("FRONTIER", "ring radius", 0.3, 1.0, 0.78, 0.02),
    param!("SQUASH", "ring perspective squash", 0.1, 0.6, 0.28, 0.02),
    param!("COLONY", "mushrooms on the ring", 4.0, 28.0, 16.0, 1.0),
    param!("WEAVE", "hyphal curl", 0.0, 1.5, 0.6, 0.05),
    param!("FLOW", "nutrient pulse speed", 0.0, 3.0, 1.0, 0.05),
    param!("FEED", "soil richness", 0.2, 2.0, 1.0, 0.05),
    param!("GROUND", "horizon height", 0.45, 0.75, 0.62, 0.01),
    param!("HUE", "cap hue deg", 0.0, 360.0, 30.0, 5.0),
    param!("GLOW", "exposure", 0.5, 1.6, 1.0, 0.05),
    param!("ASPECT", "cols per row", 0.25, 4.0, 2.0, 0.25),
];

const L_CAP: u64 = 0x41;
const L_SOIL: u64 = 0x42;
const L_NET: u64 = 0x43;
const L_SPORE: u64 = 0x44;
const L_THEME: u64 = 0x45;

const CORD_RAMP: [char; 5] = ['.', ':', '-', '+', '*'];
const LITTER: [char; 4] = ['~', '.', ',', ';'];
const STEW: usize = 420;

thread_local! {
    static FIELD: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static NET: RefCell<Vec<Seg>> = const { RefCell::new(Vec::new()) };
}

impl Mode for FairyRing {
    fn name(&self) -> &'static str {
        NAME
    }
    fn help(&self) -> &'static str {
        HELP
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        let p: [f32; KNOBS] = std::array::from_fn(|i| {
            let param = &PARAMS[i];
            let value = frame
                .args
                .get(i + 4)
                .and_then(|v| v.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(param.key, param.default));
            if value.is_finite() {
                value.clamp(param.min, param.max)
            } else {
                param.default
            }
        });
        draw(frame, &p);
    }
}

#[inline]
fn hash(seed: u64, layer: u64, index: u64, slot: u64) -> u64 {
    let mut z = seed
        ^ layer.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ index.wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ slot.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
fn unit(h: u64) -> f32 {
    (h >> 40) as f32 * (1.0 / 16_777_216.0)
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn noise2(seed: u64, x: f32, y: f32) -> f32 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (smoothstep(x - ix), smoothstep(y - iy));
    let (ix, iy) = (ix as i64 as u64, iy as i64 as u64);
    let s = |dx: u64, dy: u64| unit(hash(seed, L_SOIL, ix.wrapping_add(dx), iy.wrapping_add(dy)));
    let a = s(0, 0) + (s(1, 0) - s(0, 0)) * fx;
    let b = s(0, 1) + (s(1, 1) - s(0, 1)) * fx;
    (a + (b - a) * fy) * 2.0 - 1.0
}

fn put(grid: &mut Grid, w: usize, h: usize, x: f32, y: f32, ch: char, col: Color) {
    let (xi, yi) = (x.round() as isize, y.round() as isize);
    if xi < 0 || yi < 0 {
        return;
    }
    let (xu, yu) = (xi as usize, yi as usize);
    if xu >= w || yu >= h {
        return;
    }
    grid[yu][xu] = Cell::new(ch, col);
}

fn dim(c: Color, depth: f32, bg: Color) -> Color {
    lerp_color(c, bg, (depth * 0.55).clamp(0.0, 0.8))
}

/// One hyphal step in world row-units (U right, V down from the horizon).
struct Seg {
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    ord: f32,
    ph: f32,
}

/// One mushroom slot on the ring, sorted back to front for occlusion.
struct Cap {
    bx: f32,
    by: f32,
    bx0: f32,
    by0: f32,
    depth: f32,
    sprout: f32,
    rich: f32,
    lean: f32,
}

struct Look {
    seed: u64,
    w: usize,
    h: usize,
    aspect: f32,
    cx: f32,
    gy: f32,
    rmax: f32,
    rg: f32,
    g: f32,
    time: f32,
    weave: f32,
    flow: f32,
    feed: f32,
    glow: f32,
    colony: f32,
    scale: f32,
    squash: f32,
    bg: Color,
    cap_fill: Color,
    cap_rim: Color,
    cap_gill: Color,
    stem_col: Color,
    cord_col: Color,
    cord_hot: Color,
    tip_col: Color,
    soil_col: Color,
    soil_deep: Color,
    litter_col: Color,
    spore_col: Color,
    shim_col: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let aspect = p[10].max(0.25);
        let gy = (p[7] * h as f32).clamp(2.0, (h as f32 - 3.0).max(2.0));
        let squash = p[2].clamp(0.08, 0.6);
        let fit_w = 0.47 * w as f32 / aspect;
        let fit_front = (h as f32 - gy - 3.0) / squash;
        let fit_back = (gy - 1.5) / squash;
        let rmax = p[1] * fit_w.min(fit_front).min(fit_back).max(2.5);
        let g = (time / p[0].max(1.0)).rem_euclid(1.0);
        let rg = rmax * smoothstep((g - 0.10) / 0.38);
        let base = unit(hash(seed, L_THEME, 0, 0)) * 360.0;
        let hue = (base + p[8]) % 360.0;
        let gk = p[9] as f64;
        let cap_fill = hsl_to_rgb((hue + 6.0) as f64, 0.50, (0.40 * gk).clamp(0.08, 0.85));
        let cap_rim = hsl_to_rgb((hue + 20.0) as f64, 0.62, (0.58 * gk).clamp(0.10, 0.90));
        let stem_col = hsl_to_rgb((hue + 45.0) as f64, 0.20, (0.50 * gk).clamp(0.08, 0.80));
        let cord_col = hsl_to_rgb((hue + 150.0) as f64, 0.32, (0.40 * gk).clamp(0.06, 0.80));
        let cord_hot = hsl_to_rgb((hue + 120.0) as f64, 0.68, (0.62 * gk).clamp(0.10, 0.90));
        let soil_col = hsl_to_rgb((hue + 35.0) as f64, 0.18, (0.24 * gk).clamp(0.05, 0.60));
        let spore_col = lerp_color(
            hsl_to_rgb((hue + 70.0) as f64, 0.45, (0.70 * gk).clamp(0.10, 0.90)),
            palette[4],
            0.35,
        );
        Look {
            seed,
            w,
            h,
            aspect,
            cx: w as f32 * 0.5,
            gy,
            rmax: rmax.max(2.5),
            rg,
            g,
            time,
            weave: p[4],
            flow: p[5],
            feed: p[6],
            colony: p[3],
            scale: (h as f32 / 24.0).clamp(0.6, 1.2),
            glow: p[9],
            squash,
            bg: palette[0],
            cap_fill,
            cap_rim,
            cap_gill: darken(cap_rim, 42),
            stem_col,
            cord_col,
            cord_hot,
            tip_col: lighten(palette[4], 20),
            soil_col,
            soil_deep: darken(soil_col, 25),
            litter_col: lighten(soil_col, 22),
            spore_col,
            shim_col: darken(cord_col, 18),
        }
    }
}
fn ring_caps(look: &Look) -> Vec<Cap> {
    let mut count = (look.colony.round() as usize).clamp(4, 28);
    let a = look.rmax * look.aspect;
    let b = (look.rmax * look.squash).max(1.0);
    let perim = std::f32::consts::PI * (3.0 * (a + b) - ((3.0 * a + b) * (a + 3.0 * b)).sqrt());
    count = count.min(((perim / 6.0) as usize).max(4));
    let rot = unit(hash(look.seed, L_CAP, 0, 9)) * TAU;
    let mut caps = Vec::with_capacity(count);
    for i in 0..count {
        let j = unit(hash(look.seed, L_CAP, i as u64, 0));
        let th = TAU * ((i as f32 + 0.5 * j - 0.25) / count as f32) + rot;
        let (c, s) = (th.cos(), th.sin());
        let sprout = smoothstep((look.g - 0.34 - 0.30 * unit(hash(look.seed, L_CAP, i as u64, 1))) / 0.20)
            * (1.0 - smoothstep((look.g - 0.94) / 0.05));
        caps.push(Cap {
            bx: look.cx + look.rg * c * look.aspect,
            by: look.gy + look.rg * s * look.squash,
            bx0: look.cx + look.rmax * c * look.aspect,
            by0: look.gy + look.rmax * s * look.squash,
            depth: (1.0 - s) * 0.5,
            sprout,
            rich: 0.0,
            lean: (unit(hash(look.seed, L_CAP, i as u64, 2)) - 0.5) * 1.2,
        });
    }
    caps.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));
    caps
}

fn build_field(field: &mut [f32], fw: usize, fh: usize, look: &Look, caps: &mut [Cap]) {
    for fy in 0..fh {
        let v = (fy as f32 + 0.5) * 4.0 - look.gy;
        for fx in 0..fw {
            let u = ((fx as f32 + 0.5) * 4.0 - look.cx) / look.aspect;
            let r = (u * u + (v * 1.12) * (v * 1.12)).sqrt();
            let band = look.rg * 0.18 + 0.5;
            let t = smoothstep((r - look.rg + band) / (2.0 * band));
            let inn = 0.55 - 0.42 * smoothstep((look.rg * 0.9 - r) / (look.rg * 0.9 + 1.0));
            let out = 1.0 - 0.85 * smoothstep((r - look.rg) / (look.rmax * 0.5 + 2.0));
            let mut n = (inn + (out - inn) * t) * look.feed;
            n += 0.28 * noise2(look.seed, u * 0.75 + 9.0, v * 0.75 + 4.0) * look.feed;
            n *= smoothstep((v + 0.8) / 1.2);
            field[fy * fw + fx] = n.max(0.0);
        }
    }
    // Caps read the pre-consumption field, then drain it where they feed.
    for cap in caps.iter_mut() {
        let u = (cap.bx - look.cx) / look.aspect;
        let v = cap.by - look.gy;
        let pre = field_at(field, fw, fh, u, v, look);
        cap.rich = pre.clamp(0.0, 1.0);
        if cap.sprout > 0.15 {
            let rad = 2.0 + 2.4 * cap.rich;
            let amp = 0.85 * cap.sprout * cap.rich;
            let cfx = (u * look.aspect + look.cx) / 4.0;
            let cfy = (v + look.gy) / 4.0;
            let lo = ((cfy - rad).max(0.0) as usize).min(fh.saturating_sub(1));
            let hi = (((cfy + rad) as usize) + 1).min(fh);
            for fy in lo..hi {
                for fx in 0..fw {
                    let du = fx as f32 - cfx;
                    let dv = fy as f32 - cfy;
                    let d2 = du * du + dv * dv;
                    let fall = (-d2 / (rad * rad * 0.5)).exp();
                    let idx = fy * fw + fx;
                    field[idx] = (field[idx] - amp * fall).max(0.0);
                }
            }
        }
    }
}

fn field_at(field: &[f32], fw: usize, fh: usize, u: f32, v: f32, look: &Look) -> f32 {
    let gx = (u * look.aspect + look.cx) / 4.0 - 0.5;
    let gy = (v + look.gy) / 4.0 - 0.5;
    let (x0, y0) = (gx.floor(), gy.floor());
    let (tx, ty) = (smoothstep(gx - x0), smoothstep(gy - y0));
    let (ix, iy) = (x0 as isize, y0 as isize);
    let s = |dx: isize, dy: isize| -> f32 {
        let (x, y) = (ix + dx, iy + dy);
        if x < 0 || y < 0 || x >= fw as isize || y >= fh as isize {
            0.0
        } else {
            field[y as usize * fw + x as usize]
        }
    };
    let a = s(0, 0) + (s(1, 0) - s(0, 0)) * tx;
    let b = s(0, 1) + (s(1, 1) - s(0, 1)) * tx;
    a + (b - a) * ty
}

struct Tip {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    steps: usize,
    ph: f32,
    open: bool,
}

/// Space colonization in the soil half-disc: tips reach for hashed
/// attractors, bend toward richer field, and branch when they feed.
fn grow_network(net: &mut Vec<Seg>, field: &[f32], fw: usize, fh: usize, look: &Look) -> Vec<(f32, f32)> {
    net.clear();
    if !(0.10..0.62).contains(&look.g) || look.rg < 2.0 {
        return Vec::new();
    }
    let na = ((70.0 + 260.0 * look.rg / look.rmax) as usize).min(320);
    let mut ax: Vec<(f32, f32, bool)> = Vec::with_capacity(na);
    for i in 0..na {
        let u2 = unit(hash(look.seed, L_NET, i as u64, 1));
        let u3 = unit(hash(look.seed, L_NET, i as u64, 2));
        let r = look.rg * (0.32 + 0.78 * u2.sqrt());
        let phi = PI * (0.07 + 0.86 * u3);
        ax.push((r * phi.cos(), 0.5 + r * phi.sin() * 0.88, false));
    }
    let mut tips: Vec<Tip> = Vec::with_capacity(20);
    for j in 0..9usize {
        let psi = PI * (0.26 + 0.075 * j as f32 + 0.05 * unit(hash(look.seed, L_NET, j as u64, 3)));
        tips.push(Tip {
            x: 0.0,
            y: 1.0,
            dx: psi.cos(),
            dy: psi.sin(),
            steps: 0,
            ph: unit(hash(look.seed, L_NET, j as u64, 4)),
            open: true,
        });
    }
    const REACH: f32 = 7.0;
    const STEP: f32 = 1.7;
    let vb = (look.h as f32 - look.gy) - 0.8;
    let ub = (look.w as f32 * 0.5) / look.aspect - 0.8;
    let mut iter = 0usize;
    while iter < STEW && net.len() < STEW {
        let mut progressed = false;
        for ti in 0..tips.len() {
            if !tips[ti].open {
                continue;
            }
            let (mut best, mut bd) = (usize::MAX, REACH * REACH);
            for (ai, a) in ax.iter().enumerate() {
                if a.2 {
                    continue;
                }
                let du = a.0 - tips[ti].x;
                let dv = a.1 - tips[ti].y;
                let d2 = du * du + dv * dv;
                if d2 < bd {
                    bd = d2;
                    best = ai;
                }
            }
            if best == usize::MAX {
                tips[ti].open = false;
                continue;
            }
            let (tx, ty) = (ax[best].0 - tips[ti].x, ax[best].1 - tips[ti].y);
            let tl = (tx * tx + ty * ty).sqrt().max(1e-4);
            let (mut dx, mut dy) = (tx / tl, ty / tl);
            let e = 0.7;
            let (px, py) = (tips[ti].x, tips[ti].y);
            let gx = field_at(field, fw, fh, px + e, py, look)
                - field_at(field, fw, fh, px - e, py, look);
            let gv = field_at(field, fw, fh, px, py + e, look)
                - field_at(field, fw, fh, px, py - e, look);
            dx += gx * 0.5;
            dy += gv * 0.5;
            let ang = look.weave * 0.85 * (tips[ti].ph * TAU + (tips[ti].steps as f32) * 0.55).sin();
            let (ca, sa) = (ang.cos(), ang.sin());
            let (rx, ry) = (dx * ca - dy * sa, dx * sa + dy * ca);
            let rl = (rx * rx + ry * ry).sqrt().max(1e-4);
            let (mx, my) = (px + rx / rl * STEP, py + ry / rl * STEP);
            if my > vb || my < 0.3 || mx.abs() > ub {
                tips[ti].open = false;
                continue;
            }
            net.push(Seg {
                ax: px,
                ay: py,
                bx: mx,
                by: my,
                ord: net.len() as f32,
                ph: tips[ti].ph,
            });
            tips[ti].x = mx;
            tips[ti].y = my;
            tips[ti].steps += 1;
            progressed = true;
            if bd < 1.25 * 1.25 {
                ax[best].2 = true;
                tips[ti].open = false;
                let branch = net.len() < STEW
                    && tips.len() < 20
                    && unit(hash(look.seed, L_NET, net.len() as u64, 5)) < 0.8;
                if branch {
                    let side = if unit(hash(look.seed, L_NET, net.len() as u64, 6)) < 0.5 {
                        1.0
                    } else {
                        -1.0
                    };
                    let ba = side * (0.5 + 0.6 * unit(hash(look.seed, L_NET, net.len() as u64, 7)));
                    let (cb, sb) = (ba.cos(), ba.sin());
                    tips.push(Tip {
                        x: mx,
                        y: my,
                        dx: rx / rl * cb - ry / rl * sb,
                        dy: rx / rl * sb + ry / rl * cb,
                        steps: 0,
                        ph: unit(hash(look.seed, L_NET, net.len() as u64, 8)),
                        open: true,
                    });
                }
            }
            if tips[ti].steps > 22 {
                tips[ti].open = false;
            }
            if net.len() >= STEW {
                break;
            }
        }
        if !progressed {
            break;
        }
        iter += 1;
    }
    tips
        .iter()
        .filter(|t| t.open)
        .map(|t| (t.x, t.y))
        .collect()
}

fn paint_soil(grid: &mut Grid, look: &Look) {
    let (w, h) = (look.w, look.h);
    let hor = look.gy.round() as usize;
    if hor < h {
        for x in 0..w {
            if unit(hash(look.seed, L_SOIL, x as u64, 11)) < 0.5 {
                let col = dim(look.soil_col, 0.0, look.bg);
                grid[hor][x] = Cell::new('-', col);
            }
        }
    }
    for y in (hor + 1)..h {
        let t = y as f32 - look.gy;
        for x in 0..w {
            let r = unit(hash(look.seed, L_SOIL, (x * 7 + y) as u64, 0));
            let p = if t < 2.5 { 0.16 } else { 0.05 };
            if r < p {
                let glyph = LITTER[(r * 97.0) as usize & 3];
                let col = dim(look.litter_col, (t * 0.06).min(0.5), look.bg);
                grid[y][x] = Cell::new(glyph, col);
            } else if t >= 2.5 && unit(hash(look.seed, L_SOIL, (x * 13 + y) as u64, 1)) < 0.05 {
                let col = dim(look.soil_deep, (t * 0.05).min(0.6), look.bg);
                grid[y][x] = Cell::new('.', col);
            }
        }
    }
}

fn paint_hyphae(
    grid: &mut Grid,
    net: &[Seg],
    tips: &[(f32, f32)],
    field: &[f32],
    fw: usize,
    fh: usize,
    look: &Look,
) {
    let total = net.len() as f32;
    for seg in net {
        let ord_n = if total > 0.0 { seg.ord / total } else { 0.0 };
        let born = 0.10 + 0.42 * ord_n;
        let env = smoothstep((look.g - born) / 0.05);
        if env < 0.04 {
            continue;
        }
        let (ax, ay) = (look.cx + seg.ax * look.aspect, look.gy + seg.ay);
        let (bx, by) = (look.cx + seg.bx * look.aspect, look.gy + seg.by);
        let steps = ((ax - bx).abs().max((ay - by).abs()) as usize + 1).clamp(1, 6);
        for k in 0..=steps {
            let f = k as f32 / steps as f32;
            let (x, y) = (ax + (bx - ax) * f, ay + (by - ay) * f);
            let wave =
                0.5 + 0.5 * (TAU * (ord_n * 2.0) - look.time * look.flow * 2.4 + seg.ph * TAU).sin();
            let flux = field_at(
                field,
                fw,
                fh,
                seg.ax + (seg.bx - seg.ax) * f,
                seg.ay + (seg.by - seg.ay) * f,
                look,
            )
            .clamp(0.0, 1.3);
            let b = (0.24 + 0.28 * env + 0.42 * wave * flux).clamp(0.0, 1.0);
            let glyph = CORD_RAMP[(b * 4.99) as usize];
            let col = lerp_color(look.cord_col, look.cord_hot, (wave * flux).clamp(0.0, 1.0));
            put(grid, look.w, look.h, x, y, glyph, col);
        }
    }
    if look.g < 0.62 {
        for (tx, ty) in tips {
            let x = look.cx + tx * look.aspect;
            let y = look.gy + ty;
            let tw = 0.5 + 0.5 * (look.time * 2.6).sin();
            let col = lerp_color(look.bg, look.tip_col, (0.4 + 0.6 * tw).clamp(0.0, 1.0));
            put(grid, look.w, look.h, x, y, '*', col);
        }
    }
}

fn paint_ring(grid: &mut Grid, caps: &[Cap], look: &Look) {
    for cap in caps {
        if cap.sprout < 0.02 {
            continue;
        }
        let front = 0.55 + 0.65 * (1.0 - cap.depth);
        let sz = (cap.sprout * (0.62 + 0.55 * cap.rich) * front * look.scale).clamp(0.0, 1.35);
        let rc = 1.1 + 3.2 * sz;
        let ry = (rc * 0.72).max(0.9);
        let sh = 1.6 + 3.4 * sz;
        let (bx, by) = (cap.bx, cap.by);
        let fade = |c: Color| dim(c, cap.depth, look.bg);
        let ccx = bx + cap.lean * sz;
        let ccy = by - sh;
        let swide = if sz > 0.3 { 1 } else { 0 };
        if sz < 0.12 {
            put(grid, look.w, look.h, bx, ccy, 'o', fade(look.cap_rim));
            continue;
        }
        for k in 0..(sh.ceil() as usize + 1) {
            let y = by - k as f32;
            if y < ccy {
                break;
            }
            put(grid, look.w, look.h, bx, y, '|', fade(look.stem_col));
            if swide == 1 {
                put(grid, look.w, look.h, bx + 1.0, y, '|', fade(darken(look.stem_col, 15)));
            }
        }
        let open = sz > 0.45;
        let dyr = ry.ceil() as isize;
        let dxr = rc.ceil() as isize;
        for dyi in -dyr..=dyr {
            for dxi in -dxr..=dxr {
                let (dx, dy) = (dxi as f32, dyi as f32);
                let q = (dx / (rc + 0.35)).powi(2) + (dy / (ry + 0.35)).powi(2);
                if q > 1.0 {
                    continue;
                }
                let (x, y) = (ccx + dx, ccy + dy);
                if q > 0.50 {
                    let glyph = if sz > 0.5 { 'O' } else { 'o' };
                    put(grid, look.w, look.h, x, y, glyph, fade(look.cap_rim));
                } else if dy < 0.1 * ry || !open {
                    put(grid, look.w, look.h, x, y, 'o', fade(look.cap_fill));
                } else {
                    put(grid, look.w, look.h, x, y, ':', fade(look.cap_gill));
                }
            }
        }
    }
}

fn paint_spores(grid: &mut Grid, caps: &[Cap], look: &Look) {
    let back: Vec<&Cap> = caps.iter().filter(|c| c.depth > 0.45).collect();
    let src: Vec<&Cap> = if back.is_empty() { caps.iter().collect() } else { back };
    for i in 0..46usize {
        let mh = (unit(hash(look.seed, L_SPORE, i as u64, 0)) * src.len() as f32) as usize % src.len();
        let cap = src[mh];
        let gr = 0.68 + 0.30 * unit(hash(look.seed, L_SPORE, i as u64, 1));
        let cyc = (look.g - gr + 1.0).rem_euclid(1.0);
        if cyc > 0.30 {
            continue;
        }
        let du = cyc / 0.30;
        let a = (PI * du).sin().powf(0.9);
        let u2 = unit(hash(look.seed, L_SPORE, i as u64, 2));
        let u3 = unit(hash(look.seed, L_SPORE, i as u64, 3));
        let u4 = unit(hash(look.seed, L_SPORE, i as u64, 4));
        let u5 = unit(hash(look.seed, L_SPORE, i as u64, 5));
        let x = cap.bx0 + (u2 - 0.5) * 7.0 + (-2.0 - 4.0 * u3) * du
            + look.weave * 1.2 * (TAU * (1.5 * du) + u4 * TAU).sin();
        let y = cap.by0 - 1.5 - du * (3.5 + 5.0 * u5);
        let glyph = if a > 0.6 { '*' } else { '.' };
        let col = lerp_color(look.bg, look.spore_col, (a * look.glow).clamp(0.0, 1.0));
        put(grid, look.w, look.h, x, y, glyph, col);
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = measure_layer(NAME, "look", || Look::new(frame.seed, w, h, frame.palette, frame.time, p));
    let mut caps = measure_layer(NAME, "look", || ring_caps(&look));
    let fw = w / 4 + 2;
    let fh = h / 4 + 2;
    FIELD.with(|store| {
        let mut store = store.borrow_mut();
        if store.len() < fw * fh {
            store.resize(fw * fh, 0.0);
        }
        let field = &mut store[..fw * fh];
        measure_layer(NAME, "field", || build_field(field, fw, fh, &look, &mut caps));
        NET.with(|hold| {
            let mut net = hold.borrow_mut();
            let tips = measure_layer(NAME, "grow", || grow_network(&mut net, field, fw, fh, &look));
            measure_layer(NAME, "soil", || paint_soil(frame.grid, &look));
            measure_layer(NAME, "cords", || {
                paint_hyphae(frame.grid, &net, &tips, field, fw, fh, &look)
            });
        });
        measure_layer(NAME, "ring", || paint_ring(frame.grid, &caps, &look));
        measure_layer(NAME, "spores", || paint_spores(frame.grid, &caps, &look));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::grid_to_plain;
    use rand::{rngs::StdRng, SeedableRng};

    fn knobs() -> Vec<f32> {
        PARAMS.iter().map(|p| p.default).collect()
    }

    fn frame(w: usize, h: usize, seed: u64, time: f32, values: &[f32]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(values),
        });
        grid
    }

    fn text(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn fairy_ring_80x24() {
        insta::assert_snapshot!("fairy_ring_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn fairy_ring_colonizing_t9() {
        insta::assert_snapshot!("fairy_ring_colonizing_t9", text(&frame(80, 24, 42, 9.0, &knobs())));
    }

    #[test]
    fn fairy_ring_mature_t19_8() {
        insta::assert_snapshot!("fairy_ring_mature_t19_8", text(&frame(80, 24, 42, 19.8, &knobs())));
    }

    #[test]
    fn fairy_ring_release_t32_4() {
        insta::assert_snapshot!("fairy_ring_release_t32_4", text(&frame(80, 24, 42, 32.4, &knobs())));
    }

    #[test]
    fn fairy_ring_rich_variant() {
        let mut k = knobs();
        k[6] = 1.8;
        k[3] = 22.0;
        insta::assert_snapshot!("fairy_ring_rich_variant", text(&frame(80, 24, 42, 19.8, &k)));
    }

    #[test]
    fn fairy_ring_clip_40x12() {
        insta::assert_snapshot!("fairy_ring_clip_40x12", text(&frame(40, 12, 42, 19.8, &knobs())));
    }

    #[test]
    fn fairy_ring_frontier_pair_a() {
        let mut k = knobs();
        k[1] = 0.68;
        insta::assert_snapshot!("fairy_ring_frontier_pair_a", text(&frame(80, 24, 42, 19.8, &k)));
    }

    #[test]
    fn fairy_ring_frontier_pair_b() {
        let mut k = knobs();
        k[1] = 0.72;
        insta::assert_snapshot!("fairy_ring_frontier_pair_b", text(&frame(80, 24, 42, 19.8, &k)));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 7.0, &k)), text(&frame(90, 30, 42, 7.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 7.0, &k)), text(&frame(90, 30, 7, 7.0, &k)));
    }

    #[test]
    fn time_advances_the_colony() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 9.0, &k)));
    }

    #[test]
    fn frontier_changes_the_ring() {
        let k = knobs();
        let a = text(&frame(90, 30, 42, 19.8, &k));
        let mut wide = k.clone();
        wide[1] = 0.45;
        assert_ne!(a, text(&frame(90, 30, 42, 19.8, &wide)));
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..100 {
            let t0 = std::time::Instant::now();
            frame(w, h, 42, f as f32 * 0.4, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 100.0;
        eprintln!("fairy-ring frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
