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

const MAXN: usize = 7;
const LEVELS: usize = 8;
const TYPES: usize = MAXN;
const MAXB: usize = 3;
const RDIRS: usize = 16;
const RSEC: f32 = 1.004_82;
const INV12: f32 = 1.0 / 4096.0;
const LUT_Q: f32 = 1.25;
const MIX: [u32; MAXN] = [0x9E37_79B1, 0x85EB_CA6B, 0xC2B2_AE35, 0x27D4_EB2F, 0x1656_67B1, 0xD3A2_646C, 0xFD70_46C5];

#[derive(Clone, Copy)]
struct FrontLut {
    pack: u32,
    formed: f32,
}

fn build_lut(f: &Frame, out: &mut Vec<FrontLut>, n: usize) {
    out.clear();
    out.reserve(n);
    let inv_edge = 1.0 / f.edge_q;
    let inv_melt = 1.0 / f.melt_q;
    let inv_vapor = 1.0 / f.vapor_q;
    let inv_meltw = 1.0 / (f.edge_q * 1.6).max(0.008);
    let scale = LUT_Q / (n - 1) as f32;
    for i in 0..n {
        let q = i as f32 * scale;
        let mut formed = 0.0f32;
        let mut lead = 0.0f32;
        let mut melt = 0.0f32;
        let mut gate = 0.0f32;
        let mut moat = false;
        for b in 0..f.nb {
            let dq = f.q_out[b] - q;
            let dm = q - f.q_in[b];
            formed = formed.max(smooth(dq * inv_edge) * smooth(dm * inv_melt));
            if dq > 0.0 {
                if dq < f.edge_q {
                    let g = dq * inv_edge;
                    lead = lead.max(4.0 * g * (1.0 - g));
                }
            } else {
                moat |= dq > -f.moat_q;
                gate = gate.max((1.0 + dq * inv_vapor).clamp(0.0, 1.0));
            }
            let g = dm * inv_meltw;
            if dm > 0.0 && g < 1.0 {
                melt = melt.max(4.0 * g * (1.0 - g));
            }
        }
        let pack = (lead * 255.0) as u32
            | ((melt * 255.0) as u32) << 8
            | ((gate * 255.0) as u32) << 16
            | (moat as u32) << 24;
        out.push(FrontLut { pack, formed });
    }
}


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
    lut: Vec<FrontLut>,
}

thread_local! {
    static CACHE: RefCell<Option<Cached>> = RefCell::new(None);
}

fn build(seed: u64, folds_knob: usize) -> Cached {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x0_9A5C_2117);
    let picks = [5usize, 5, 7, 5, 4, 7, 6, 5];
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
        lut: Vec::new(),
    }
}

fn hash_bits(x: u32, y: u32, k: u32) -> u32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (k as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 32;
    h as u32
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
    aspect: f32,
    spacing: f32,
    rcol: [f32; RDIRS],
    rrow: [f32; RDIRS],
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
        aspect,
        spacing: d,
        rcol: [0.0; RDIRS],
        rrow: [0.0; RDIRS],
    };
    for i in 0..RDIRS {
        let th = PI * i as f32 / RDIRS as f32;
        f.rcol[i] = th.cos() / (aspect * d);
        f.rrow[i] = th.sin() / d;
    }
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
    let ramp = c.ramp;
    let seed32 = (seed ^ (seed >> 32)) as u32;
    let lut_n = (4096).min((w * h / 2).max(256));
    let p = measure_layer("opus-2-quasicrystal", "fronts", || {
        if c.lut.len() != lut_n {
            c.lut.clear();
        }
        build_lut(&f, &mut c.lut, lut_n);
        paints(palette, k, c, f.n, pulse)
    });
    let lut = &c.lut;
    let twinkle = ((tt * 0.35).floor() as i64 as u32) ^ seed32;

    measure_layer("opus-2-quasicrystal", "lattice", || {
        lattice_pass(grid, w, h, &f, &p, k, &ramp, seed32, twinkle, lut);
    });
    measure_layer("opus-2-quasicrystal", "nucleus", || {
        nucleus_pass(grid, w, h, &f, &p, c.star, pulse, k);
    });
}

/// Upper envelope of the 2N lines +/-(A_j + B_j x) that define the polygonal
/// radius along one row. Walked left to right, one fma per column.
struct Envelope {
    m: [f32; 2 * RDIRS],
    b: [f32; 2 * RDIRS],
    brk: [f32; 2 * RDIRS],
    len: usize,
    at: usize,
}

impl Envelope {
    fn build(a: &[f32], bslope: &[f32], n: usize) -> Self {
        let mut m = [0.0f32; 2 * RDIRS];
        let mut b = [0.0f32; 2 * RDIRS];
        let mut cnt = 0usize;
        for j in 0..n {
            m[cnt] = bslope[j];
            b[cnt] = a[j];
            cnt += 1;
            m[cnt] = -bslope[j];
            b[cnt] = -a[j];
            cnt += 1;
        }
        for i in 1..cnt {
            let (km, kb) = (m[i], b[i]);
            let mut j = i;
            while j > 0 && (m[j - 1] > km || (m[j - 1] == km && b[j - 1] < kb)) {
                m[j] = m[j - 1];
                b[j] = b[j - 1];
                j -= 1;
            }
            m[j] = km;
            b[j] = kb;
        }
        let mut hm = [0.0f32; 2 * RDIRS];
        let mut hb = [0.0f32; 2 * RDIRS];
        let mut len = 0usize;
        for i in 0..cnt {
            if len > 0 && hm[len - 1] == m[i] {
                continue;
            }
            while len >= 2 {
                let x_new = (b[i] - hb[len - 2]) / (hm[len - 2] - m[i]);
                let x_old = (hb[len - 1] - hb[len - 2]) / (hm[len - 2] - hm[len - 1]);
                if x_new <= x_old {
                    len -= 1;
                } else {
                    break;
                }
            }
            hm[len] = m[i];
            hb[len] = b[i];
            len += 1;
        }
        let mut brk = [f32::INFINITY; 2 * RDIRS];
        for i in 0..len.saturating_sub(1) {
            brk[i] = (hb[i + 1] - hb[i]) / (hm[i] - hm[i + 1]);
        }
        brk[len.saturating_sub(1)] = f32::INFINITY;
        Envelope { m: hm, b: hb, brk, len: len.max(1), at: 0 }
    }

    #[inline(always)]
    fn at(&mut self, x: f32) -> f32 {
        while self.at + 1 < self.len && x >= self.brk[self.at] {
            self.at += 1;
        }
        self.m[self.at] * x + self.b[self.at]
    }
}

fn lattice_pass(grid: &mut Grid, w: usize, h: usize, f: &Frame, p: &Paint, k: &Opus2QuasicrystalKnobs, ramp: &[char; LEVELS], seed32: u32, twinkle: u32, lut: &[FrontLut]) {
    match f.n {
        0..=4 => lattice_rows::<4>(grid, w, h, f, p, k, ramp, seed32, twinkle, lut),
        5 => lattice_rows::<5>(grid, w, h, f, p, k, ramp, seed32, twinkle, lut),
        6 => lattice_rows::<6>(grid, w, h, f, p, k, ramp, seed32, twinkle, lut),
        _ => lattice_rows::<7>(grid, w, h, f, p, k, ramp, seed32, twinkle, lut),
    }
}

fn lattice_rows<const N: usize>(grid: &mut Grid, w: usize, h: usize, f: &Frame, p: &Paint, k: &Opus2QuasicrystalKnobs, ramp: &[char; LEVELS], seed32: u32, twinkle: u32, lut: &[FrontLut]) {
    let facet = f.facet;
    let one_facet = 1.0 - f.facet;
    let inv_rmax = f.inv_rmax;
    let lut_scale = (lut.len() - 1) as f32 / LUT_Q;
    let lut_top = lut.len() - 1;
    let ghost = k.dust.clamp(0.0, 1.0);
    let ghost_low = ghost * 0.09;
    let dust = ghost * 0.4;
    let glow = k.glow.clamp(0.0, 1.5) * (1.0 / 255.0);
    let inv255 = 1.0 / 255.0;
    let mut step = [0.0f32; N];
    let mut invthr = [0.0f32; N];
    let mut glyph = [' '; N];
    let mut mix = [0u32; N];
    for j in 0..N {
        step[j] = f.colstep[j];
        invthr[j] = f.invthr[j];
        glyph[j] = f.glyph[j];
        mix[j] = MIX[j];
    }
    for y in 0..h {
        let mut a0 = [0.0f32; N];
        let mut fr = [0.0f32; N];
        let mut ksum = 0i32;
        let mut hsh = seed32;
        let dy = y as f32 + 0.5 - f.ncy;
        let dx0 = 0.5 - f.ncx;
        for j in 0..N {
            a0[j] = dx0 * step[j] + dy * f.rowstep[j];
            let u = a0[j] + f.gamma[j];
            let fl = u.floor();
            let kj = fl as i32;
            ksum = ksum.wrapping_add(kj);
            hsh = hsh.wrapping_add((kj as u32).wrapping_mul(mix[j]));
            fr[j] = u - fl;
        }
        let mut env = Envelope::build(&a0, &step, N);
        let mut renv = if one_facet > 0.0 {
            let mut ra = [0.0f32; RDIRS];
            for i in 0..RDIRS {
                ra[i] = dx0 * f.rcol[i] + dy * f.rrow[i];
            }
            Envelope::build(&ra, &f.rcol, RDIRS)
        } else {
            Envelope::build(&a0, &step, N)
        };
        let row = &mut grid[y][..w];
        for (x, cell) in row.iter_mut().enumerate() {
            let rad = env.at(x as f32);
            let q = if one_facet > 0.0 {
                let round = renv.at(x as f32) * RSEC;
                (rad * facet + round * one_facet) * inv_rmax
            } else {
                rad * inv_rmax
            };
            let e = lut[((q * lut_scale) as usize).min(lut_top)];
            let pack = e.pack;
            let formed = e.formed;

            let mut quick = None;
            let lead = (pack & 0xFF) as f32 * glow;
            if lead > 0.5 {
                let ch = if lead > 0.92 { '@' } else if lead > 0.74 { '#' } else { '%' };
                quick = Some(Cell::new(ch, p.rim));
            } else if (pack >> 24) & 1 == 1 {
                quick = Some(Cell::new(' ', p.bg));
            } else if formed <= 0.02 {
                let gate = ((pack >> 16) & 0xFF) as f32 * inv255;
                let prob = if gate > 0.0 { ghost * gate.max(0.13) * gate.max(0.13) } else { ghost_low };
                let hb = hash_bits(x as u32, y as u32, twinkle);
                if prob <= 0.0 || (hb & 0xFFF) as f32 * INV12 >= prob {
                    let melt = ((pack >> 8) & 0xFF) as f32 * glow;
                    quick = Some(if melt > 0.4 {
                        Cell::new(if melt > 0.8 { ':' } else { '.' }, p.melt)
                    } else if dust > 0.0 && gate > 0.0 && ((hb >> 12) & 0xFFF) as f32 * INV12 < dust * gate * gate {
                        let v = (hb >> 24) as f32 * (1.0 / 256.0);
                        let ch = if v < 0.5 { '.' } else if v < 0.82 { '`' } else { ',' };
                        Cell::new(ch, p.dust)
                    } else {
                        Cell::new(' ', p.bg)
                    });
                }
            }
            if let Some(c) = quick {
                *cell = c;
                for j in 0..N {
                    let nf = fr[j] + step[j];
                    let fl = nf.floor();
                    fr[j] = nf - fl;
                    let dk = fl as i32;
                    ksum = ksum.wrapping_add(dk);
                    hsh = hsh.wrapping_add((dk as u32).wrapping_mul(mix[j]));
                }
                continue;
            }

            let mut d1 = f32::INFINITY;
            let mut d2 = f32::INFINITY;
            let mut i1 = 0usize;
            for j in 0..N {
                let g = fr[j];
                let dist = g.min(1.0 - g) * invthr[j];
                let better = dist < d1;
                d2 = if better { d1 } else { d2.min(dist) };
                i1 = if better { j } else { i1 };
                d1 = if better { dist } else { d1 };
                let nf = g + step[j];
                let fl = nf.floor();
                fr[j] = nf - fl;
                let dk = fl as i32;
                ksum = ksum.wrapping_add(dk);
                hsh = hsh.wrapping_add((dk as u32).wrapping_mul(mix[j]));
            }

            if formed <= 0.02 {
                let gate = ((pack >> 16) & 0xFF) as f32 * inv255;
                let thr = if gate > 0.0 { 0.5 + 1.7 * gate.max(0.13) } else { 0.45 };
                *cell = if d1 < thr { Cell::new(glyph[i1], p.dust) } else { Cell::new(' ', p.bg) };
                continue;
            }
            let wthr = 0.4 + 0.6 * formed;
            if d1 < wthr {
                *cell = if d2 < wthr * 1.35 {
                    Cell::new(if d1 < wthr * 0.45 { '+' } else { 'x' }, p.node)
                } else {
                    let lv = (((0.45 + 0.55 * formed) * LEVELS as f32) as usize).min(LEVELS - 1);
                    Cell::new(glyph[i1], p.web[lv])
                };
                continue;
            }
            let ty = (((ksum.wrapping_add(0x0010_0000) as u32) % N as u32) as usize).min(TYPES - 1);
            let shade = ((hsh >> 11) & 0x7) as f32 * (1.0 / 7.0);
            let level = f.lvl[ty] * (0.72 + 0.56 * shade) * formed;
            let li = ((level * LEVELS as f32) as usize).min(LEVELS - 1);
            *cell = Cell::new(ramp[li], p.tile[ty][li]);
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
    fn envelope_matches_brute_force() {
        let mut worst = 0.0f32;
        for trial in 0..40u32 {
            let n = 4 + (trial % (MAXN as u32 - 3)) as usize;
            let mut a = [0.0f32; MAXN];
            let mut b = [0.0f32; MAXN];
            for j in 0..n {
                let t = (trial * 7 + j as u32 * 13) as f32;
                a[j] = (t * 0.37).sin() * 40.0;
                b[j] = (t * 0.11).cos() * 0.4;
            }
            let mut env = Envelope::build(&a, &b, n);
            for x in 0..300 {
                let xf = x as f32;
                let mut want = 0.0f32;
                for j in 0..n {
                    want = want.max((a[j] + b[j] * xf).abs());
                }
                worst = worst.max((env.at(xf) - want).abs());
            }
        }
        assert!(worst < 1e-3, "envelope drift {worst}");
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
