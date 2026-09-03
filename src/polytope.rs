//! polytope -- a regular 4D polytope turning in two or three incommensurate planes,
//! projected 4D to 3D to 2D, wireframe over a receding floor with a shadow and vertex trails.
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

const PHI: f32 = 1.618_034;
const WB: usize = 12;
const ZB: usize = 8;
const PLANE_NAMES: [&str; 6] = ["xy", "xz", "xw", "yz", "yw", "zw"];
const PLANE_AXES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];

pub(crate) struct PolytopeKnobs {
    pub poly: f32,
    pub speed: f32,
    pub planes: f32,
    pub fov: f32,
    pub zoom: f32,
    pub style: f32,
    pub floor: f32,
    pub trail: f32,
    pub tail: f32,
    pub hue: f32,
    pub span: f32,
    pub orbit: f32,
    pub pitch: f32,
    pub tile: f32,
    pub flow: f32,
    pub glow: f32,
    pub label: f32,
    pub cam: f32,
    pub aspect: f32,
    pub cull: f32,
    pub inset: f32,
}

impl PolytopeKnobs {
    pub(crate) fn from_env() -> Self {
        PolytopeKnobs {
            poly: param_f32("POLY", 0.0),
            speed: param_f32("SPEED", 40.0),
            planes: param_f32("PLANES", 3.0),
            fov: param_f32("FOV", 3.0),
            zoom: param_f32("ZOOM", 0.46),
            style: param_f32("STYLE", 0.0),
            floor: param_f32("FLOOR", 0.8),
            trail: param_f32("TRAIL", 40.0),
            tail: param_f32("TAIL", 0.3),
            hue: param_f32("HUE", 0.0),
            span: param_f32("SPAN", 100.0),
            orbit: param_f32("ORBIT", 1.5),
            pitch: param_f32("PITCH", 22.0),
            tile: param_f32("TILE", 0.6),
            flow: param_f32("FLOW", 0.08),
            glow: param_f32("GLOW", 1.0),
            label: param_f32("LABEL", 1.0),
            cam: param_f32("CAM", 4.5),
            aspect: param_f32("ASPECT", 2.0),
            cull: param_f32("CULL", 0.35),
            inset: param_f32("INSET", 0.2),
        }
    }

    fn poly_n(&self) -> u32 {
        (self.poly.round() as u32).min(7)
    }

    fn planes_n(&self) -> u32 {
        (self.planes.round() as u32).clamp(1, 3)
    }

    fn trail_n(&self) -> usize {
        (self.trail.round().max(0.0) as usize).min(240)
    }
}

struct Poly {
    name: &'static str,
    schlafli: String,
    verts: Vec<[f32; 4]>,
    edges: Vec<(u16, u16)>,
}

#[derive(Clone, Copy)]
struct Plane {
    axes: (usize, usize),
    rate: f32,
    phase: f32,
    name: &'static str,
}

#[derive(Clone, Copy, Default)]
struct Scr {
    sx: f32,
    sy: f32,
    near: f32,
    w: f32,
    ok: bool,
}

struct Cached {
    key: (u64, u32, u32),
    poly: Poly,
    planes: Vec<Plane>,
    hue_seed: f32,
    style_seed: u32,
    yaw0: f32,
    p3: Vec<[f32; 3]>,
    scr: Vec<Scr>,
    prev: Vec<Scr>,
    cur: Vec<Scr>,
    zbuf: Vec<f32>,
    ebuf: Vec<u16>,
    tglow: Vec<f32>,
    tw: Vec<f32>,
    crossings: Vec<(usize, usize)>,
    buf_dims: (usize, usize),
    lut: Vec<Color>,
}

thread_local! {
    static CACHE: RefCell<Option<Cached>> = RefCell::new(None);
}

fn norm4(v: &mut [f32; 4]) {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + v[3] * v[3]).sqrt().max(1e-6);
    for c in v.iter_mut() {
        *c /= n;
    }
}

fn dist2(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    (0..4).map(|i| (a[i] - b[i]) * (a[i] - b[i])).sum()
}

fn edges_nearest(verts: &[[f32; 4]]) -> Vec<(u16, u16)> {
    let mut best = f32::MAX;
    for i in 0..verts.len() {
        for j in i + 1..verts.len() {
            let d = dist2(&verts[i], &verts[j]);
            if d > 1e-6 && d < best {
                best = d;
            }
        }
    }
    let lim = best * 1.02;
    let mut edges = Vec::new();
    for i in 0..verts.len() {
        for j in i + 1..verts.len() {
            if dist2(&verts[i], &verts[j]) < lim {
                edges.push((i as u16, j as u16));
            }
        }
    }
    edges
}

fn sgn(mask: usize, bit: usize) -> f32 {
    if (mask >> bit) & 1 == 1 { -1.0 } else { 1.0 }
}

fn verts_600() -> Vec<[f32; 4]> {
    let mut v = Vec::with_capacity(120);
    for i in 0..4 {
        for s in [-1.0, 1.0] {
            let mut p = [0.0; 4];
            p[i] = s;
            v.push(p);
        }
    }
    for m in 0..16 {
        v.push([sgn(m, 0) * 0.5, sgn(m, 1) * 0.5, sgn(m, 2) * 0.5, sgn(m, 3) * 0.5]);
    }
    let base = [PHI / 2.0, 0.5, 1.0 / (2.0 * PHI), 0.0];
    let even: [[usize; 4]; 12] = [
        [0, 1, 2, 3], [0, 2, 3, 1], [0, 3, 1, 2], [1, 0, 3, 2], [1, 2, 0, 3], [1, 3, 2, 0],
        [2, 0, 1, 3], [2, 1, 3, 0], [2, 3, 0, 1], [3, 0, 2, 1], [3, 1, 0, 2], [3, 2, 1, 0],
    ];
    for perm in even {
        for m in 0..8 {
            let mut p = [0.0; 4];
            let mut bit = 0;
            for (slot, &src) in perm.iter().enumerate() {
                if src != 3 {
                    p[slot] = sgn(m, bit) * base[src];
                    bit += 1;
                }
            }
            v.push(p);
        }
    }
    v
}

fn verts_120() -> Vec<[f32; 4]> {
    let v = verts_600();
    let e = edges_nearest(&v);
    let n = v.len();
    let mut adj = vec![false; n * n];
    for &(a, b) in &e {
        adj[a as usize * n + b as usize] = true;
        adj[b as usize * n + a as usize] = true;
    }
    let mut out = Vec::with_capacity(600);
    for i in 0..n {
        let nb: Vec<usize> = (i + 1..n).filter(|&j| adj[i * n + j]).collect();
        for a in 0..nb.len() {
            for b in a + 1..nb.len() {
                if !adj[nb[a] * n + nb[b]] {
                    continue;
                }
                for c in b + 1..nb.len() {
                    if adj[nb[a] * n + nb[c]] && adj[nb[b] * n + nb[c]] {
                        let mut p = [0.0; 4];
                        for q in [i, nb[a], nb[b], nb[c]] {
                            for k in 0..4 {
                                p[k] += v[q][k] * 0.25;
                            }
                        }
                        out.push(p);
                    }
                }
            }
        }
    }
    out
}

fn make_poly(choice: u32, p: usize, q: usize) -> Poly {
    let (name, schlafli, mut verts, edges): (&str, String, Vec<[f32; 4]>, Option<Vec<(u16, u16)>>) = match choice {
        1 => {
            let r5 = 5f32.sqrt();
            let v = vec![
                [1.0, 1.0, 1.0, -1.0 / r5],
                [1.0, -1.0, -1.0, -1.0 / r5],
                [-1.0, 1.0, -1.0, -1.0 / r5],
                [-1.0, -1.0, 1.0, -1.0 / r5],
                [0.0, 0.0, 0.0, 4.0 / r5],
            ];
            let mut e = Vec::new();
            for i in 0..5u16 {
                for j in i + 1..5 {
                    e.push((i, j));
                }
            }
            ("5-cell", "{3,3,3}".to_string(), v, Some(e))
        }
        2 => {
            let v: Vec<[f32; 4]> = (0..16).map(|m| [sgn(m, 0), sgn(m, 1), sgn(m, 2), sgn(m, 3)]).collect();
            ("tesseract", "{4,3,3}".to_string(), v, None)
        }
        3 => {
            let mut v = Vec::new();
            for i in 0..4 {
                for s in [-1.0, 1.0] {
                    let mut p = [0.0; 4];
                    p[i] = s;
                    v.push(p);
                }
            }
            ("16-cell", "{3,3,4}".to_string(), v, None)
        }
        4 => {
            let mut v = Vec::new();
            for i in 0..4 {
                for j in i + 1..4 {
                    for m in 0..4 {
                        let mut p = [0.0; 4];
                        p[i] = sgn(m, 0);
                        p[j] = sgn(m, 1);
                        v.push(p);
                    }
                }
            }
            ("24-cell", "{3,4,3}".to_string(), v, None)
        }
        5 => ("600-cell", "{3,3,5}".to_string(), verts_600(), None),
        6 => ("120-cell", "{5,3,3}".to_string(), verts_120(), None),
        _ => {
            let mut v = Vec::with_capacity(p * q);
            let mut e = Vec::with_capacity(2 * p * q);
            for i in 0..p {
                let a = 2.0 * PI * i as f32 / p as f32;
                for j in 0..q {
                    let b = 2.0 * PI * j as f32 / q as f32;
                    v.push([a.cos(), a.sin(), b.cos(), b.sin()]);
                    let id = (i * q + j) as u16;
                    e.push((id, (((i + 1) % p) * q + j) as u16));
                    e.push((id, (i * q + (j + 1) % q) as u16));
                }
            }
            ("duoprism", format!("{{{}}}x{{{}}}", p, q), v, Some(e))
        }
    };
    for v in verts.iter_mut() {
        norm4(v);
    }
    let edges = edges.unwrap_or_else(|| edges_nearest(&verts));
    Poly { name, schlafli, verts, edges }
}

fn build(seed: u64, poly_choice: u32, planes_n: u32) -> Cached {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x4D_7C0F_A11);
    let pick = [1u32, 2, 3, 4, 5, 7][rng.random_range(0..6)];
    let choice = if poly_choice == 0 { pick } else { poly_choice };
    let p = rng.random_range(3..=8usize);
    let q = rng.random_range(3..=8usize);
    let poly = make_poly(choice, p, q);
    let mut order: Vec<usize> = (0..6).collect();
    for i in (1..6).rev() {
        let j = rng.random_range(0..=i);
        order.swap(i, j);
    }
    let mut chosen: Vec<usize> = order.iter().take(planes_n as usize).copied().collect();
    if !chosen.iter().any(|&i| PLANE_AXES[i].1 == 3) {
        let wp = [2usize, 4, 5][rng.random_range(0..3)];
        let last = chosen.len() - 1;
        chosen[last] = wp;
    }
    let ratios = [1.0, 1.0 / PHI, 1.0 / (PHI * PHI)];
    let planes = chosen
        .iter()
        .enumerate()
        .map(|(n, &i)| Plane {
            axes: PLANE_AXES[i],
            rate: ratios[n] * if rng.random::<f32>() < 0.5 { -1.0 } else { 1.0 },
            phase: rng.random::<f32>() * 2.0 * PI,
            name: PLANE_NAMES[i],
        })
        .collect();
    let hue_seed = rng.random_range(0..360) as f32;
    let style_seed = rng.random_range(1..=3u32);
    let yaw0 = rng.random::<f32>() * 2.0 * PI;
    let nv = poly.verts.len();
    Cached {
        key: (seed, poly_choice, planes_n),
        poly,
        planes,
        hue_seed,
        style_seed,
        yaw0,
        p3: vec![[0.0; 3]; nv],
        scr: vec![Scr::default(); nv],
        prev: vec![Scr::default(); nv],
        cur: vec![Scr::default(); nv],
        zbuf: Vec::new(),
        ebuf: Vec::new(),
        tglow: Vec::new(),
        tw: Vec::new(),
        crossings: Vec::new(),
        buf_dims: (0, 0),
        lut: vec![Color::Reset; WB * ZB],
    }
}

#[derive(Clone, Copy)]
struct View {
    cx: f32,
    cy: f32,
    fx: f32,
    fy: f32,
    fov: f32,
    cam: f32,
    pc: f32,
    ps: f32,
    yaw: f32,
    floor_y: f32,
}

impl View {
    fn to_cam(&self, p: [f32; 3]) -> [f32; 3] {
        [p[0], p[1] * self.pc + p[2] * self.ps, -p[1] * self.ps + p[2] * self.pc]
    }

    fn project(&self, p: [f32; 3]) -> Scr {
        let c = self.to_cam(p);
        let zc = (c[2] + self.cam).max(0.2);
        let sx = self.cx + self.fx * c[0] / zc;
        let sy = self.cy - self.fy * c[1] / zc;
        let near = ((self.cam - zc) / 3.0 + 0.5).clamp(0.0, 1.0);
        Scr { sx, sy, near, w: 0.0, ok: c[2] + self.cam > 0.2 }
    }
}

fn rotors(c: &Cached, t: f32) -> [(usize, usize, f32, f32); 3] {
    let mut rots: [(usize, usize, f32, f32); 3] = [(0, 0, 1.0, 0.0); 3];
    for (i, pl) in c.planes.iter().enumerate() {
        let a = pl.phase + pl.rate * t;
        rots[i] = (pl.axes.0, pl.axes.1, a.cos(), a.sin());
    }
    rots
}

fn rotate4(v: &[f32; 4], rots: &[(usize, usize, f32, f32)]) -> [f32; 4] {
    let mut p = *v;
    for r in rots {
        let (a, b) = (p[r.0], p[r.1]);
        p[r.0] = a * r.2 - b * r.3;
        p[r.1] = a * r.3 + b * r.2;
    }
    p
}

fn pose(c: &Cached, view: &View, t: f32, out_p3: &mut [[f32; 3]], out_scr: &mut [Scr]) {
    let rots = rotors(c, t);
    let np = c.planes.len();
    let (yc, ys) = (view.yaw.cos(), view.yaw.sin());
    for (i, v) in c.poly.verts.iter().enumerate() {
        let p = rotate4(v, &rots[..np]);
        let s = view.fov / (view.fov - p[3]).max(0.2);
        let x = p[0] * s;
        let y = p[1] * s;
        let z = p[2] * s;
        let p3 = [x * yc + z * ys, y, -x * ys + z * yc];
        out_p3[i] = p3;
        let mut sc = view.project(p3);
        sc.w = p[3];
        out_scr[i] = sc;
    }
}

fn line_walk(a: (f32, f32), b: (f32, f32), w: usize, h: usize, mut f: impl FnMut(usize, usize, f32)) {
    let lim_x = 4.0 * w as f32;
    let lim_y = 4.0 * h as f32;
    if a.0.abs() > lim_x || b.0.abs() > lim_x || a.1.abs() > lim_y || b.1.abs() > lim_y {
        return;
    }
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as usize;
    let inv = 1.0 / steps as f32;
    for s in 0..=steps {
        let u = s as f32 * inv;
        let x = a.0 + dx * u;
        let y = a.1 + dy * u;
        if x < 0.0 || y < 0.0 {
            continue;
        }
        let (xi, yi) = (x as usize, y as usize);
        if xi < w && yi < h {
            f(xi, yi, u);
        }
    }
}

const EDGE_GLYPHS: [[char; 4]; 3] = [['·', ':', '.', '.'], ['-', '|', '/', '\\'], ['─', '│', '╱', '╲']];

fn band_of(near: f32) -> usize {
    if near > 0.62 {
        2
    } else if near > 0.3 {
        1
    } else {
        0
    }
}

fn put(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, cell: Cell) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        grid[y as usize][x as usize] = cell;
    }
}

pub(crate) fn draw_polytope(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &PolytopeKnobs) {
    let key = (seed, k.poly_n(), k.planes_n());
    CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let stale = slot.as_ref().map(|c| c.key != key).unwrap_or(true);
        if stale {
            *slot = Some(build(seed, key.1, key.2));
        }
        let c = slot.as_mut().unwrap();
        render(grid, w, h, palette, t, k, c);
    });
}

fn render(grid: &mut Grid, w: usize, h: usize, palette: &[Color; 5], t: f32, k: &PolytopeKnobs, c: &mut Cached) {
    let n = w * h;
    let bg = darken(palette[0], 4);
    measure_layer("polytope", "clear", || {
        for row in grid.iter_mut().take(h) {
            for cell in row.iter_mut().take(w) {
                *cell = Cell::blank();
            }
        }
        if c.buf_dims != (w, h) {
            c.zbuf = vec![f32::MAX; n];
            c.ebuf = vec![0; n];
            c.tglow = vec![0.0; n];
            c.tw = vec![0.0; n];
            c.buf_dims = (w, h);
        } else {
            c.zbuf.fill(f32::MAX);
            c.ebuf.fill(0);
            c.tglow.fill(0.0);
            c.tw.fill(0.0);
        }
        c.crossings.clear();
        let base = shift_hue(palette[3], (c.hue_seed + k.hue) as f64);
        let span = k.span as f64;
        for wb in 0..WB {
            let hue = (wb as f64 / (WB - 1) as f64 - 0.5) * span;
            let col = shift_hue(base, hue);
            for zb in 0..ZB {
                let f = zb as f32 / (ZB - 1) as f32;
                let mut cc = lerp_color(bg, col, 0.22 + 0.78 * f);
                if zb == ZB - 1 {
                    cc = lighten(cc, 40);
                }
                c.lut[wb * ZB + zb] = cc;
            }
        }
    });
    if w < 8 || h < 4 {
        return;
    }
    let style = if k.style.round() as u32 == 0 { c.style_seed } else { (k.style.round() as u32).clamp(1, 3) };
    let aspect = k.aspect.max(0.25);
    let fov = k.fov.max(1.2);
    let cam = k.cam.max(2.5);
    let fy = k.zoom.max(0.05) * (h as f32 / 2.0).min(w as f32 / (2.0 * aspect)) * cam;
    let pitch = k.pitch.to_radians();
    let yaw = c.yaw0 + if t > 0.0 { k.orbit.to_radians() * t } else { 0.0 };
    let view = View {
        cx: w as f32 / 2.0,
        cy: h as f32 / 2.0 + fy * pitch.tan() * 0.2,
        fx: fy * aspect,
        fy,
        fov,
        cam,
        pc: pitch.cos(),
        ps: pitch.sin(),
        yaw,
        floor_y: -k.floor.max(0.0) - 0.4,
    };
    let anim_t = if t > 0.0 { t } else { 0.0 };
    let period = k.speed.max(1.0);
    let omega = 2.0 * PI / period;
    let (poly_edges, nv) = (c.poly.edges.len(), c.poly.verts.len());
    let cull = k.cull.clamp(0.0, 0.9) * ((poly_edges as f32 - 200.0) / 600.0).clamp(0.0, 1.0);
    let lut = std::mem::take(&mut c.lut);
    let lut_at = |wv: f32, near: f32| -> Color {
        let wb = (((wv + 1.0) * 0.5).clamp(0.0, 0.999) * WB as f32) as usize;
        let zb = (near.clamp(0.0, 0.999) * ZB as f32) as usize;
        lut[wb * ZB + zb]
    };

    let mut p3 = std::mem::take(&mut c.p3);
    let mut scr = std::mem::take(&mut c.scr);
    let mut zbuf = std::mem::take(&mut c.zbuf);
    let mut ebuf = std::mem::take(&mut c.ebuf);
    let mut tglow = std::mem::take(&mut c.tglow);
    let mut tw = std::mem::take(&mut c.tw);
    let mut crossings = std::mem::take(&mut c.crossings);
    measure_layer("polytope", "pose", || {
        pose(c, &view, anim_t * omega, &mut p3, &mut scr);
    });

    let floor_col = darken(palette[2], 30);
    let horizon_col = lerp_color(bg, palette[2], 0.35);
    if style & 1 == 1 {
        measure_layer("polytope", "floor", || {
            let tile = k.tile.max(0.1);
            let drift = k.flow * anim_t;
            let eye_y = cam * view.ps;
            let eye_z = -cam * view.pc;
            let mut horizon_row: i32 = -1;
            let floor_z = |v: f32| -> Option<(f32, f32)> {
                let dy = v * view.pc - view.ps;
                if dy >= -1e-4 {
                    return None;
                }
                let tt = (view.floor_y - eye_y) / dy;
                if tt <= 0.0 {
                    return None;
                }
                Some((tt, eye_z + (v * view.ps + view.pc) * tt + drift))
            };
            for y in 0..h {
                let v = (view.cy - y as f32 - 0.5) / view.fy;
                let Some((tt, zf)) = floor_z(v) else {
                    horizon_row = y as i32;
                    continue;
                };
                let z_top = floor_z(v + 0.5 / view.fy).map(|p| p.1).unwrap_or(zf + tile * 4.0);
                let z_bot = floor_z(v - 0.5 / view.fy).map(|p| p.1).unwrap_or(zf);
                let rows_per_tile = tile / (z_top - z_bot).max(1e-4);
                let dense = (rows_per_tile / 2.5).clamp(0.0, 1.0);
                let z_line = (z_top / tile).floor() != (z_bot / tile).floor() && rows_per_tile > 1.2;
                let fade = (1.0 - tt / (cam * 3.5)).clamp(0.0, 1.0) * dense;
                if fade < 0.05 {
                    continue;
                }
                let col = lerp_color(bg, floor_col, 0.2 + 0.55 * fade);
                let half = 0.5 * tt / view.fx;
                let row = &mut grid[y];
                for x in 0..w {
                    let u = (x as f32 + 0.5 - view.cx) / view.fx;
                    let xf = u * tt;
                    let x_line = ((xf - half) / tile).floor() != ((xf + half) / tile).floor() && half * 2.0 < tile * 0.8;
                    let ch = match (z_line, x_line) {
                        (true, true) => '+',
                        (true, false) => '·',
                        (false, true) => if fade > 0.5 { ':' } else { '.' },
                        _ => continue,
                    };
                    row[x] = Cell::new(ch, col);
                }
            }
            if horizon_row >= 0 && (horizon_row as usize) < h {
                let row = &mut grid[horizon_row as usize];
                for x in 0..w {
                    row[x] = Cell::new('─', horizon_col);
                }
            }
        });

        measure_layer("polytope", "shadow", || {
            let shadow_col = lerp_color(bg, darken(palette[1], 20), 0.5);
            let shadow_far = lerp_color(bg, darken(palette[1], 20), 0.28);
            let slant = 0.35;
            for &(a, b) in &c.poly.edges {
                let (pa, pb) = (p3[a as usize], p3[b as usize]);
                let drop = |p: [f32; 3]| {
                    let lift = p[1] - view.floor_y;
                    view.project([p[0] + slant * lift, view.floor_y, p[2] + slant * 0.4 * lift])
                };
                let (sa, sb) = (drop(pa), drop(pb));
                if !sa.ok || !sb.ok {
                    continue;
                }
                let (na, nb) = (sa.near, sb.near);
                line_walk((sa.sx, sa.sy), (sb.sx, sb.sy), w, h, |x, y, u| {
                    let nr = na + (nb - na) * u;
                    let cell = &mut grid[y][x];
                    let ch = if nr > 0.55 { '▒' } else if nr > 0.3 { '░' } else { ':' };
                    *cell = Cell::new(ch, if nr > 0.3 { shadow_col } else { shadow_far });
                });
            }
        });
    }

    let trail_n = if style & 2 == 2 { k.trail_n() } else { 0 };
    let mut prev = std::mem::take(&mut c.prev);
    let mut cur = std::mem::take(&mut c.cur);
    if trail_n > 0 {
        measure_layer("polytope", "trails", || {
            let tail = k.tail.max(0.005);
            prev.copy_from_slice(&scr);
            let mut spare = std::mem::take(&mut p3);
            for s in 1..=trail_n {
                let ts = (anim_t - s as f32 * tail) * omega;
                let view_s = View { yaw: c.yaw0 + k.orbit.to_radians() * (anim_t - s as f32 * tail), ..view };
                pose(c, &view_s, ts, &mut spare, &mut cur);
                let fade = 1.0 - s as f32 / (trail_n as f32 + 1.0);
                let amp = fade * fade;
                for i in 0..nv {
                    let (a, b) = (prev[i], cur[i]);
                    if !a.ok || !b.ok {
                        continue;
                    }
                    let wv = a.w;
                    line_walk((a.sx, a.sy), (b.sx, b.sy), w, h, |x, y, _| {
                        let idx = y * w + x;
                        if tglow[idx] < amp {
                            tglow[idx] = amp;
                            tw[idx] = wv;
                        }
                    });
                }
                std::mem::swap(&mut prev, &mut cur);
            }
            p3 = spare;
            for y in 0..h {
                for x in 0..w {
                    let idx = y * w + x;
                    let g = tglow[idx];
                    if g < 0.04 {
                        continue;
                    }
                    let ch = if g < 0.25 { '.' } else if g < 0.55 { '·' } else if g < 0.85 { ':' } else { '•' };
                    grid[y][x] = Cell::new(ch, lut_at(tw[idx], g * 0.6));
                }
            }
        });
    }

    measure_layer("polytope", "edges", || {
        let dir_of = |sa: Scr, sb: Scr| -> u8 {
            let ang = ((sb.sy - sa.sy) * aspect).atan2(sb.sx - sa.sx).abs();
            if ang < PI / 8.0 || ang > 7.0 * PI / 8.0 {
                0
            } else if ang > 3.0 * PI / 8.0 && ang < 5.0 * PI / 8.0 {
                1
            } else if (sb.sx > sa.sx) == (sb.sy > sa.sy) {
                3
            } else {
                2
            }
        };
        for (ei, &(a, b)) in c.poly.edges.iter().enumerate() {
            let (sa, sb) = (scr[a as usize], scr[b as usize]);
            if !sa.ok || !sb.ok {
                continue;
            }
            if sa.near.max(sb.near) < cull {
                continue;
            }
            let dir = dir_of(sa, sb);
            let id = ei as u16 + 1;
            line_walk((sa.sx, sa.sy), (sb.sx, sb.sy), w, h, |x, y, u| {
                let idx = y * w + x;
                let near = sa.near + (sb.near - sa.near) * u;
                let wv = sa.w + (sb.w - sa.w) * u;
                let depth = 1.0 - near;
                if depth >= zbuf[idx] {
                    return;
                }
                let other = ebuf[idx];
                if other != 0 && other != id && k.glow > 0.5 && zbuf[idx] - depth > 0.3 && near > 0.55 {
                    let (oa, ob) = c.poly.edges[other as usize - 1];
                    let shared = oa == a || oa == b || ob == a || ob == b;
                    let odir = dir_of(scr[oa as usize], scr[ob as usize]);
                    if !shared && odir != dir {
                        crossings.push((idx, wv.to_bits() as usize));
                    }
                }
                zbuf[idx] = depth;
                ebuf[idx] = id;
                grid[y][x] = Cell::new(EDGE_GLYPHS[band_of(near)][dir as usize], lut_at(wv, near));
            });
        }
        for &(idx, wbits) in &crossings {
            let wv = f32::from_bits(wbits as u32);
            let near = 1.0 - zbuf[idx];
            let (x, y) = (idx % w, idx / w);
            let ch = if near > 0.45 { '*' } else { '+' };
            grid[y][x] = Cell::new(ch, lighten(lut_at(wv, near.max(0.6)), 50));
        }
    });

    measure_layer("polytope", "vertices", || {
        for i in 0..nv {
            let s = scr[i];
            if !s.ok || s.sx < 0.0 || s.sy < 0.0 || s.near < cull {
                continue;
            }
            let (x, y) = (s.sx as usize, s.sy as usize);
            if x >= w || y >= h {
                continue;
            }
            let w01 = (s.w + 1.0) * 0.5;
            let ch = if w01 < 0.25 {
                '·'
            } else if w01 < 0.5 {
                'o'
            } else if w01 < 0.75 {
                '●'
            } else {
                '◆'
            };
            let idx = y * w + x;
            let depth = 1.0 - s.near;
            if depth <= zbuf[idx] + 0.08 {
                zbuf[idx] = depth;
                grid[y][x] = Cell::new(ch, lighten(lut_at(s.w, s.near), 20));
            }
        }
    });

    measure_layer("polytope", "inset", || {
        let r = k.inset.clamp(0.0, 0.5) * h as f32;
        if r < 2.0 {
            return;
        }
        let rots = rotors(c, anim_t * omega);
        let np = c.planes.len();
        let icx = w as f32 - r * aspect - 2.0;
        let icy = r + 2.0 + if style & 1 == 1 { (view.cy - view.fy * pitch.tan()).max(0.0) } else { 0.0 };
        let ink = lerp_color(bg, palette[4], 0.28);
        let ink_v = lerp_color(bg, palette[4], 0.5);
        for (i, v) in c.poly.verts.iter().enumerate() {
            let p = rotate4(v, &rots[..np]);
            scr[i].sx = icx + p[0] * r * aspect;
            scr[i].sy = icy - p[1] * r;
            scr[i].w = p[3];
        }
        for &(a, b) in &c.poly.edges {
            let (sa, sb) = (scr[a as usize], scr[b as usize]);
            line_walk((sa.sx, sa.sy), (sb.sx, sb.sy), w, h, |x, y, _| {
                if grid[y][x].ch == ' ' {
                    grid[y][x] = Cell::new('·', ink);
                }
            });
        }
        for i in 0..nv {
            let s = scr[i];
            if s.sx < 0.0 || s.sy < 0.0 {
                continue;
            }
            let (x, y) = (s.sx as usize, s.sy as usize);
            if x < w && y < h {
                grid[y][x] = Cell::new(if s.w > 0.0 { 'o' } else { '.' }, ink_v);
            }
        }
    });

    measure_layer("polytope", "label", || {
        if k.label > 0.5 && h >= 6 {
            let names: Vec<&str> = c.planes.iter().map(|p| p.name).collect();
            let text = format!("{} {}  {}  {}v {}e", c.poly.schlafli, c.poly.name, names.join("+"), nv, poly_edges);
            let fg = lerp_color(bg, palette[4], 0.6);
            for (i, ch) in text.chars().enumerate() {
                put(grid, w, h, 1 + i as i32, h as i32 - 1, Cell::new(ch, fg));
            }
        }
    });

    c.lut = lut;
    c.p3 = p3;
    c.scr = scr;
    c.prev = prev;
    c.cur = cur;
    c.zbuf = zbuf;
    c.ebuf = ebuf;
    c.tglow = tglow;
    c.tw = tw;
    c.crossings = crossings;
}

pub(crate) fn cli_polytope(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = PolytopeKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 21] = [
        &mut k.poly,
        &mut k.speed,
        &mut k.planes,
        &mut k.fov,
        &mut k.zoom,
        &mut k.style,
        &mut k.floor,
        &mut k.trail,
        &mut k.tail,
        &mut k.hue,
        &mut k.span,
        &mut k.orbit,
        &mut k.pitch,
        &mut k.tile,
        &mut k.flow,
        &mut k.glow,
        &mut k.label,
        &mut k.cam,
        &mut k.aspect,
        &mut k.cull,
        &mut k.inset,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_polytope(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = PolytopeKnobs::from_env();
        draw_polytope(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_polytope_small() {
        insta::assert_snapshot!("polytope_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn edge_counts_match_the_regular_polytopes() {
        let expect = [(1, 5, 10), (2, 16, 32), (3, 8, 24), (4, 24, 96), (5, 120, 720), (6, 600, 1200)];
        for (choice, nv, ne) in expect {
            let p = make_poly(choice, 3, 3);
            assert_eq!((p.verts.len(), p.edges.len()), (nv, ne), "{}", p.name);
        }
        let d = make_poly(7, 5, 7);
        assert_eq!((d.verts.len(), d.edges.len()), (35, 70));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn t_turns_the_polytope() {
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 4.0));
        assert_ne!(run(90, 30, 42, 4.0), run(90, 30, 42, 9.0));
    }

    #[test]
    fn every_polytope_renders_with_trails_and_floor() {
        for choice in 1..=7 {
            let mut g = vec![vec![Cell::blank(); 60]; 20];
            let p = crate::color::make_palette(3);
            let mut k = PolytopeKnobs::from_env();
            k.poly = choice as f32;
            k.style = 3.0;
            draw_polytope(&mut g, 60, 20, 3, &p, 5.0, &k);
        }
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let mut k = PolytopeKnobs::from_env();
        k.poly = 6.0;
        k.style = 3.0;
        draw_polytope(&mut g, w, h, 42, &p, 0.0, &k);
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..200 {
            let t0 = std::time::Instant::now();
            draw_polytope(&mut g, w, h, 42, &p, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("polytope frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }
}
