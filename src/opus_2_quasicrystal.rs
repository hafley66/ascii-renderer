//! opus-2-quasicrystal -- de Bruijn multigrid quasilattice, a faceted growth front
//! blooming from the nucleus, phason drift, and a slow turn of the whole pencil.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::PI;

const MAXN: usize = 9;
const LEVELS: usize = 8;
const TYPES: usize = MAXN;
const MAXB: usize = 3;

const RAMP_SOLID: [char; LEVELS] = ['.', ':', ';', '=', '*', '8', '#', '%'];
const RAMP_STIPPLE: [char; LEVELS] = ['.', ',', ';', ':', 'o', 'O', '&', '@'];
const RAMP_HATCH: [char; LEVELS] = ['`', '\'', '"', '^', '~', '=', '%', '#'];

pub(crate) struct Opus2QuasicrystalKnobs {
    pub speed: f32,
    pub cycle: f32,
    pub folds: f32,
    pub scale: f32,
    pub linew: f32,
    pub band: f32,
    pub edge: f32,
    pub turn: f32,
    pub phason: f32,
    pub facet: f32,
    pub density: f32,
    pub dust: f32,
    pub hue: f32,
    pub glow: f32,
    pub aspect: f32,
    pub phase: f32,
    pub blooms: f32,
}

impl Opus2QuasicrystalKnobs {
    pub(crate) fn from_env() -> Self {
        Opus2QuasicrystalKnobs {
            speed: param_f32("SPEED", 1.0),
            cycle: param_f32("CYCLE", 130.0),
            folds: param_f32("FOLDS", 0.0),
            scale: param_f32("SCALE", 9.0),
            linew: param_f32("LINEW", 0.85),
            band: param_f32("BAND", 0.72),
            edge: param_f32("EDGE", 0.025),
            turn: param_f32("TURN", 0.22),
            phason: param_f32("PHASON", 0.012),
            facet: param_f32("FACET", 0.85),
            density: param_f32("DENSITY", 0.8),
            dust: param_f32("DUST", 0.3),
            hue: param_f32("HUE", 46.0),
            glow: param_f32("GLOW", 0.85),
            aspect: param_f32("ASPECT", 2.0),
            phase: param_f32("PHASE", 0.0),
            blooms: param_f32("BLOOMS", 2.0),
        }
    }

    fn folds_n(&self, seeded: usize) -> usize {
        let f = self.folds.round() as i32;
        if f < 4 { seeded } else { (f as usize).min(MAXN) }
    }
}

struct Cached {
    key: (u64, u32),
    n: usize,
    gamma0: [f32; MAXN],
    drift: [f32; MAXN],
    ramp: [char; LEVELS],
    base_hue: f32,
    nudge_x: f32,
    nudge_y: f32,
    phase0: f32,
    star: f32,
    scratch: Vec<u32>,
}

thread_local! {
    static CACHE: RefCell<Option<Cached>> = RefCell::new(None);
}

fn build(seed: u64, folds_knob: usize) -> Cached {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x0_9A5C_2117);
    let picks = [5usize, 5, 7, 5, 4, 7, 9, 6];
    let seeded = picks[rng.random_range(0..picks.len())];
    let n = if folds_knob >= 4 { folds_knob.min(MAXN) } else { seeded };
    let mut gamma0 = [0.0f32; MAXN];
    let mut drift = [0.0f32; MAXN];
    for j in 0..MAXN {
        gamma0[j] = rng.random::<f32>();
        drift[j] = rng.random::<f32>() * 2.0 - 1.0;
    }
    let ramp = match rng.random_range(0..3) {
        0 => RAMP_SOLID,
        1 => RAMP_STIPPLE,
        _ => RAMP_HATCH,
    };
    Cached {
        key: (seed, folds_knob as u32),
        n,
        gamma0,
        drift,
        ramp,
        base_hue: rng.random::<f32>() * 360.0,
        nudge_x: rng.random::<f32>() * 0.28 - 0.14,
        nudge_y: rng.random::<f32>() * 0.24 - 0.12,
        phase0: rng.random::<f32>(),
        star: rng.random::<f32>() * 0.5 + 0.5,
        scratch: Vec::new(),
    }
}

fn hash2(x: u32, y: u32, k: u32) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (k as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 32;
    (h & 0xFF_FFFF) as f32 / 16_777_216.0
}

fn smooth(s: f32) -> f32 {
    let s = s.clamp(0.0, 1.0);
    s * s * (3.0 - 2.0 * s)
}

fn frac(v: f32) -> f32 {
    v - v.floor()
}

/// Screen-space glyph for a line whose world direction is perpendicular to `theta`.
fn line_glyph(theta: f32, aspect: f32) -> char {
    let dx = -theta.sin() * aspect;
    let dy = theta.cos();
    let mut a = dy.atan2(dx);
    if a < 0.0 {
        a += PI;
    }
    let o = a / PI;
    if !(0.125..0.875).contains(&o) {
        '-'
    } else if o < 0.375 {
        '\\'
    } else if o < 0.625 {
        '|'
    } else {
        '/'
    }
}

pub(crate) fn draw_opus_2_quasicrystal(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &Opus2QuasicrystalKnobs) {
    let folds_knob = k.folds.round().clamp(0.0, MAXN as f32) as usize;
    CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let key = (seed, folds_knob as u32);
        let stale = slot.as_ref().map(|c| c.key != key).unwrap_or(true);
        if stale {
            *slot = Some(build(seed, folds_knob));
        }
        let c = slot.as_mut().unwrap();
        render(grid, w, h, seed, palette, t, k, c);
    });
}

struct Frame {
    n: usize,
    colstep: [f32; MAXN],
    rowstep: [f32; MAXN],
    gamma: [f32; MAXN],
    glyph: [char; MAXN],
    invthr: [f32; MAXN],
    ncx: f32,
    ncy: f32,
    inv_rmax: f32,
    nb: usize,
    q_out: [f32; MAXB],
    q_in: [f32; MAXB],
    edge_q: f32,
    melt_q: f32,
    moat_q: f32,
    vapor_q: f32,
    band: f32,
    edge: f32,
    facet: f32,
    linew: f32,
    density: f32,
    lvl: [f32; TYPES],
}

fn geometry(w: usize, h: usize, t: f32, k: &Opus2QuasicrystalKnobs, c: &Cached) -> Frame {
    let n = k.folds_n(c.n).clamp(4, MAXN);
    let aspect = k.aspect.clamp(0.25, 4.0);
    let d = k.scale.clamp(1.2, 40.0);
    let tt = if t > 0.0 { t * k.speed.clamp(0.0, 8.0) } else { 0.0 };
    let rot = tt * k.turn.clamp(-30.0, 30.0) * PI / 180.0;
    let ncx = w as f32 * (0.5 + c.nudge_x);
    let ncy = h as f32 * (0.5 + c.nudge_y);

    let mut f = Frame {
        n,
        colstep: [0.0; MAXN],
        rowstep: [0.0; MAXN],
        gamma: [0.0; MAXN],
        glyph: ['-'; MAXN],
        invthr: [1.0; MAXN],
        ncx,
        ncy,
        inv_rmax: 1.0,
        nb: (k.blooms.round() as usize).clamp(1, MAXB),
        q_out: [0.0; MAXB],
        q_in: [0.0; MAXB],
        edge_q: 0.02,
        melt_q: 0.3,
        moat_q: 0.02,
        vapor_q: 0.12,
        band: k.band.clamp(0.02, 1.0),
        edge: k.edge.clamp(0.004, 0.5),
        facet: k.facet.clamp(0.0, 1.0),
        linew: k.linew.clamp(0.05, 3.0),
        density: k.density.clamp(0.0, 1.0),
        lvl: [0.0; TYPES],
    };
    for ty in 0..TYPES {
        let a = frac(ty as f32 * 0.618_034);
        f.lvl[ty] = f.density * (0.16 + 0.84 * a);
    }
    for j in 0..n {
        let theta = PI * j as f32 / n as f32 + rot;
        f.colstep[j] = theta.cos() / (aspect * d);
        f.rowstep[j] = theta.sin() / d;
        f.gamma[j] = c.gamma0[j] + tt * k.phason.clamp(0.0, 4.0) * c.drift[j];
        f.glyph[j] = line_glyph(theta, aspect);
    }
    let wide = f.linew * (5.0 / n as f32).powf(0.55);
    for j in 0..n {
        let theta = PI * j as f32 / n as f32 + rot;
        let len = ((theta.cos() * aspect).powi(2) + theta.sin().powi(2)).sqrt().max(1e-3);
        f.invthr[j] = len * d / wide.max(0.05);
    }
    let mut rmax = 1.0f32;
    for (cx, cy) in [(0.0f32, 0.0f32), (w as f32, 0.0), (0.0, h as f32), (w as f32, h as f32)] {
        let (dx, dy) = (cx - ncx, cy - ncy);
        for j in 0..n {
            let v = dx * f.colstep[j] + dy * f.rowstep[j];
            rmax = rmax.max(v.abs());
        }
    }
    f.inv_rmax = 1.0 / (rmax * 1.02);
    let span = 1.0 + f.band;
    let base_phase = tt / k.cycle.clamp(0.5, 600.0) + c.phase0 + k.phase;
    for b in 0..f.nb {
        f.q_out[b] = frac(base_phase + b as f32 / f.nb as f32) * span;
        f.q_in[b] = f.q_out[b] - f.band;
    }
    f.edge_q = f.edge;
    f.melt_q = (f.band * 0.5).max(0.02);
    f.moat_q = f.edge * 1.3;
    f.vapor_q = (f.edge * 7.0).max(0.05);
    f
}

struct Paint {
    bg: Color,
    tile: [[Color; LEVELS]; TYPES],
    web: [Color; LEVELS],
    node: Color,
    rim: Color,
    melt: Color,
    dust: Color,
    star: Color,
}

fn paints(palette: &[Color; 5], k: &Opus2QuasicrystalKnobs, c: &Cached, n: usize, pulse: f32) -> Paint {
    let bg = palette[0];
    let spread = k.hue.clamp(0.0, 240.0) as f64;
    let base = c.base_hue as f64;
    let glow = k.glow.clamp(0.0, 1.5);
    let mut tile = [[bg; LEVELS]; TYPES];
    let _ = n;
    for ty in 0..TYPES {
        let scramble = (ty as f64 * 0.381_966).fract();
        let hue = base + spread * (scramble - 0.5);
        let full = shift_hue(palette[3], hue);
        for l in 0..LEVELS {
            let amt = 0.28 + 0.72 * (l as f32 + 1.0) / LEVELS as f32;
            tile[ty][l] = lerp_color(bg, full, amt);
        }
    }
    let web_full = lighten(shift_hue(palette[1], base + spread * 0.35), (30.0 * glow) as u8);
    let mut web = [bg; LEVELS];
    for l in 0..LEVELS {
        let amt = 0.3 + 0.7 * (l as f32 + 1.0) / LEVELS as f32;
        web[l] = lerp_color(bg, web_full, amt);
    }
    Paint {
        bg,
        tile,
        web,
        node: lighten(palette[4], (40.0 * glow) as u8),
        rim: lighten(shift_hue(palette[3], base + 40.0), (70.0 * glow) as u8),
        melt: darken(shift_hue(palette[2], base - 30.0), 30),
        dust: darken(palette[2], 70),
        star: lerp_color(palette[3], palette[4], 0.35 + 0.5 * pulse),
    }
}

fn render(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &Opus2QuasicrystalKnobs, c: &mut Cached) {
    measure_layer("opus-2-quasicrystal", "clear", || {
        for row in grid.iter_mut().take(h) {
            for cell in row.iter_mut().take(w) {
                *cell = Cell::blank();
            }
        }
    });
    if w < 8 || h < 4 {
        return;
    }
    let f = geometry(w, h, t, k, c);
    let tt = if t > 0.0 { t * k.speed.clamp(0.0, 8.0) } else { 0.0 };
    let pulse = 0.5 + 0.5 * (tt * 0.18).sin();
    let p = paints(palette, k, c, f.n, pulse);
    let ramp = c.ramp;
    let seed32 = (seed ^ (seed >> 32)) as u32;
    if c.scratch.len() != w * h {
        c.scratch.resize(w * h, 0);
    }
    let scratch = &mut c.scratch;
    let twinkle = ((tt * 0.35).floor() as i64 as u32) ^ seed32;
    let ghost = k.dust.clamp(0.0, 1.0);

    measure_layer("opus-2-quasicrystal", "lattice", || {
        lattice_pass(grid, w, h, &f, &p, &ramp, seed32, twinkle, ghost, scratch);
    });
    measure_layer("opus-2-quasicrystal", "fronts", || {
        fronts_pass(grid, w, h, &p, k, twinkle, scratch);
    });
    measure_layer("opus-2-quasicrystal", "nucleus", || {
        nucleus_pass(grid, w, h, &f, &p, c.star, pulse, k);
    });
}

fn lattice_pass(grid: &mut Grid, w: usize, h: usize, f: &Frame, p: &Paint, ramp: &[char; LEVELS], seed32: u32, twinkle: u32, ghost: f32, scratch: &mut [u32]) {
    let n = f.n;
    let inv_edge = 1.0 / f.edge_q;
    let inv_melt = 1.0 / f.melt_q;
    let inv_vapor = 1.0 / f.vapor_q;
    let melt_w = (f.edge_q * 1.6).max(0.008);
    for y in 0..h {
        let mut u = [0.0f32; MAXN];
        let mut v = [0.0f32; MAXN];
        let mut kk = [0i32; MAXN];
        let mut fr = [0.0f32; MAXN];
        let dy = y as f32 + 0.5 - f.ncy;
        let dx0 = 0.5 - f.ncx;
        for j in 0..n {
            v[j] = dx0 * f.colstep[j] + dy * f.rowstep[j];
            u[j] = v[j] + f.gamma[j];
            let fl = u[j].floor();
            kk[j] = fl as i32;
            fr[j] = u[j] - fl;
        }
        let row = &mut grid[y];
        let base = y * w;
        for x in 0..w {
            let mut d1 = 1.0e9f32;
            let mut d2 = 1.0e9f32;
            let mut i1 = 0usize;
            let mut rad = 0.0f32;
            let mut sum = 0.0f32;
            let mut ksum = 0i32;
            let mut hsh = seed32;
            for j in 0..n {
                let g = fr[j];
                let raw = if g < 0.5 { g } else { 1.0 - g };
                let dist = raw * f.invthr[j];
                if dist < d1 {
                    d2 = d1;
                    d1 = dist;
                    i1 = j;
                } else if dist < d2 {
                    d2 = dist;
                }
                let av = v[j].abs();
                if av > rad {
                    rad = av;
                }
                sum += v[j] * v[j];
                ksum = ksum.wrapping_add(kk[j]);
                hsh = hsh.wrapping_mul(0x9E37_79B1).wrapping_add(kk[j] as u32);
                let nf = fr[j] + f.colstep[j];
                let fl = nf.floor();
                fr[j] = nf - fl;
                kk[j] += fl as i32;
                v[j] += f.colstep[j];
            }
            let round = (sum * 2.0 / n as f32).sqrt();
            let q = (rad * f.facet + round * (1.0 - f.facet)) * f.inv_rmax;

            let mut formed = 0.0f32;
            let mut lead = 0.0f32;
            let mut melt = 0.0f32;
            let mut gate = 0.0f32;
            let mut moat = false;
            for b in 0..f.nb {
                let dq = f.q_out[b] - q;
                let dm = q - f.q_in[b];
                let fm = smooth(dq * inv_edge) * smooth(dm * inv_melt);
                if fm > formed {
                    formed = fm;
                }
                if dq > 0.0 && dq < f.edge_q {
                    let g = dq * inv_edge;
                    lead = lead.max(4.0 * g * (1.0 - g));
                } else if dq < 0.0 {
                    if dq > -f.moat_q {
                        moat = true;
                    }
                    gate = gate.max((1.0 + dq * inv_vapor).clamp(0.0, 1.0));
                }
                if dm > 0.0 && dm < melt_w {
                    let g = dm / melt_w;
                    melt = melt.max(4.0 * g * (1.0 - g));
                }
            }
            scratch[base + x] = (lead * 255.0) as u32
                | ((melt * 255.0) as u32) << 8
                | ((gate * 255.0) as u32) << 16
                | (moat as u32) << 24;

            if formed <= 0.02 {
                let (thr, prob) = if gate > 0.0 {
                    let gf = gate.max(0.13);
                    (0.5 + 1.7 * gf, ghost * gf * gf)
                } else {
                    (0.45, ghost * 0.09)
                };
                if !moat && prob > 0.0 && d1 < thr && hash2(x as u32, y as u32, twinkle) < prob {
                    row[x] = Cell::new(f.glyph[i1], p.dust);
                } else {
                    row[x] = Cell::new(' ', p.bg);
                }
                continue;
            }
            let wthr = 0.4 + 0.6 * formed;
            if d1 < wthr {
                if d2 < wthr * 1.35 {
                    row[x] = Cell::new(if d1 < wthr * 0.45 { '+' } else { 'x' }, p.node);
                } else {
                    let lv = ((0.45 + 0.55 * formed) * (LEVELS as f32 - 0.01)) as usize;
                    row[x] = Cell::new(f.glyph[i1], p.web[lv]);
                }
                continue;
            }
            let ty = (ksum.rem_euclid(n as i32) as usize).min(TYPES - 1);
            let shade = ((hsh >> 11) & 0x7) as f32 / 7.0;
            let level = f.lvl[ty] * (0.72 + 0.56 * shade) * formed;
            let li = ((level * LEVELS as f32) as usize).min(LEVELS - 1);
            row[x] = Cell::new(ramp[li], p.tile[ty][li]);
        }
    }
}

fn fronts_pass(grid: &mut Grid, w: usize, h: usize, p: &Paint, k: &Opus2QuasicrystalKnobs, twinkle: u32, scratch: &[u32]) {
    let dust = k.dust.clamp(0.0, 1.0) * 0.4;
    let glow = k.glow.clamp(0.0, 1.5);
    for y in 0..h {
        let row = &mut grid[y];
        let base = y * w;
        for x in 0..w {
            let packed = scratch[base + x];
            let lead = (packed & 0xFF) as f32 / 255.0 * glow;
            if lead > 0.5 {
                let ch = if lead > 0.92 { '@' } else if lead > 0.74 { '#' } else { '%' };
                row[x] = Cell::new(ch, p.rim);
                continue;
            }
            if (packed >> 24) & 1 == 1 {
                row[x] = Cell::new(' ', p.bg);
                continue;
            }
            if row[x].ch != ' ' {
                continue;
            }
            let melt = ((packed >> 8) & 0xFF) as f32 / 255.0 * glow;
            if melt > 0.4 {
                row[x] = Cell::new(if melt > 0.8 { ':' } else { '.' }, p.melt);
                continue;
            }
            let gate = ((packed >> 16) & 0xFF) as f32 / 255.0;
            if dust > 0.0 && gate > 0.0 {
                let u = hash2(x as u32, y as u32, twinkle);
                if u < dust * gate * gate {
                    let v = hash2(x as u32, y as u32, twinkle ^ 0x5BF0_3A17);
                    let ch = if v < 0.5 { '.' } else if v < 0.82 { '`' } else { ',' };
                    row[x] = Cell::new(ch, p.dust);
                }
            }
        }
    }
}

fn nucleus_pass(grid: &mut Grid, w: usize, h: usize, f: &Frame, p: &Paint, star: f32, pulse: f32, k: &Opus2QuasicrystalKnobs) {
    let glow = k.glow.clamp(0.0, 1.5);
    if glow <= 0.01 {
        return;
    }
    let aspect = k.aspect.clamp(0.25, 4.0);
    let reach = ((2.5 + 5.5 * star * (0.55 + 0.45 * pulse)) * glow).clamp(1.0, 14.0);
    let cx = f.ncx;
    let cy = f.ncy;
    let clear_r = reach * 0.55 + 1.0;
    let y0 = (cy - clear_r).floor().max(0.0) as usize;
    let y1 = ((cy + clear_r).ceil() as usize).min(h.saturating_sub(1));
    let x0 = (cx - clear_r * aspect).floor().max(0.0) as usize;
    let x1 = ((cx + clear_r * aspect).ceil() as usize).min(w.saturating_sub(1));
    for y in y0..=y1 {
        let dy = y as f32 + 0.5 - cy;
        for x in x0..=x1 {
            let dx = (x as f32 + 0.5 - cx) / aspect;
            if dx * dx + dy * dy < clear_r * clear_r {
                grid[y][x] = Cell::new(' ', p.bg);
            }
        }
    }
    for j in 0..f.n {
        let theta = PI * j as f32 / f.n as f32;
        for sign in [1.0f32, -1.0] {
            let (dx, dy) = (-theta.sin() * aspect * sign, theta.cos() * sign);
            let steps = reach as i32;
            for i in 1..=steps {
                let px = (cx + dx * i as f32).round() as i32;
                let py = (cy + dy * i as f32).round() as i32;
                if px < 0 || py < 0 || px as usize >= w || py as usize >= h {
                    break;
                }
                let fade = 1.0 - (i as f32 - 1.0) / steps.max(1) as f32;
                let ch = if i == steps { '*' } else { f.glyph[j] };
                grid[py as usize][px as usize] = Cell::new(ch, lerp_color(p.bg, p.star, 0.4 + 0.6 * fade));
            }
        }
    }
    let px = cx.round() as i32;
    let py = cy.round() as i32;
    if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
        let ch = if pulse > 0.66 { '@' } else if pulse > 0.33 { '*' } else { 'o' };
        grid[py as usize][px as usize] = Cell::new(ch, p.star);
    }
}

pub(crate) fn cli_opus_2_quasicrystal(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = Opus2QuasicrystalKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 17] = [
        &mut k.speed,
        &mut k.cycle,
        &mut k.folds,
        &mut k.scale,
        &mut k.linew,
        &mut k.band,
        &mut k.edge,
        &mut k.turn,
        &mut k.phason,
        &mut k.facet,
        &mut k.density,
        &mut k.dust,
        &mut k.hue,
        &mut k.glow,
        &mut k.aspect,
        &mut k.phase,
        &mut k.blooms,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_opus_2_quasicrystal(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = Opus2QuasicrystalKnobs::from_env();
        draw_opus_2_quasicrystal(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_opus_2_quasicrystal_small() {
        insta::assert_snapshot!("opus_2_quasicrystal_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_opus_2_quasicrystal_moving() {
        insta::assert_snapshot!("opus_2_quasicrystal_110x36_t45", run(110, 36, 7, 45.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn t_advances_the_front() {
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 20.0));
        assert_ne!(run(90, 30, 42, 20.0), run(90, 30, 42, 60.0));
    }

    #[test]
    fn every_fold_order_terminates() {
        for folds in 4..=MAXN {
            let mut g = vec![vec![Cell::blank(); 60]; 20];
            let p = crate::color::make_palette(3);
            let mut k = Opus2QuasicrystalKnobs::from_env();
            k.folds = folds as f32;
            draw_opus_2_quasicrystal(&mut g, 60, 20, 3, &p, 5.0, &k);
        }
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = Opus2QuasicrystalKnobs::from_env();
        draw_opus_2_quasicrystal(&mut g, w, h, 42, &p, 0.0, &k);
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for fi in 0..200 {
            let t0 = std::time::Instant::now();
            draw_opus_2_quasicrystal(&mut g, w, h, 42, &p, fi as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("opus-2-quasicrystal frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }
}
