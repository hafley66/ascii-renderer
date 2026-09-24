//! Streamnet: the orthogonal equipotential and streamline families of a seeded
//! analytic complex potential, `w(z) = sum c_k z^k + sum q_p / (z - p_p)`, drawn
//! as a two-colour line net with dipoles that orbit and flow comets riding it.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lerp_color};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::TAU;

pub(super) struct StreamNet;
pub(super) static MODE: StreamNet = StreamNet;

const NAME: &str = "streamnet";
const KNOBS: usize = 17;
const HELP: &str = "streamnet: equipotential/streamline net of a morphing complex potential; families, dipoles, bands, flow comets [zoom] [degree] [density] [thick] [warp] [spin] [morph] [hue] [aspect] [depth] [ground] [halo] [poles] [strength] [tracers] [form] [flowdir]";

const PARAMS: &[Param] = &[
    param!("ZOOM", "plane scale", 0.8, 2.0, 1.0, 0.05),
    param!("DEGREE", "polynomial degree", 2.0, 6.0, 3.0, 1.0),
    param!("DENSITY", "lines per unit", 1.5, 8.0, 3.5, 0.5),
    param!("THICK", "line weight", 0.05, 0.45, 0.15, 0.01),
    param!("WARP", "coordinate warp", 0.0, 0.6, 0.1, 0.02),
    param!("SPIN", "rotation rad/s", -0.5, 0.5, 0.05, 0.01),
    param!("MORPH", "phase drift rate", 0.0, 1.2, 0.5, 0.05),
    param!("HUE", "base hue", 0.0, 360.0, 200.0, 1.0),
    param!("ASPECT", "cols per row", 0.8, 3.0, 2.0, 0.05),
    param!("DEPTH", "speed response", 0.0, 1.3, 0.8, 0.05),
    param!("GROUND", "field wash", 0.0, 1.0, 0.32, 0.05),
    param!("HALO", "line underglow", 0.0, 0.9, 0.28, 0.05),
    param!("POLES", "dipole count", 0.0, 6.0, 3.0, 1.0),
    param!("STRENGTH", "dipole strength", 0.0, 1.0, 0.5, 0.05),
    param!("TRACERS", "flow comets", 0.0, 500.0, 90.0, 10.0),
    param!("FORM", "potential family", 0.0, 3.0, 1.0, 1.0, FORM_CHOICES),
    param!("FLOWDIR", "comet drift to equipotentials", 0.0, 1.0, 0.0, 0.05),
];

const L_TERM: u64 = 0x21;
const L_WARP: u64 = 0x22;
const L_POLE: u64 = 0x23;
const L_TRAC: u64 = 0x31;
const L_BAND: u64 = 0x32;
const FORM_CHOICES: &[&str] = &["harmonic", "dipolar", "banded", "chaotic"];
const NBAND: usize = 3;

const MAXD: usize = 7;
const MAXP: usize = 6;
const NSTEP: usize = 56;
const TAIL: usize = 8;
const PARALLEL_MIN_CELLS: usize = 20_480;

impl Mode for StreamNet {
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

/// Splitmix64 over (seed, layer, index, slot); consumes no rng stream.
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
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Bilinear value noise on an integer lattice, output in -1..1.
#[inline]
fn noise(seed: u64, layer: u64, u: f32, v: f32) -> f32 {
    let (iu, iv) = (u.floor(), v.floor());
    let (fu, fv) = (smooth(u - iu), smooth(v - iv));
    let (iu, iv) = (iu as i64 as u64, iv as i64 as u64);
    let s = |du: u64, dv: u64| unit(hash(seed, layer, iu.wrapping_add(du), iv.wrapping_add(dv)));
    let a = s(0, 0) + (s(1, 0) - s(0, 0)) * fu;
    let b = s(0, 1) + (s(1, 1) - s(0, 1)) * fu;
    (a + (b - a) * fv) * 2.0 - 1.0
}

/// One frame's resolved coefficients, dipoles, geometry, and palette.
struct Look {
    seed: u64,
    cx: f32,
    cy: f32,
    inv: f32,
    aspect: f32,
    degree: usize,
    dens: f32,
    thick: f32,
    warp: f32,
    hue: f32,
    depth: f32,
    ground: f32,
    halo: f32,
    rot_c: f32,
    rot_s: f32,
    cr: [f32; MAXD],
    ci: [f32; MAXD],
    np: usize,
    pr: [f32; MAXP],
    pi: [f32; MAXP],
    qr: [f32; MAXP],
    qi: [f32; MAXP],
    nb: usize,
    bar: [f32; NBAND],
    bai: [f32; NBAND],
    bbr: [f32; NBAND],
    bbi: [f32; NBAND],
    rmin2: f32,
    bg: Color,
    wash: Color,
    tracers: usize,
    flow: f32,
    flowa: f32,
    time: f32,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let degree = (p[1].round() as i32).clamp(2, 6) as usize;
        let aspect = p[8].max(0.5);
        let zoom = p[0].max(0.2);
        let half_w = (w as f32 * 0.5) / aspect;
        let half_h = h as f32 * 0.5;
        let radius = (half_w * half_w + half_h * half_h).sqrt().max(1.0);
        let inv = 1.0 / (radius * zoom);
        let theta = p[5] * time;

        let mut cr = [0.0f32; MAXD];
        let mut ci = [0.0f32; MAXD];
        // Linear term gets a seeded phase and weight so seeds differ strongly.
        let a1 = 0.82 + 0.3 * unit(hash(seed, L_TERM, 1, 2));
        let p1 = unit(hash(seed, L_TERM, 1, 3)) * TAU;
        cr[1] = a1 * p1.cos();
        ci[1] = a1 * p1.sin();
        let mid = (degree as f32 + 1.0) * 0.5;
        for k in 2..=degree {
            let amp = (0.4 + 0.5 * unit(hash(seed, L_TERM, k as u64, 0))) / 1.6f32.powi(k as i32);
            let ph = unit(hash(seed, L_TERM, k as u64, 1)) * TAU;
            let om = (k as f32 - mid) * 0.6;
            let a = ph + om * p[6] * time;
            cr[k] = amp * a.cos();
            ci[k] = amp * a.sin();
        }

        let form = p[15].round() as i32;
        let want_poles = form == 1 || form == 3;
        let want_band = form == 2 || form == 3;

        let np = if want_poles { (p[12].round() as i32).clamp(0, MAXP as i32) as usize } else { 0 };
        let mut pr = [0.0f32; MAXP];
        let mut pi = [0.0f32; MAXP];
        let mut qr = [0.0f32; MAXP];
        let mut qi = [0.0f32; MAXP];
        for k in 0..np {
            let kk = k as u64;
            let br = 0.2 + 0.55 * unit(hash(seed, L_POLE, kk, 0));
            let ba = unit(hash(seed, L_POLE, kk, 1)) * TAU;
            let oa = unit(hash(seed, L_POLE, kk, 2)) * TAU + time * (0.4 + 0.5 * p[6]);
            pr[k] = br * ba.cos() + 0.06 * oa.cos();
            pi[k] = br * ba.sin() + 0.06 * oa.sin();
            let mag = p[13] * 0.035 * (0.6 + 0.8 * unit(hash(seed, L_POLE, kk, 3)));
            let ga = unit(hash(seed, L_POLE, kk, 4)) * TAU;
            qr[k] = mag * ga.cos();
            qi[k] = mag * ga.sin();
        }

        let nb = if want_band { NBAND } else { 0 };
        let mut bar = [0.0f32; NBAND];
        let mut bai = [0.0f32; NBAND];
        let mut bbr = [0.0f32; NBAND];
        let mut bbi = [0.0f32; NBAND];
        for k in 0..nb {
            let kk = k as u64;
            let amp = 0.06 + p[13] * (0.10 + 0.16 * unit(hash(seed, L_BAND, kk, 0)));
            let ph = unit(hash(seed, L_BAND, kk, 1)) * TAU + (k as f32 - 1.0) * 0.5 * p[6] * time;
            bar[k] = amp * ph.cos();
            bai[k] = amp * ph.sin();
            bbr[k] = 1.2 + 1.4 * unit(hash(seed, L_BAND, kk, 2));
            bbi[k] = (unit(hash(seed, L_BAND, kk, 3)) - 0.5) * 0.6;
        }

        Look {
            seed,
            cx: w as f32 * 0.5,
            cy: h as f32 * 0.5,
            inv,
            aspect,
            degree,
            dens: p[2],
            thick: p[3].max(0.02),
            warp: p[4],
            hue: p[7],
            depth: p[9],
            ground: p[10],
            halo: p[11],
            rot_c: theta.cos(),
            rot_s: theta.sin(),
            cr,
            ci,
            np,
            pr,
            pi,
            qr,
            qi,
            nb,
            bar,
            bai,
            bbr,
            bbi,
            rmin2: 0.0025,
            bg: darken(palette[0], 18),
            wash: hsl_to_rgb(((p[7] + 205.0) / 360.0).rem_euclid(1.0) as f64, 0.55, 0.13),
            tracers: (p[14].round() as usize).min(2000),
            flow: 0.55,
            flowa: p[16],
            time,
        }
    }
}

/// Per-cell potential samples: both conjugate values and the complex velocity.
#[derive(Clone, Copy, Default)]
struct Sample {
    phi: f32,
    psi: f32,
    re: f32,
    im: f32,
}

thread_local! {
    static SCRATCH: RefCell<Vec<Sample>> = const { RefCell::new(Vec::new()) };
    static PATH: RefCell<Vec<(f32, f32)>> = const { RefCell::new(Vec::new()) };
}

#[inline(always)]
fn each_row<T: Send, F>(buf: &mut [T], w: usize, h: usize, row: F)
where
    F: Fn((usize, &mut [T])) + Sync + Send,
{
    if w == 0 {
        return;
    }
    if w * h >= PARALLEL_MIN_CELLS {
        buf.par_chunks_mut(w).enumerate().with_min_len(8).for_each(row);
    } else {
        buf.chunks_mut(w).enumerate().for_each(row);
    }
}

#[inline]
fn csin(xr: f32, xi: f32) -> (f32, f32) {
    let e = xi.exp();
    let (ch, sh) = ((e + 1.0 / e) * 0.5, (e - 1.0 / e) * 0.5);
    let (s, c) = xr.sin_cos();
    (s * ch, c * sh)
}

#[inline]
fn ccos(xr: f32, xi: f32) -> (f32, f32) {
    let e = xi.exp();
    let (ch, sh) = ((e + 1.0 / e) * 0.5, (e - 1.0 / e) * 0.5);
    let (s, c) = xr.sin_cos();
    (c * ch, -s * sh)
}

/// Evaluate the analytic potential and its derivative at one plane point.
#[inline]
fn potential(a: f32, b: f32, look: &Look) -> (f32, f32, f32, f32) {
    let mut ar = 0.0f32;
    let mut ai = 0.0f32;
    let mut gr = 0.0f32;
    let mut gi = 0.0f32;
    for k in (1..=look.degree).rev() {
        let ck = k as f32;
        let nk = look.cr[k];
        let mk = look.ci[k];
        let nr = ar * a - ai * b + nk;
        let ni = ar * b + ai * a + mk;
        let gnr = gr * a - gi * b + ck * nk;
        let gni = gr * b + gi * a + ck * mk;
        ar = nr;
        ai = ni;
        gr = gnr;
        gi = gni;
    }
    let mut wr = ar * a - ai * b;
    let mut wi = ar * b + ai * a;
    let mut dr = gr;
    let mut di = gi;
    for k in 0..look.np {
        let rx = a - look.pr[k];
        let ry = b - look.pi[k];
        let d2 = (rx * rx + ry * ry).max(look.rmin2);
        let qr = look.qr[k];
        let qi = look.qi[k];
        wr += (qr * rx + qi * ry) / d2;
        wi += (qi * rx - qr * ry) / d2;
        let cr2 = rx * rx - ry * ry;
        let ci2 = -2.0 * rx * ry;
        let dd = d2 * d2;
        dr -= (qr * cr2 - qi * ci2) / dd;
        di -= (qi * cr2 + qr * ci2) / dd;
    }
    if look.nb > 0 {
        for k in 0..look.nb {
            let xr = look.bbr[k] * a - look.bbi[k] * b;
            let xi = look.bbr[k] * b + look.bbi[k] * a;
            let (sxr, sxi) = csin(xr, xi);
            let (cxr, cxi) = ccos(xr, xi);
            wr += look.bar[k] * sxr - look.bai[k] * sxi;
            wi += look.bar[k] * sxi + look.bai[k] * sxr;
            let abr = look.bar[k] * look.bbr[k] - look.bai[k] * look.bbi[k];
            let abi = look.bar[k] * look.bbi[k] + look.bai[k] * look.bbr[k];
            dr += abr * cxr - abi * cxi;
            di += abr * cxi + abi * cxr;
        }
    }
    (wr, wi, dr, di)
}

/// Map a cell centre to the (rotated) plane point, with an optional warp.
#[inline]
fn cell_to_plane(x: f32, y: f32, look: &Look, warped: bool) -> (f32, f32) {
    let dx = (x - look.cx) * look.inv / look.aspect;
    let dy = (y - look.cy) * look.inv;
    let mut a = dx * look.rot_c - dy * look.rot_s;
    let mut b = dx * look.rot_s + dy * look.rot_c;
    if warped && look.warp > 0.0 {
        a += noise(look.seed, L_WARP, dx * 2.6, dy * 2.6) * look.warp * 0.14;
        b += noise(look.seed, L_WARP, dx * 2.6 + 40.0, dy * 2.6 - 17.0) * look.warp * 0.14;
    }
    (a, b)
}

#[inline]
fn plane_to_cell(a: f32, b: f32, look: &Look) -> (f32, f32) {
    let dx = a * look.rot_c + b * look.rot_s;
    let dy = -a * look.rot_s + b * look.rot_c;
    (dx * look.aspect / look.inv + look.cx, dy / look.inv + look.cy)
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.palette, frame.time, p);
    SCRATCH.with(|slot| {
        let mut buf = slot.borrow_mut();
        if buf.len() < w * h {
            buf.resize(w * h, Sample::default());
        }
        let field = &mut buf[..w * h];
        measure_layer(NAME, "field", || eval_field(field, w, h, &look));
        measure_layer(NAME, "wash", || paint_wash(frame.grid, field, w, h, &look));
        measure_layer(NAME, "ink", || paint_ink(frame.grid, field, w, h, &look));
        if look.tracers > 0 {
            measure_layer(NAME, "flow", || paint_flow(frame.grid, w, h, &look));
        }
    });
}

/// Fill the per-cell potential samples over the whole grid.
fn eval_field(field: &mut [Sample], w: usize, h: usize, look: &Look) {
    each_row(field, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate() {
            let (a, b) = cell_to_plane(x as f32 + 0.5, y as f32 + 0.5, look, true);
            let (phi, psi, re, im) = potential(a, b, look);
            *cell = Sample { phi, psi, re, im };
        }
    });
}

/// Lay the dark plate: background tinted by local field speed.
fn paint_wash(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate() {
            let s = field[base + x];
            let spd = (s.re * s.re + s.im * s.im).sqrt();
            let vel = spd / (1.0 + spd);
            *cell = Cell::with_bg(' ', look.bg, lerp_color(look.bg, look.wash, look.ground * vel));
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        grid.par_iter_mut().enumerate().for_each(paint);
    } else {
        grid.iter_mut().enumerate().for_each(paint);
    }
}

/// Draw both families as thin lines, oriented and lit by the local velocity.
fn paint_ink(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate() {
            let s = field[base + x];
            let spd = (s.re * s.re + s.im * s.im).sqrt();
            let vel = spd / (1.0 + spd);
            let fx = s.phi * look.dens;
            let fy = s.psi * look.dens;
            let dr = (fx - fx.round()).abs();
            let dc = (fy - fy.round()).abs();
            let th = look.thick;
            let rphi = (1.0 - dr / th).max(0.0);
            let rpsi = (1.0 - dc / th).max(0.0);
            let equip = rphi >= rpsi;
            let line = if equip { rphi } else { rpsi };
            if line > 0.04 {
                let slope = orient(equip, s.re, s.im, look.aspect);
                let node = rphi > 0.55 && rpsi > 0.55;
                let glyph = if node { '*' } else { glyph_for(slope) };
                let hue = look.hue + if equip { 90.0 } else { 0.0 } + 28.0 * (vel - 0.5);
                let light = (0.4 + 0.5 * smooth(line) * (0.55 + look.depth * vel)).min(0.92);
                cell.fg = hsl_to_rgb((hue / 360.0).rem_euclid(1.0) as f64, 0.7, light as f64);
                cell.ch = glyph;
            } else {
                let halo_th = th * 3.0;
                let halo = (1.0 - dr.min(dc) / halo_th).max(0.0);
                if halo > 0.02 && look.halo > 0.0 {
                    let hue = look.hue + if equip { 90.0 } else { 0.0 };
                    cell.fg = hsl_to_rgb(
                        (hue / 360.0).rem_euclid(1.0) as f64,
                        0.5,
                        (look.halo * halo * 0.26) as f64,
                    );
                    cell.ch = if halo > 0.5 { '.' } else { ' ' };
                }
            }
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        grid.par_iter_mut().enumerate().for_each(paint);
    } else {
        grid.iter_mut().enumerate().for_each(paint);
    }
}

/// Advect flow comets along the streamlines and stamp them as bright beads.
fn paint_flow(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    PATH.with(|slot| {
        let mut path = slot.borrow_mut();
        if path.len() < NSTEP {
            path.resize(NSTEP, (0.0f32, 0.0f32));
        }
        let t = look.seed;
        for j in 0..look.tracers {
            let sx = unit(hash(t, L_TRAC, j as u64, 0)) * w as f32;
            let sy = unit(hash(t, L_TRAC, j as u64, 1)) * h as f32;
            let (mut a, mut b) = cell_to_plane(sx, sy, look, false);
            let ph = unit(hash(t, L_TRAC, j as u64, 2)) + look.time * look.flow;
            let r = ph - ph.floor();
            let head = TAIL + (r * (NSTEP - 1 - TAIL) as f32) as usize;
            for i in 0..NSTEP {
                let (_phi, _psi, re, im) = potential(a, b, look);
                let mag = (re * re + im * im).sqrt().max(1e-4);
                path[i] = plane_to_cell(a, b, look);
                let fa = look.flowa;
                let inv = 0.05 / mag;
                a += (re * (1.0 - fa) + im * fa) * inv;
                b += (-im * (1.0 - fa) + re * fa) * inv;
            }
            let (hx, hy) = path[head.min(NSTEP - 1)];
            stamp(grid, w, h, hx, hy, 'o', look.hue, 0.35, 0.85);
            for k in 1..=TAIL {
                if head >= k {
                    let (cx, cy) = path[head - k];
                    let l = 0.66 - 0.1 * k as f32;
                    stamp(grid, w, h, cx, cy, if k < 3 { '+' } else { '.' }, look.hue, 0.4, l as f64);
                }
            }
        }
    });
}

#[inline]
fn stamp(grid: &mut Grid, w: usize, h: usize, fx: f32, fy: f32, ch: char, hue: f32, s: f64, l: f64) {
    let x = fx.round();
    let y = fy.round();
    if x < 0.0 || y < 0.0 {
        return;
    }
    let (x, y) = (x as usize, y as usize);
    if x >= w || y >= h {
        return;
    }
    grid[y][x].ch = ch;
    grid[y][x].fg = hsl_to_rgb((hue / 360.0).rem_euclid(1.0) as f64, s, l);
}

#[inline]
fn orient(equip: bool, re: f32, im: f32, aspect: f32) -> f32 {
    if equip {
        let den = im * aspect;
        if den.abs() < 1e-3 {
            1e3
        } else {
            re / den
        }
    } else {
        let den = re * aspect;
        if den.abs() < 1e-3 {
            1e3
        } else {
            -im / den
        }
    }
}

#[inline]
fn glyph_for(slope: f32) -> char {
    let a = slope.abs();
    if a > 2.4 {
        '|'
    } else if a < 0.45 {
        '-'
    } else if slope > 0.0 {
        '\\'
    } else {
        '/'
    }
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
    fn streamnet_seed42() {
        insta::assert_snapshot!("streamnet_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn streamnet_seed42_t6() {
        insta::assert_snapshot!("streamnet_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_morphs_the_net() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 5.0, &k)));
    }

    #[test]
    fn poles_change_the_net() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[12] = 0.0;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn density_changes_the_net() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[2] = 7.0;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn form_changes_the_net() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[15] = 2.0;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn flowdir_moves_comets() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[16] = 1.0;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..100 {
            let t0 = std::time::Instant::now();
            frame(w, h, 42, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 100.0;
        eprintln!("streamnet frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
