//! Shamsa: looking up into a Safavid dome. Girih rosettes interlace across
//! muqarnas tiers while a lantern light sweeps the vault. Design: briefs/shamsa.md.
use crate::_0_profile::measure_layer;
use crate::color::{hsl_to_rgb, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};

pub(super) struct Shamsa;
pub(super) static MODE: Shamsa = Shamsa;

const NAME: &str = "shamsa";
const KNOBS: usize = 10;
const HELP: &str = "shamsa: Iran, Safavid dome girih strapwork, muqarnas tiers, sweeping lantern [FOLD] [DEPTH] [SPIN] [FLOW] [LANTERN] [VINE] [STRAP] [BLOOM] [HUE] [ASPECT]";

const PARAMS: &[Param] = &[
    param!("FOLD", "girih star order", 5.0, 14.0, 10.0, 1.0),
    param!("DEPTH", "dome projection depth", 0.0, 1.0, 0.72, 0.01),
    param!("SPIN", "dome rotation rad/s", -0.3, 0.3, 0.06, 0.01),
    param!("FLOW", "lantern sweep rad/s", 0.0, 1.5, 0.35, 0.05),
    param!("LANTERN", "lantern height on the dome", 0.0, 1.0, 0.3, 0.01),
    param!("VINE", "arabesque growth budget", 0.0, 1.0, 0.55, 0.01),
    param!("STRAP", "strap width", 0.02, 0.3, 0.16, 0.005),
    param!("BLOOM", "glow around bright knots", 0.0, 1.0, 0.45, 0.01),
    param!("HUE", "palette rotation degrees", 0.0, 360.0, 210.0, 1.0),
    param!("ASPECT", "cols per row", 1.0, 4.0, 2.1, 0.05),
];

const L_FIELD: u64 = 0x21;
const L_VAULT: u64 = 0x22;
const L_STRAP: u64 = 0x23;
const L_BOSS: u64 = 0x24;
const L_GLOW: u64 = 0x26;

const PARALLEL_MIN_CELLS: usize = 20_480;

/// Dome surface ramp, dark plaster to lit tile.
const VAULT_RAMP: [char; 12] = [' ', '.', ',', ':', ';', '!', 'i', 'l', '+', 'x', 'A', '@'];

/// Splitmix64 over (seed, layer, index, slot); no rng stream is consumed.
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
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn wrap_pi(a: f32) -> f32 {
    let a = a.rem_euclid(TAU);
    if a > PI {
        a - TAU
    } else {
        a
    }
}

/// Screen row fraction from dome polar angle, flat disc to hemisphere.
#[inline]
fn n_of_u(u: f32, depth: f32) -> f32 {
    mix(u, (u * PI * 0.5).sin(), depth)
}

/// Inverse projection: dome polar angle from screen row fraction.
#[inline]
fn u_of_r(n: f32, depth: f32) -> f32 {
    mix(n, n.clamp(0.0, 1.0).asin() * (2.0 / PI), depth)
}
/// d(screen fraction)/d(dome angle), drives tier foreshortening.
#[inline]
fn dndu(u: f32, depth: f32) -> f32 {
    mix(1.0, (u * PI * 0.5).cos() * (PI * 0.5), depth)
}

#[inline]
fn ramp(ramp: &[char], v: f32) -> char {
    let i = (v.clamp(0.0, 1.0) * (ramp.len() - 1) as f32).round() as usize;
    ramp[i]
}

/// One bright anchor for the glow pass.
struct NodeGlow {
    x: f32,
    y: f32,
    b: f32,
}

/// One frame's resolved projection, girih layout, lantern, and palette.
struct Look {
    seed: u64,
    cx: f32,
    cy: f32,
    radius: f32,
    aspect: f32,
    fold: i32,
    depth: f32,
    strap: f32,
    bloom: f32,
    vine: f32,
    time: f32,
    spin_now: f32,
    light_az: f32,
    light_u: f32,
    tier_n: f32,
    rb: f32,
    jit_a: [f32; 10],
    jit_b: [f32; 10],
    nodes: Vec<NodeGlow>,
    wall: Color,
    dome_lo: Color,
    dome_hi: Color,
    seam: Color,
    niche: Color,
    strap_dim: Color,
    strap_lit: Color,
    u_a: f32,
    u_b: f32,
    n_a: f32,
    n_b: f32,
    u_in: f32,
    u_out: f32,
    size_c: f32,
    size_a: f32,
    size_b: f32,
    strap_hot: Color,
    vine_a: Color,
    boss: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let aspect = p[9].clamp(1.0, 4.0);
        let cx = w as f32 * 0.5;
        let cy = h as f32 * 0.5;
        let radius = (cy - 1.0).min(cx / aspect - 1.0).max(2.0);
        let depth = p[1];
        let spin_now = p[2] * time;
        let light_az = -2.35 + p[3] * time + (unit(hash(seed, L_FIELD, 0, 1)) - 0.5) * 1.2;
        let n_a = 0.64 + (unit(hash(seed, L_STRAP, 0, 4)) - 0.5) * 0.07;
        let n_b = 0.86 + (unit(hash(seed, L_STRAP, 0, 5)) - 0.5) * 0.05;
        let u_a = u_of_r(n_a, depth);
        let u_b = u_of_r(n_b, depth);
        let u_in = u_of_r((n_a + 0.19).min(0.99), depth);
        let u_out = u_of_r((n_b - 0.13).max(0.2), depth);
        let fold = ((p[0].round() as i32) + (hash(seed, L_STRAP, 0, 11) % 3) as i32 - 1).clamp(5, 14);
        let tier_n = (5 + (hash(seed, L_VAULT, 0, 6) % 3) as i32) as f32;
        let rb = smoothstep((radius - 11.0) / 7.0);
        let mut jit_a = [0.0f32; 10];
        let mut jit_b = [0.0f32; 10];
        for i in 0..10 {
            jit_a[i] = (unit(hash(seed, L_STRAP, i as u64, 9)) - 0.5) * 0.16;
            jit_b[i] = (unit(hash(seed, L_STRAP, (i + 10) as u64, 9)) - 0.5) * 0.16;
        }
        let hue = 206.0 + (unit(hash(seed, L_VAULT, 0, 7)) - 0.5) * 26.0 + (p[8] - 210.0);
        let col = |dh: f64, s: f64, l: f64| hsl_to_rgb(((hue as f64 + dh).rem_euclid(360.0)) as f64, s, l);
        let wedge_a = TAU / 10.0;
        let size_a = 0.16 * (1.0 + (unit(hash(seed, L_STRAP, 0, 12)) - 0.5) * 0.36);
        let size_b = 0.115 * (1.0 + (unit(hash(seed, L_STRAP, 0, 13)) - 0.5) * 0.36) * rb.max(0.2);
        let mut nodes = Vec::with_capacity(21);
        nodes.push(NodeGlow {
            x: cx,
            y: cy,
            b: 1.0,
        });
        for i in 0..10 {
            let th = i as f32 * wedge_a + jit_a[i] + spin_now;
            let dl = u_a - (0.18 + 0.72 * p[4]);
            let b = (0.35 + 0.65 * (-dl * dl * 5.0).exp()).min(1.0);
            nodes.push(NodeGlow {
                x: cx + th.cos() * n_a * radius * aspect,
                y: cy + th.sin() * n_a * radius,
                b,
            });
        }
        for i in 0..10 {
            let th = (i as f32 + 0.5) * wedge_a + jit_b[i] + spin_now;
            let dl = u_b - (0.18 + 0.72 * p[4]);
            let b = ((0.3 + 0.6 * (-dl * dl * 5.0).exp()).min(1.0)) * rb;
            nodes.push(NodeGlow {
                x: cx + th.cos() * n_b * radius * aspect,
                y: cy + th.sin() * n_b * radius,
                b,
            });
        }
        Look {
            seed,
            cx,
            cy,
            radius,
            aspect,
            fold,
            depth,
            strap: p[6],
            bloom: p[7],
            vine: p[5],
            time,
            spin_now,
            light_az,
            light_u: 0.18 + 0.72 * p[4],
            u_a,
            u_b,
            n_a,
            n_b,
            u_in,
            u_out,
            size_c: 0.40,
            size_a,
            size_b,
            tier_n,
            rb,
            jit_a,
            jit_b,
            nodes,
            wall: col(14.0, 0.5, 0.05),
            dome_lo: col(8.0, 0.52, 0.085),
            dome_hi: col(-6.0, 0.58, 0.30),
            seam: col(16.0, 0.45, 0.05),
            niche: col(-24.0, 0.55, 0.20),
            strap_dim: col(-160.0, 0.40, 0.30),
            strap_lit: col(-168.0, 0.78, 0.62),
            strap_hot: col(-172.0, 0.85, 0.78),
            vine_a: col(-48.0, 0.55, 0.34),
            boss: col(-170.0, 0.70, 0.82),
        }
    }
}

/// Per-cell dome sample: projection, strap distance, crossing state, tiers.
#[derive(Clone, Copy, Default)]
struct Sample {
    u: f32,
    theta: f32,
    light: f32,
    strap: f32,
    sdir: f32,
    over: u8,
    tier: u8,
    tier_cell: u8,
    owner: u8,
    rho: f32,
    foam: f32,
}

thread_local! {
    static SCRATCH: RefCell<Vec<Sample>> = const { RefCell::new(Vec::new()) };
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
        measure_layer(NAME, "field", || sample_field(field, w, h, &look));
        measure_layer(NAME, "vault", || paint_vault(frame.grid, field, w, h, &look));
        measure_layer(NAME, "straps", || paint_straps(frame.grid, field, w, h, &look));
        measure_layer(NAME, "boss", || paint_boss(frame.grid, w, h, &look));
        measure_layer(NAME, "glow", || paint_glow(frame.grid, w, h, &look));
    });
}

/// Project every cell onto the dome and evaluate girih distances plus light.
fn sample_field(field: &mut [Sample], w: usize, h: usize, look: &Look) {
    let sw = look.strap * look.radius;
    let wedge_a = TAU / 10.0;
    let wedge_k = TAU / look.fold as f32;
    let kink = 0.52f32;
    let size_cells_c = look.size_c * look.radius;
    for (y, row) in field.chunks_mut(w).enumerate() {
        let dy = y as f32 + 0.5 - look.cy;
        for (x, s) in row.iter_mut().enumerate() {
            let dx = (x as f32 + 0.5 - look.cx) / look.aspect;
            let r = (dx * dx + dy * dy).sqrt();
            if r > look.radius * 1.04 {
                *s = Sample {
                    u: 9.9,
                    theta: 0.0,
                    light: 0.0,
                    strap: 1.0e9,
                    sdir: 0.0,
                    over: 0,
                    tier: 0,
                    tier_cell: 0,
                    owner: 0,
                    rho: 0.0,
                    foam: 0.0,
                };
                continue;
            }
            let n = r / look.radius;
            let u = u_of_r(n, look.depth);
            let theta_r = dy.atan2(dx);
            let theta = theta_r + look.spin_now;
            let met_r = look.radius * dndu(u, look.depth);

            // lantern light on the dome surface, rim falloff applied later
            let du_l = u - look.light_u;
            let dth_l = wrap_pi(theta - look.light_az) * n.max(0.06);
            let d2 = du_l * du_l + dth_l * dth_l;
            let mut light = 1.35 * (-d2 / 0.18).exp() + 0.55 * (-(u * 3.4) * (u * 3.4)).exp();

            // pick the owning rosette among center, ring A, ring B
            let norm_c = r / size_cells_c;
            let i_a = (((theta / wedge_a).round() as i64).rem_euclid(10)) as usize;
            let du_a = u - look.u_a;
            let dt_a = wrap_pi(theta - i_a as f32 * wedge_a - look.jit_a[i_a]);
            let d_a = ((du_a * met_r).powi(2) + (dt_a * r).powi(2)).sqrt();
            let norm_a = d_a / (look.size_a * look.radius);
            let i_b = ((((theta - wedge_a * 0.5) / wedge_a).round() as i64).rem_euclid(10)) as usize;
            let du_b = u - look.u_b;
            let dt_b = wrap_pi(theta - (i_b as f32 + 0.5) * wedge_a - look.jit_b[i_b]);
            let d_b = ((du_b * met_r).powi(2) + (dt_b * r).powi(2)).sqrt();
            let norm_b = d_b / (look.size_b * look.radius);
            let (owner, dist_cells, size_cells) = if norm_c <= norm_a && norm_c <= norm_b {
                (1u8, r, size_cells_c)
            } else if norm_a <= norm_b {
                (2u8, d_a, look.size_a * look.radius)
            } else {
                (3u8, d_b, look.size_b * look.radius)
            };

            // local frame of the owner and analytic girih star distance
            let mut strap = 1.0e9;
            let mut sdir = 0.0f32;
            let mut fam = 0u8;
            let mut idx = 0u64;
            let mut rho = 9.9f32;
            if owner > 0 && dist_cells < size_cells * 1.45 {
                let (lx, ly) = if owner == 1 {
                    (dx / size_cells, dy / size_cells)
                } else if owner == 2 {
                    (dt_a * r / size_cells, du_a * met_r / size_cells)
                } else {
                    (dt_b * r / size_cells, du_b * met_r / size_cells)
                };
                rho = (lx * lx + ly * ly).sqrt();
                let phi = ly.atan2(lx);
                if rho < 1.6 {
                    let nr = (phi / wedge_k).round();
                    let dphi = phi - nr * wedge_k;
                    let d1 = rho * dphi.sin().abs();
                    let iw = (phi / wedge_k).floor() as i64;
                    let ia = iw.rem_euclid(look.fold as i64);
                    let ib = (iw + 1).rem_euclid(look.fold as i64);
                    let ka = ia as f32 * wedge_k;
                    let kb = ib as f32 * wedge_k;
                    let qx = lx - kink * ka.cos();
                    let qy = ly - kink * ka.sin();
                    let da = (qx * (kb + wedge_k).sin() - qy * (kb + wedge_k).cos()).abs();
                    let px = lx - kink * kb.cos();
                    let py = ly - kink * kb.sin();
                    let db_ = (px * ka.sin() - py * ka.cos()).abs();
                    let d2s = da.min(db_);
                    let blend = smoothstep((rho - kink) / 0.10);
                    let mut d = mix(d1, d2s, blend);
                    d += (rho - 1.0).max(0.0) * 3.0;
                    strap = d * size_cells;
                    sdir = if blend < 0.5 {
                        beta_to_screen(nr * wedge_k, theta_r)
                    } else if da <= db_ {
                        beta_to_screen(kb + wedge_k, theta_r)
                    } else {
                        beta_to_screen(ka, theta_r)
                    };
                    fam = 0;
                    idx = ia as u64;
                }
            }

            // tier bands along the two node rings
            let band_a = (u - look.u_a).abs() * met_r;
            if band_a < strap {
                strap = band_a;
                sdir = theta_r + PI * 0.5;
                fam = 1;
                idx = 0;
            }
            let band_b = (u - look.u_b).abs() * met_r;
            if band_b < strap {
                strap = band_b;
                sdir = theta_r + PI * 0.5;
                fam = 1;
                idx = 1;
            }
            // radial connectors between the rings
            if u > look.u_in && u < look.u_out {
                let i_c = (((theta / wedge_a).round() as i64).rem_euclid(10)) as usize;
                let dt_c = wrap_pi(theta - i_c as f32 * wedge_a - look.jit_a[i_c]);
                let d3 = dt_c.abs() * r;
                if d3 < strap {
                    strap = d3;
                    sdir = theta_r + dt_c;
                    fam = 2;
                    idx = i_c as u64;
                }
            }

            // muqarnas tier and niche ids
            let tpos = u * look.tier_n;
            let tier = tpos.floor().max(0.0) as u8;
            let niche = (((theta.rem_euclid(TAU)) / wedge_a).floor() as i64).rem_euclid(10);
            let tier_cell = (hash(look.seed, L_VAULT, tier as u64, niche as u64) % 4) as u8;
            let npos = (theta.rem_euclid(TAU)) / wedge_a;
            let foam = (npos - npos.round()).abs() * wedge_a * r;

            // over or under where two strap families cross
            let second = second_family(look, u, theta, r, met_r, strap, fam, wedge_a);
            let mut over = 0u8;
            if second.0 < sw * 1.7 && strap < sw * 1.7 {
                let key = hash(look.seed, 0x25, (fam as u64) * 31 + idx, (second.1 as u64) * 31 + second.2);
                let winner = if key & 1 == 0 { fam } else { second.1 };
                over = if fam == winner { 1 } else { 2 };
            }

            // straps and tiers occlude the lantern
            let occl = (1.0 - (strap / sw).clamp(0.0, 1.0)) * 0.42;
            light *= 1.0 - occl;
            light *= 1.0 - 0.5 * smoothstep((n - 0.86) / 0.14);
            *s = Sample {
                u,
                theta,
                light,
                strap,
                sdir,
                over,
                tier,
                tier_cell,
                owner,
                rho,
                foam,
            };
        }
    }
}

/// Map a local strap direction angle into the screen frame at radius theta_r.
#[inline]
fn beta_to_screen(beta: f32, theta_r: f32) -> f32 {
    let (cb, sb) = beta.sin_cos();
    let (ct, st) = theta_r.sin_cos();
    (cb * st + sb * ct).atan2(-cb * ct + sb * st)
}

/// Nearest other strap family at this cell, for over/under crossings.
fn second_family(
    look: &Look,
    u: f32,
    theta: f32,
    r: f32,
    met_r: f32,
    best: f32,
    fam: u8,
    wedge_a: f32,
) -> (f32, u8, u64) {
    let mut out = (1.0e9, 0u8, 0u64);
    if fam != 1 {
        let band_a = (u - look.u_a).abs() * met_r;
        if band_a < out.0 {
            out = (band_a, 1, 0);
        }
        let band_b = (u - look.u_b).abs() * met_r;
        if band_b < out.0 {
            out = (band_b, 1, 1);
        }
    }
    if fam != 2 && u > look.u_in && u < look.u_out {
        let i_c = (((theta / wedge_a).round() as i64).rem_euclid(10)) as usize;
        let dt_c = wrap_pi(theta - i_c as f32 * wedge_a - look.jit_a[i_c]);
        let d3 = dt_c.abs() * r;
        if d3 < out.0 {
            out = (d3, 2, i_c as u64);
        }
    }
    if out.0 >= best {
        out.0 = 1.0e9;
    }
    out
}

/// Dome plaster, muqarnas tiers, and the dark wall beyond the rim.
fn paint_vault(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.u > 2.0 {
                let gx = (x / 7) as u64;
                let gy = (y / 3) as u64;
                let pick = hash(look.seed, L_VAULT, gx, gy) % 53;
                let ch = if pick == 0 {
                    '\''
                } else if pick == 1 {
                    '`'
                } else {
                    ' '
                };
                *cell = Cell::new(ch, look.wall);
                continue;
            }
            let tier_jit = match s.tier_cell {
                0 => -0.10,
                1 => -0.02,
                2 => 0.06,
                _ => 0.12,
            };
            let mut g = s.light * 0.4 + 0.13 + tier_jit;
            g *= 1.0 - 0.35 * smoothstep((s.u - 0.78) / 0.22);
            let shade = g.clamp(0.0, 1.0);
            let mut ch = ramp(&VAULT_RAMP, shade * 0.9);
            let mut fg = lerp_color(look.dome_lo, look.dome_hi, shade);
            if s.u < 1.0 {
                let tpos = s.u * look.tier_n;
                let frac = tpos - tpos.floor();
                let edge_d = frac.min(1.0 - frac) * look.radius * dndu(s.u, look.depth);
                if edge_d < 0.9 {
                    fg = look.seam;
                } else if s.foam < 0.8 && shade > 0.22 {
                    fg = lerp_color(fg, look.niche, 0.7);
                } else if s.strap > 3.0 * look.strap * look.radius {
                    fg = lerp_color(fg, look.niche, 0.35);
                }
            }
            let rim = 1.0 - smoothstep((s.u - 0.82) / 0.18);
            fg = lerp_color(look.wall, fg, rim.max(0.12));
            *cell = Cell::new(ch, fg);
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// Girih straps: direction glyphs, lantern tint, carved under-crossings.
fn paint_straps(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let sw = (look.strap * look.radius).max(0.6);
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.strap >= sw || s.u > 2.0 {
                continue;
            }
            if s.over == 2 && s.strap < sw * 0.6 {
                continue;
            }
            let edge = s.strap / sw;
            let lit = (s.light * 1.35).clamp(0.0, 1.0);
            let a = s.sdir.rem_euclid(PI);
            let dir_ch = if a < PI / 8.0 || a >= 7.0 * PI / 8.0 {
                '-'
            } else if a < 3.0 * PI / 8.0 {
                '\\'
            } else if a < 5.0 * PI / 8.0 {
                '|'
            } else {
                '/'
            };
            let ch = if s.over == 1 && lit > 0.6 && edge < 0.5 {
                '+'
            } else if edge < 0.5 {
                if lit > 0.55 {
                    '#'
                } else {
                    '='
                }
            } else {
                dir_ch
            };
            let mut fg = lerp_color(look.strap_dim, look.strap_lit, lit);
            if edge < 0.3 && lit > 0.7 {
                fg = look.strap_hot;
            }
            *cell = Cell::new(ch, fg);
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// The apex medallion: bright core with fold rays and a lit rim.
fn paint_boss(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let boss_r = (look.size_c * look.radius * 0.42 + 1.5).min(look.radius * 0.22);
    let wedge_k = TAU / look.fold as f32;
    let y0 = (look.cy - boss_r - 1.0).max(0.0) as usize;
    let y1 = ((look.cy + boss_r + 1.0) as usize).min(h);
    let x0 = ((look.cx - (boss_r + 1.0) * look.aspect).max(0.0)) as usize;
    let x1 = ((look.cx + (boss_r + 1.0) * look.aspect) as usize).min(w);
    for y in y0..y1 {
        let dy = y as f32 + 0.5 - look.cy;
        for x in x0..x1 {
            let dx = (x as f32 + 0.5 - look.cx) / look.aspect;
            let rd = (dx * dx + dy * dy).sqrt();
            if rd > boss_r {
                continue;
            }
            let phi = (dy.atan2(dx) - look.spin_now).rem_euclid(TAU);
            let dphi = (phi - (phi / wedge_k).round() * wedge_k).abs();
            let lit = (look.light_az.cos().abs() * 0.3 + 0.7).min(1.0);
            let (ch, fg) = if rd < boss_r * 0.42 {
                ('@', look.boss)
            } else if (rd - boss_r * 0.72).abs() < 0.8 {
                ('*', lighten(look.boss, 10))
            } else if dphi < 0.05 + rd * 0.03 {
                ('*', lerp_color(look.strap_lit, look.strap_hot, lit))
            } else {
                ('o', lerp_color(look.strap_dim, look.strap_lit, rd / boss_r))
            };
            grid[y][x] = Cell::new(ch, fg);
        }
    }
    for (ni, node) in look.nodes.iter().enumerate().skip(1) {
        let er2 = if ni > 10 { 0.35 * look.rb } else { 0.35 };
        if er2 < 0.05 {
            continue;
        }
        let ey0 = (node.y - 1.2).max(0.0) as usize;
        let ey1 = ((node.y + 1.2) as usize + 1).min(h);
        let ex0 = ((node.x - 1.2 * look.aspect).max(0.0)) as usize;
        let ex1 = ((node.x + 1.2 * look.aspect) as usize + 1).min(w);
        for y in ey0..ey1 {
            let dy = y as f32 + 0.5 - node.y;
            for x in ex0..ex1 {
                let dx = (x as f32 + 0.5 - node.x) / look.aspect;
                if dx * dx + dy * dy > er2 {
                    continue;
                }
                let lit = if node.b > 0.55 { 1.0 } else { 0.35 };
                let c = lerp_color(look.strap_dim, look.strap_hot, lit);
                let ch = if ni > 10 { 'o' } else { 'O' };
                grid[y][x] = Cell::new(ch, c);
            }
        }
    }
}

/// Sparse glow dust around bright knots; never paints over structure.
fn paint_glow(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    if look.bloom <= 0.01 {
        return;
    }
    let reach = 1.5 + look.bloom * 2.5 + look.vine * look.bloom * 1.5;
    for node in &look.nodes {
        let y0 = (node.y - reach).max(0.0) as usize;
        let y1 = ((node.y + reach) as usize + 1).min(h);
        let x0 = (node.x - reach * look.aspect).max(0.0) as usize;
        let x1 = ((node.x + reach * look.aspect) as usize + 1).min(w);
        for y in y0..y1 {
            for x in x0..x1 {
                if grid[y][x].ch != ' ' {
                    continue;
                }
                let dx = (x as f32 + 0.5 - node.x) / look.aspect;
                let dy = y as f32 + 0.5 - node.y;
                let d = (dx * dx + dy * dy).sqrt() / reach;
                if d >= 1.0 {
                    continue;
                }
                let f = (1.0 - d) * (1.0 - d) * look.bloom * node.b;
                if f < 0.22 {
                    continue;
                }
                let ch = if f > 0.4 { '.' } else { '`' };
                grid[y][x] = Cell::new(ch, lerp_color(look.dome_hi, look.vine_a, f));
            }
        }
    }
}

impl Mode for Shamsa {
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

#[cfg(test)]
mod tests {

    #[test]
    fn strap_knob_changes_straps() {
        let mut thin = knobs();
        thin[6] = 0.04;
        let mut thick = knobs();
        thick[6] = 0.28;
        assert_ne!(
            text(&frame(90, 30, 42, 0.0, &thin)),
            text(&frame(90, 30, 42, 0.0, &thick))
        );
    }
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
    fn shamsa_seed42() {
        insta::assert_snapshot!("shamsa_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn light_sweeps_with_time() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 5.0, &k)));
    }

    #[test]
    fn small_grids_stay_bounded() {
        let k = knobs();
        for (w, h) in [(10usize, 6usize), (3usize, 2usize), (1usize, 1usize)] {
            let _ = text(&frame(w, h, 42, 0.0, &k));
        }
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
        eprintln!("shamsa frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}

