//! poincare -- a {p,q} hyperbolic tessellation in the Poincare disk or the upper
//! half-plane, sliding under a slow Mobius flow with glowing geodesics and Escher tile families.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::rngs::StdRng;
use std::f32::consts::PI;

const TILINGS: [(u32, u32); 5] = [(7, 3), (5, 4), (3, 7), (4, 5), (6, 4)];
const MAX_OPS: usize = 256;
const LUT: usize = 256;
const LEVELS: usize = 16;
const RAMP_A: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
const RAMP_B: [char; 8] = [' ', '·', ':', 'o', '•', 'O', '●', '◆'];
const MAX_ARCS: usize = 8;

pub(crate) struct PoincareKnobs {
    pub p: f32,
    pub q: f32,
    pub depth: f32,
    pub speed: f32,
    pub twist: f32,
    pub hue: f32,
    pub model: f32,
    pub edge: f32,
    pub haze: f32,
    pub arcs: f32,
    pub glow: f32,
    pub petals: f32,
    pub fade: f32,
    pub aspect: f32,
    pub threads: f32,
    pub sway: f32,
    pub label: f32,
    pub span: f32,
    pub star: f32,
    pub pulse: f32,
    pub line: f32,
    pub detail: f32,
    pub dots: f32,
    pub focus: f32,
    pub thick: f32,
    pub rings: f32,
    pub dither: f32,
}

impl PoincareKnobs {
    pub(crate) fn from_env() -> Self {
        PoincareKnobs {
            p: param_f32("P", 0.0),
            q: param_f32("Q", 0.0),
            depth: param_f32("DEPTH", 24.0),
            speed: param_f32("SPEED", 0.05),
            twist: param_f32("TWIST", 2.0),
            hue: param_f32("HUE", 0.0),
            model: param_f32("MODEL", 0.0),
            edge: param_f32("EDGE", 0.07),
            haze: param_f32("HAZE", 2.5),
            arcs: param_f32("ARCS", 3.0),
            glow: param_f32("GLOW", 0.25),
            petals: param_f32("PETALS", 1.0),
            fade: param_f32("FADE", 3.0),
            aspect: param_f32("ASPECT", 2.0),
            threads: param_f32("THREADS", 0.0),
            sway: param_f32("SWAY", 0.4),
            label: param_f32("LABEL", 1.0),
            span: param_f32("SPAN", 2.6),
            star: param_f32("STAR", 0.16),
            pulse: param_f32("PULSE", 0.15),
            line: param_f32("LINE", 0.8),
            detail: param_f32("DETAIL", 10.0),
            dots: param_f32("DOTS", 0.2),
            focus: param_f32("FOCUS", 0.55),
            thick: param_f32("THICK", 1.8),
            rings: param_f32("RINGS", 1.5),
            dither: param_f32("DITHER", 0.08),
        }
    }
}

#[derive(Clone, Copy)]
struct C {
    re: f32,
    im: f32,
}

impl C {
    const fn new(re: f32, im: f32) -> C {
        C { re, im }
    }
    fn mul(self, o: C) -> C {
        C::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
    fn add(self, o: C) -> C {
        C::new(self.re + o.re, self.im + o.im)
    }
    fn div(self, o: C) -> C {
        let d = o.re * o.re + o.im * o.im;
        let inv = if d > 1e-30 { 1.0 / d } else { 0.0 };
        C::new((self.re * o.re + self.im * o.im) * inv, (self.im * o.re - self.re * o.im) * inv)
    }
    fn norm2(self) -> f32 {
        self.re * self.re + self.im * self.im
    }
}

/// Mobius map z -> (a z + b) / (c z + d).
#[derive(Clone, Copy)]
struct Mob {
    a: C,
    b: C,
    c: C,
    d: C,
}

impl Mob {
    fn apply(&self, z: C) -> C {
        self.a.mul(z).add(self.b).div(self.c.mul(z).add(self.d))
    }
    /// Image of z and the complex derivative there.
    fn apply_d(&self, z: C, det: C) -> (C, C) {
        let den = self.c.mul(z).add(self.d);
        (self.a.mul(z).add(self.b).div(den), det.div(den.mul(den)))
    }
    fn det(&self) -> C {
        self.a.mul(self.d).add(C::new(-1.0, 0.0).mul(self.b.mul(self.c)))
    }
    fn then(&self, outer: &Mob) -> Mob {
        Mob {
            a: outer.a.mul(self.a).add(outer.b.mul(self.c)),
            b: outer.a.mul(self.b).add(outer.b.mul(self.d)),
            c: outer.c.mul(self.a).add(outer.d.mul(self.c)),
            d: outer.c.mul(self.b).add(outer.d.mul(self.d)),
        }
    }
    fn rotation(theta: f32) -> Mob {
        Mob { a: C::new(theta.cos(), theta.sin()), b: C::new(0.0, 0.0), c: C::new(0.0, 0.0), d: C::new(1.0, 0.0) }
    }
    /// Hyperbolic translation by distance s along the geodesic at angle phi through the origin.
    fn translation(s: f32, phi: f32) -> Mob {
        let tau = (s * 0.5).tanh();
        let e = C::new(phi.cos() * tau, phi.sin() * tau);
        Mob { a: C::new(1.0, 0.0), b: e, c: C::new(e.re, -e.im), d: C::new(1.0, 0.0) }
    }
    fn scale(f: f32) -> Mob {
        Mob { a: C::new(f, 0.0), b: C::new(0.0, 0.0), c: C::new(0.0, 0.0), d: C::new(1.0, 0.0) }
    }
    fn cayley() -> Mob {
        Mob { a: C::new(1.0, 0.0), b: C::new(0.0, -1.0), c: C::new(1.0, 0.0), d: C::new(0.0, 1.0) }
    }
}

struct Tiling {
    p: u32,
    q: u32,
    om: f32,
    ov: f32,
    r_v: f32,
    c: f32,
    rho: f32,
    rho2: f32,
    wrap: f32,
    sin_a: f32,
    cos_a: f32,
    sin_2a: f32,
    cos_2a: f32,
}

fn acosh(x: f32) -> f32 {
    let x = x.max(1.0);
    (x + (x * x - 1.0).sqrt()).ln()
}

impl Tiling {
    fn new(p: u32, q: u32) -> Tiling {
        let a = PI / p as f32;
        let b = PI / q as f32;
        let om = acosh(b.cos() / a.sin());
        let ov = acosh(a.cos() / a.sin() * b.cos() / b.sin());
        let mv = acosh(a.cos() / b.sin());
        let r_m = (om * 0.5).tanh();
        let r_v = (ov * 0.5).tanh();
        let c = (r_m + 1.0 / r_m) * 0.5;
        let rho = (1.0 / r_m - r_m) * 0.5;
        let wrap = if p % 2 == 0 {
            4.0 * om
        } else if q % 2 == 0 {
            4.0 * (om + ov)
        } else {
            2.0 * (om + ov + mv)
        };
        Tiling { p, q, om, ov, r_v, c, rho, rho2: rho * rho, wrap, sin_a: a.sin(), cos_a: a.cos(), sin_2a: (2.0 * a).sin(), cos_2a: (2.0 * a).cos() }
    }
}

fn pick_tiling(seed: u64, k: &PoincareKnobs) -> (u32, u32) {
    let auto = TILINGS[((seed_bits(seed, 0) * 5.0) as usize).min(4)];
    let p = if k.p >= 3.0 { (k.p.round() as u32).clamp(3, 16) } else { auto.0 };
    let mut q = if k.q >= 3.0 { (k.q.round() as u32).clamp(3, 16) } else { auto.1 };
    if k.p >= 3.0 && k.q < 3.0 {
        q = auto.1;
    }
    while (p as f32).recip() + (q as f32).recip() >= 0.5 - 1e-6 {
        q += 1;
    }
    (p, q)
}

fn hash(x: u32, y: u32, k: u32, seed: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9) ^ (k as u64).wrapping_mul(0x94D0_49BB_1331_11EB) ^ seed;
    h ^= h >> 31;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 32;
    (h & 0xFF_FFFF) as f32 / 16_777_216.0
}

struct Reduced {
    x: f32,
    y: f32,
    ring: u32,
    par: u32,
    n: usize,
    da: C,
}

/// Reflect a disk point into the fundamental triangle, then replay the word on the
/// origin to recover the tile center in the tiling frame.
fn reduce(tl: &Tiling, mut x: f32, mut y: f32, depth: u32, ops: &mut [u8; MAX_OPS], d0: C) -> Option<Reduced> {
    let mut n = 0usize;
    let mut da = d0;
    let mut ring = 0u32;
    let mut par = 0u32;
    loop {
        let mut guard = 0u32;
        loop {
            if y < 0.0 {
                y = -y;
                par ^= 1;
                da.im = -da.im;
                ops[n] = 0;
                n += 1;
                if n == MAX_OPS {
                    return None;
                }
            }
            if x * tl.sin_a - y * tl.cos_a < 0.0 {
                let nx = x * tl.cos_2a + y * tl.sin_2a;
                let ny = x * tl.sin_2a - y * tl.cos_2a;
                x = nx;
                y = ny;
                par ^= 1;
                da = C::new(tl.cos_2a, tl.sin_2a).mul(C::new(da.re, -da.im));
                ops[n] = 1;
                n += 1;
                if n == MAX_OPS {
                    return None;
                }
            } else {
                break;
            }
            guard += 1;
            if guard > 40 {
                return None;
            }
        }
        let dx = x - tl.c;
        let dd = dx * dx + y * y;
        if dd < tl.rho2 {
            let inv = 1.0 / dd;
            let f = tl.rho2 * inv;
            let g = -f * inv;
            da = C::new(g * (dx * dx - y * y), g * 2.0 * dx * y).mul(C::new(da.re, -da.im));
            x = tl.c + dx * f;
            y *= f;
            par ^= 1;
            ring += 1;
            ops[n] = 2;
            n += 1;
            if n == MAX_OPS || ring > depth {
                return None;
            }
        } else {
            break;
        }
    }
    Some(Reduced { x, y, ring, par, n, da })
}

/// Replay the word on the origin: the tile center in the tiling frame.
fn tile_center(tl: &Tiling, ops: &[u8]) -> (f32, f32) {
    let (mut cx, mut cy) = (0.0f32, 0.0f32);
    for &op in ops.iter().rev() {
        match op {
            0 => cy = -cy,
            1 => {
                let nx = cx * tl.cos_2a + cy * tl.sin_2a;
                let ny = cx * tl.sin_2a - cy * tl.cos_2a;
                cx = nx;
                cy = ny;
            }
            _ => {
                let dx = cx - tl.c;
                let f = tl.rho2 / (dx * dx + cy * cy);
                cx = tl.c + dx * f;
                cy *= f;
            }
        }
    }
    (cx, cy)
}

#[derive(Clone, Copy)]
struct Comet {
    tau: f32,
    glow: f32,
}

struct Frame {
    tl: Tiling,
    half: bool,
    pre: Mob,
    pre_det: C,
    fwd: Mob,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    unit: f32,
    aspect: f32,
    depth: u32,
    haze: f32,
    tile_units: f32,
    inner_units: f32,
    rings: f32,
    dither: f32,
    thick: f32,
    edge_w: f32,
    line: f32,
    detail: f32,
    dot_r: f32,
    star_w: f32,
    star_scale: f32,
    inv_sector: f32,
    petals: f32,
    checker_gen: bool,
    comets: [Comet; MAX_ARCS],
    n_comets: usize,
    glow_hyp: f32,
    shade_lut: [f32; LUT],
    sin_lut: [f32; LUT],
    pulse_ph: f32,
    star_ph: f32,
    radial_lut: [f32; LUT],
    lut_a: [Color; LEVELS],
    lut_b: [Color; LEVELS],
    lut_edge: [Color; LEVELS],
    lut_haze: [Color; 3],
    star: Color,
    star_dim: Color,
    comet_fg: Color,
    bg_a: Color,
    bg_b: Color,
    pulse: f32,
    seed: u64,
    t: f32,
}

fn seed_bits(seed: u64, k: u32) -> f32 {
    hash(k, 77, 11, seed)
}

fn build_frame(w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &PoincareKnobs, focus_y: f32) -> Frame {
    let (p, q) = pick_tiling(seed, k);
    let tl = Tiling::new(p, q);
    let half = match k.model.round() as i32 {
        1 => false,
        2 => true,
        _ => seed_bits(seed, 1) < 0.4,
    };
    let aspect = k.aspect.clamp(0.25, 4.0);

    // flow: axis translation (wrapped to the tiling period), perpendicular sway, twist
    let dir = if seed_bits(seed, 2) < 0.5 { 1.0 } else { -1.0 };
    let theta0 = seed_bits(seed, 3) * 2.0 * PI;
    let s_raw = dir * k.speed * t;
    let s = s_raw.rem_euclid(tl.wrap);
    let sway_t = 37.0 + 11.0 * seed_bits(seed, 4);
    let sigma = k.sway * (2.0 * PI * t / sway_t + seed_bits(seed, 5) * 2.0 * PI).sin();
    let theta = theta0 + k.twist.to_radians() * t;
    let fwd = Mob::translation(s, 0.0).then(&Mob::translation(sigma, PI * 0.5)).then(&Mob::rotation(theta));
    let inv = Mob::rotation(-theta).then(&Mob::translation(-sigma, PI * 0.5)).then(&Mob::translation(-s, 0.0));
    let pre = if half { Mob::scale(1.0 / focus_y).then(&Mob::cayley()).then(&inv) } else { inv };

    // comets: posts spread along the axis, one lights up whenever it passes the center
    let n_comets = (k.arcs.round() as usize).min(MAX_ARCS);
    let mut comets = [Comet { tau: 0.0, glow: 0.0 }; MAX_ARCS];
    let glow_w = k.glow.max(0.01);
    for (i, cm) in comets.iter_mut().enumerate().take(n_comets) {
        let a = (i as f32 + seed_bits(seed, 20 + i as u32)) / n_comets as f32 * tl.wrap;
        let mut phase = (s - a).rem_euclid(tl.wrap);
        if phase > tl.wrap * 0.5 {
            phase -= tl.wrap;
        }
        let reach = (phase / (glow_w * 3.0).min(tl.wrap / n_comets as f32 * 0.5)).powi(2);
        cm.glow = (-reach).exp();
        cm.tau = ((s - phase) * 0.5).tanh();
    }

    let hue_shift = (seed % 360) as f64 * 0.37 + k.hue as f64;
    let pal: Vec<Color> = palette.iter().map(|c| shift_hue(*c, hue_shift)).collect();
    let mut lut_a = [Color::Reset; LEVELS];
    let mut lut_b = [Color::Reset; LEVELS];
    let mut lut_edge = [Color::Reset; LEVELS];
    for i in 0..LEVELS {
        let f = i as f32 / (LEVELS - 1) as f32;
        lut_a[i] = lerp_color(darken(pal[1], 90), lighten(pal[1], 70), f);
        lut_b[i] = lerp_color(darken(pal[3], 90), lighten(pal[3], 70), f);
        lut_edge[i] = lerp_color(darken(pal[2], 70), lighten(pal[2], 40), f);
    }
    let mut shade_lut = [0.0f32; LUT];
    let mut sin_lut = [0.0f32; LUT];
    let mut radial_lut = [0.0f32; LUT];
    let fade = k.fade.max(0.2);
    for i in 0..LUT {
        let r = (i as f32 / (LUT - 1) as f32).min(0.9999);
        let d = ((1.0 + r) / (1.0 - r)).ln();
        shade_lut[i] = (-d / fade).exp();
        sin_lut[i] = (i as f32 / LUT as f32 * 2.0 * PI).sin();
        let rl = r * tl.r_v;
        radial_lut[i] = (((1.0 + rl) / (1.0 - rl)).ln() / tl.ov).min(1.0);
    }
    Frame {
        half,
        pre_det: pre.det(),
        pre,
        fwd,
        cx: w as f32 * 0.5,
        cy: h as f32 * 0.5,
        rx: 0.0,
        ry: 0.0,
        unit: 0.0,
        aspect,
        depth: (k.depth.round() as u32).clamp(2, 128),
        haze: k.haze.max(0.05),
        tile_units: 2.0 * tl.ov,
        inner_units: 2.0 * tl.om,
        rings: k.rings.max(0.0),
        dither: k.dither.clamp(0.0, 1.0),
        thick: k.thick.max(0.5),
        edge_w: k.edge.max(0.0),
        line: k.line.max(0.0),
        detail: k.detail.max(0.5),
        dot_r: k.dots.clamp(0.0, 1.0),
        star_w: k.star.max(0.0),
        star_scale: 1.0 - tl.r_v * tl.r_v,
        inv_sector: p as f32 / PI,
        petals: k.petals.max(0.25),
        checker_gen: q % 2 == 0,
        comets,
        n_comets,
        glow_hyp: glow_w * 0.6,
        shade_lut,
        sin_lut,
        pulse_ph: t * 0.7 / (2.0 * PI),
        star_ph: t * 1.3 / (2.0 * PI),
        radial_lut,
        lut_a,
        lut_b,
        lut_edge,
        lut_haze: [darken(pal[2], 40), darken(pal[2], 70), darken(pal[2], 95)],
        star: lighten(pal[4], 30),
        star_dim: darken(pal[4], 60),
        comet_fg: lighten(pal[4], 40),
        bg_a: darken(pal[0], 10),
        bg_b: lerp_color(darken(pal[0], 10), pal[3], 0.16),
        pulse: k.pulse.clamp(0.0, 1.0),
        seed,
        t,
        tl,
    }
}

fn level(v: f32, n: usize) -> usize {
    ((v.clamp(0.0, 0.9999)) * n as f32) as usize
}

fn haze_cell(fr: &Frame, sc: f32, x: u32, y: u32) -> Cell {
    let f = (sc / fr.haze).clamp(0.0, 1.0);
    let jitter = hash(x, y, 3, fr.seed);
    let (ch, ci) = if f > 0.7 {
        (if jitter < 0.5 { ':' } else { '.' }, 0)
    } else if f > 0.4 {
        ('░', 1)
    } else {
        ('▒', 2)
    };
    Cell::new(ch, fr.lut_haze[ci])
}

/// Per-thread memo: cells whose reflection word matches the last one share a tile.
struct TileMemo {
    n: usize,
    ops: [u8; MAX_OPS],
    shade: f32,
    tile_h: f32,
}

impl TileMemo {
    fn new() -> TileMemo {
        TileMemo { n: usize::MAX, ops: [0; MAX_OPS], shade: 0.0, tile_h: 0.0 }
    }
}

fn shade_cell(fr: &Frame, x: usize, y: usize, ops: &mut [u8; MAX_OPS], memo: &mut TileMemo) -> Cell {
    let (z, sc) = if fr.half {
        let zx = (x as f32 + 0.5 - fr.cx) * fr.unit;
        let zy = (fr.cy * 2.0 - y as f32 - 0.5) * fr.unit * fr.aspect;
        (C::new(zx, zy), zy / (fr.unit * fr.aspect))
    } else {
        let zx = (x as f32 + 0.5 - fr.cx) / fr.rx;
        let zy = (fr.cy - y as f32 - 0.5) / fr.ry;
        let r2 = zx * zx + zy * zy;
        if r2 >= 1.0 {
            return Cell::blank();
        }
        (C::new(zx, zy), fr.ry * (1.0 - r2) * 0.5)
    };
    let tile_rows = sc * fr.tile_units;
    if tile_rows < fr.haze {
        return haze_cell(fr, tile_rows, x as u32, y as u32);
    }
    let (wpt, dpre) = fr.pre.apply_d(z, fr.pre_det);
    if wpt.norm2() >= 0.9999 {
        return haze_cell(fr, 0.0, x as u32, y as u32);
    }
    let Some(rd) = reduce(&fr.tl, wpt.re, wpt.im, fr.depth, ops, dpre) else {
        return haze_cell(fr, 0.0, x as u32, y as u32);
    };
    let tl = &fr.tl;

    // tile shade from the tile center's distance to the view focus
    if memo.n != rd.n || memo.ops[..rd.n] != ops[..rd.n] {
        let (cx, cy) = tile_center(tl, &ops[..rd.n]);
        let cv = fr.fwd.apply(C::new(cx, cy));
        memo.n = rd.n;
        memo.ops[..rd.n].copy_from_slice(&ops[..rd.n]);
        memo.shade = fr.shade_lut[level(cv.norm2().sqrt(), LUT)];
        memo.tile_h = hash((cx * 4096.0).round() as i32 as u32, (cy * 4096.0).round() as i32 as u32, 9, fr.seed);
    }
    let shade = memo.shade;
    let tile_h = memo.tile_h;
    let pulse = 1.0 - fr.pulse + fr.pulse * fr.sin_lut[level((fr.pulse_ph + tile_h).fract(), LUT)];
    let shade = shade * pulse;
    let family = if fr.checker_gen { rd.ring & 1 } else { (tile_h < 0.5) as u32 };
    let bg = if family == 0 { fr.bg_a } else { fr.bg_b };

    let r2 = rd.x * rd.x + rd.y * rd.y;
    let r = r2.sqrt();
    let radial = fr.radial_lut[level(r / tl.r_v, LUT)];
    let dxc = rd.x - tl.c;
    let edge_s = ((dxc * dxc + rd.y * rd.y) - tl.rho2) / (tl.rho * (1.0 - r2));
    let vx = rd.x - tl.r_v * tl.cos_a;
    let vy = rd.y - tl.r_v * tl.sin_a;

    // level of detail from the local scale: far tiles keep only their edge mesh
    let lod = ((sc * fr.inner_units - fr.detail * 0.4) / (fr.detail * 0.6)).clamp(0.0, 1.0);
    let lod = lod * lod * (3.0 - 2.0 * lod);
    let thr = fr.edge_w.clamp(fr.line / sc, 2.0 * fr.line / sc);
    let thr_s = thr * (1.0 + thr * thr / 6.0);
    let star_ok = fr.star_w * sc >= 0.5 && lod > 0.0;

    let star_eff = fr.star_w.min(0.7 / sc) * fr.star_scale;
    let mut cell = if star_ok && vx * vx + vy * vy < star_eff * star_eff {
        let tw = 0.5 + 0.5 * fr.sin_lut[level((fr.star_ph + tile_h * 1.43).fract(), LUT)];
        let bright = shade * (0.5 + 0.5 * tw);
        let ch = if bright > 0.45 { '*' } else { '+' };
        Cell::with_bg(ch, lerp_color(fr.star_dim, fr.star, bright), bg)
    } else if edge_s < thr_s {
        let li = level(shade, LEVELS);
        let ch = if shade < 0.1 {
            '.'
        } else if thr * sc > fr.thick {
            if shade > 0.5 { '#' } else { '%' }
        } else {
            let tv = C::new(-rd.y, dxc).div(rd.da);
            let (vx, vy) = if rd.par == 1 { (tv.re, -tv.im) } else { (tv.re, tv.im) };
            let (ex, ey) = ((vx * fr.aspect).abs(), vy.abs());
            if ey < 0.42 * ex {
                '-'
            } else if ex < 0.42 * ey {
                '|'
            } else if (vx * fr.aspect) * (-vy) > 0.0 {
                '\\'
            } else {
                '/'
            }
        };
        Cell::with_bg(ch, fr.lut_edge[li], bg)
    } else if lod <= 0.0 {
        let ch = if radial < fr.dot_r { if family == 0 { 'o' } else { '•' } } else { ' ' };
        let fg = if family == 0 { fr.lut_a[level(shade, LEVELS)] } else { fr.lut_b[level(shade, LEVELS)] };
        Cell::with_bg(ch, fg, bg)
    } else {
        let ang = rd.y.atan2(rd.x) * fr.inv_sector;
        let petal = 0.5 + 0.5 * (fr.petals * PI * ang + if family == 0 { 0.0 } else { PI }).cos();
        let core = (1.0 - radial * 2.5).max(0.0);
        let ring = 0.55 + 0.45 * (2.0 * PI * fr.rings * radial).cos();
        let body = (1.0 - radial).sqrt() * (0.3 + 0.7 * petal) * ring;
        let grain = (hash(x as u32, y as u32, 4, fr.seed) - 0.5) * fr.dither;
        let inten = (shade * (core + body).min(1.0) * lod * lod + grain).max(0.0);
        if family == 0 {
            let li = level(inten, RAMP_A.len());
            Cell::with_bg(RAMP_A[li], fr.lut_a[level(inten, LEVELS)], bg)
        } else {
            let li = level(inten, RAMP_B.len());
            Cell::with_bg(RAMP_B[li], fr.lut_b[level(inten, LEVELS)], bg)
        }
    };

    for cm in fr.comets.iter().take(fr.n_comets) {
        if cm.glow < 0.02 {
            continue;
        }
        let den = C::new(1.0 - cm.tau * wpt.re, -cm.tau * wpt.im);
        let wp = C::new(wpt.re - cm.tau, wpt.im).div(den);
        let n2 = wp.norm2();
        if n2 >= 1.0 {
            continue;
        }
        let ds = 2.0 * wp.re.abs() / (1.0 - n2);
        let band = fr.glow_hyp.min(1.2 / sc);
        let band_s = band * (1.0 + band * band / 6.0);
        if ds < band_s {
            let f = (1.0 - ds / band_s) * cm.glow;
            if f > 0.45 {
                cell = Cell::with_bg('~', fr.comet_fg, bg);
            } else {
                cell.fg = lerp_color(cell.fg, fr.comet_fg, f * 1.2);
            }
        }
    }
    cell
}

fn paint_rows(rows: &mut [Vec<Cell>], y0: usize, w: usize, fr: &Frame) {
    let mut ops = [0u8; MAX_OPS];
    let mut memo = TileMemo::new();
    for (i, row) in rows.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            *cell = shade_cell(fr, x, y0 + i, &mut ops, &mut memo);
        }
    }
}

/// Rows go out in small chunks pulled from a shared queue so fast and slow cores stay busy.
fn paint_field(grid: &mut Grid, w: usize, h: usize, fr: &Frame, threads: usize) {
    if threads <= 1 {
        paint_rows(&mut grid[..h], 0, w, fr);
        return;
    }
    let chunk = 4usize;
    let mut queue: Vec<(usize, &mut [Vec<Cell>])> = grid[..h].chunks_mut(chunk).enumerate().map(|(i, c)| (i * chunk, c)).collect();
    queue.reverse();
    let queue = std::sync::Mutex::new(queue);
    std::thread::scope(|s| {
        for _ in 0..threads {
            let queue = &queue;
            s.spawn(move || {
                loop {
                    let next = queue.lock().map(|mut q| q.pop()).unwrap_or(None);
                    let Some((y0, rows)) = next else { break };
                    paint_rows(rows, y0, w, fr);
                }
            });
        }
    });
}

fn put_text(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, text: &str, fg: Color) {
    for (i, ch) in text.chars().enumerate() {
        let xx = x + i as i32;
        if xx >= 0 && y >= 0 && (xx as usize) < w && (y as usize) < h {
            let bg = grid[y as usize][xx as usize].bg;
            grid[y as usize][xx as usize] = Cell::with_bg(ch, fg, bg);
        }
    }
}

pub(crate) fn draw_poincare(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &PoincareKnobs) {
    measure_layer("poincare", "clear", || {
        for row in grid.iter_mut().take(h) {
            for cell in row.iter_mut().take(w) {
                *cell = Cell::blank();
            }
        }
    });
    if w < 4 || h < 3 {
        return;
    }
    let t = t.max(0.0);
    let aspect = k.aspect.clamp(0.25, 4.0);
    let unit = 2.0 * k.span.max(0.2) / w as f32;
    let focus_y = (h as f32 * unit * aspect * k.focus.clamp(0.05, 0.95)).max(1e-3);
    let mut fr = measure_layer("poincare", "setup", || build_frame(w, h, seed, palette, t, k, focus_y));
    let mut ry = (h as f32) * 0.5 - 0.5;
    let mut rx = ry * fr.aspect;
    if rx * 2.0 > w as f32 - 1.0 {
        rx = (w as f32 - 1.0) * 0.5;
        ry = rx / fr.aspect;
    }
    fr.rx = rx.max(1.0);
    fr.ry = ry.max(1.0);
    fr.unit = unit;

    let threads = {
        let want = k.threads.round() as usize;
        if want >= 1 { want.min(64) } else { std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) }
    };
    measure_layer("poincare", "field", || paint_field(grid, w, h, &fr, threads));

    measure_layer("poincare", "rim", || {
        if fr.half {
            let fg = fr.lut_haze[0];
            for x in 0..w {
                let ch = if hash(x as u32, 0, 5, seed) < 0.5 { '=' } else { '-' };
                grid[h - 1][x] = Cell::new(ch, fg);
            }
        }
    });

    measure_layer("poincare", "label", || {
        if k.label > 0.5 {
            let text = format!("{{{},{}}}", fr.tl.p, fr.tl.q);
            put_text(grid, w, h, 1, 0, &text, fr.star_dim);
        }
    });
}

pub(crate) fn cli_poincare(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = PoincareKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 27] = [
        &mut k.p,
        &mut k.q,
        &mut k.depth,
        &mut k.speed,
        &mut k.twist,
        &mut k.hue,
        &mut k.model,
        &mut k.edge,
        &mut k.haze,
        &mut k.arcs,
        &mut k.glow,
        &mut k.petals,
        &mut k.fade,
        &mut k.aspect,
        &mut k.threads,
        &mut k.sway,
        &mut k.label,
        &mut k.span,
        &mut k.star,
        &mut k.pulse,
        &mut k.line,
        &mut k.detail,
        &mut k.dots,
        &mut k.focus,
        &mut k.thick,
        &mut k.rings,
        &mut k.dither,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_poincare(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = PoincareKnobs::from_env();
        draw_poincare(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_poincare_small() {
        insta::assert_snapshot!("poincare_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_poincare_flow() {
        insta::assert_snapshot!("poincare_80x24_t12", run(80, 24, 42, 12.0));
    }

    #[test]
    fn tilings_are_hyperbolic() {
        for (p, q) in TILINGS {
            let tl = Tiling::new(p, q);
            assert!(tl.om.is_finite() && tl.ov.is_finite() && tl.wrap > 0.0, "{{{p},{q}}}");
            assert!(tl.c > 1.0 && tl.rho > 0.0);
        }
    }

    #[test]
    fn wrap_is_a_symmetry() {
        for (p, q) in TILINGS {
            let tl = Tiling::new(p, q);
            let mut ops = [0u8; MAX_OPS];
            let shift = Mob::translation(tl.wrap, 0.0);
            for i in 0..40 {
                let z = C::new(0.05 + 0.017 * i as f32, 0.31 - 0.011 * i as f32);
                let a = reduce(&tl, z.re, z.im, 64, &mut ops, C::new(1.0, 0.0)).unwrap();
                let zs = shift.apply(z);
                let b = reduce(&tl, zs.re, zs.im, 64, &mut ops, C::new(1.0, 0.0)).unwrap();
                assert!((a.x - b.x).abs() < 2e-3 && (a.y - b.y).abs() < 2e-3, "{{{p},{q}}} {i} {} {} vs {} {}", a.x, a.y, b.x, b.y);
                assert_eq!(a.par, b.par, "{{{p},{q}}} parity {i}");
            }
        }
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn t_moves_the_flow() {
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 6.0));
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = PoincareKnobs::from_env();
        draw_poincare(&mut g, w, h, 42, &p, 0.0, &k);
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..200 {
            let t0 = std::time::Instant::now();
            draw_poincare(&mut g, w, h, 42, &p, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("poincare frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }

    #[test]
    #[ignore = "manual 2000x1000 timing; ASCII_P_THREADS picks the thread count"]
    fn bench_big() {
        let (w, h) = (2000usize, 1000usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = PoincareKnobs::from_env();
        draw_poincare(&mut g, w, h, 42, &p, 0.0, &k);
        let start = std::time::Instant::now();
        for f in 0..10 {
            draw_poincare(&mut g, w, h, 42, &p, 3.0 + f as f32 * 0.06, &k);
        }
        eprintln!("poincare bench 2000x1000: {:.2} ms/frame", start.elapsed().as_secs_f64() * 100.0);
    }
}
