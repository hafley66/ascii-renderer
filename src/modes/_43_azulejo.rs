use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use std::f32::consts::{PI, TAU};

pub(super) struct AzulejoMode;
pub(super) static MODE: AzulejoMode = AzulejoMode;
const PARAMS: &[Param] = &[
    param!(
        "BAYS",
        "framed bays; zero chooses by seed",
        0.0,
        4.0,
        0.0,
        1.0
    ),
    param!(
        "SCALE",
        "tile courses across the facade",
        3.0,
        12.0,
        4.0,
        0.25
    ),
    param!(
        "PATTERN",
        "seed / octagon / cross / braid / rosette / stepped",
        0.0,
        5.0,
        0.0,
        1.0
    ),
    param!("ARCH", "pointed arch depth", 0.0, 1.0, 0.55, 0.05),
    param!(
        "INSET",
        "star valleys and ribbon width",
        0.15,
        0.85,
        0.46,
        0.025
    ),
    param!("FLOW", "tile-course travel", -1.0, 1.0, 0.12, 0.05),
    param!("TURN", "inlay rotation", -1.0, 1.0, 0.08, 0.05),
    param!("BREATH", "inlay folding amplitude", 0.0, 1.0, 0.35, 0.05),
    param!(
        "RELIEF",
        "glaze, bevel and ceramic grain",
        0.0,
        1.0,
        0.65,
        0.05
    ),
    param!("SPEED", "ornament clock", 0.0, 2.0, 0.5, 0.05),
    param!("BOND", "sheared tile bond", 0.0, 1.0, 0.0, 0.05),
];

impl Mode for AzulejoMode {
    fn name(&self) -> &'static str {
        "azulejo"
    }
    fn help(&self) -> &'static str {
        "Glazed architectural tessellations: five inlay families, flowing courses and folding stars [bays] [scale] [pattern] [arch] [inset] [flow] [turn] [breath] [relief] [speed] [bond]"
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        let k = std::array::from_fn(|i| {
            let p = &PARAMS[i];
            let v = frame
                .args
                .get(i + 4)
                .and_then(|v| v.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(p.key, p.default));
            if v.is_finite() {
                v.clamp(p.min, p.max)
            } else {
                p.default
            }
        });
        draw_azulejo(frame, &k);
    }
}

fn hash(mut n: u64) -> u64 {
    n ^= n >> 30;
    n = n.wrapping_mul(0xbf58476d1ce4e5b9);
    n ^= n >> 27;
    n = n.wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}

// Pair sorted scanline intersections, including concave shoulders at deep inset.
// Clip every span against the panel arch and grid. Identity controls ceramic tone.
fn polygon(
    grid: &mut Grid,
    clip: &[(usize, usize)],
    points: &[[f32; 2]],
    colors: &[Cell; 8],
    identity: u64,
    relief: f32,
    grain: usize,
) {
    let h = grid.len();
    let mut p = [[0.0; 2]; 16];
    let center = points
        .iter()
        .fold([0.0, 0.0], |a, b| [a[0] + b[0], a[1] + b[1]]);
    let center = [
        center[0] / points.len() as f32,
        center[1] / points.len() as f32,
    ];
    for (out, src) in p.iter_mut().zip(points) {
        *out = [
            center[0] + (src[0] - center[0]) * 0.958,
            center[1] + (src[1] - center[1]) * 0.958,
        ];
    }
    let p = &p[..points.len()];
    let y0 = p
        .iter()
        .map(|p| p[1])
        .fold(f32::INFINITY, f32::min)
        .ceil()
        .max(0.0) as usize;
    let y1 = (p
        .iter()
        .map(|p| p[1])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .max(0.0) as usize)
        .min(h);
    for y in y0..y1 {
        let yy = y as f32 + 0.5;
        let mut crossings = [0.0f32; 16];
        let mut count = 0;
        for i in 0..p.len() {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            if (a[1] <= yy && b[1] > yy) || (b[1] <= yy && a[1] > yy) {
                let x = a[0] + (yy - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
                crossings[count] = x;
                count += 1;
            }
        }
        crossings[..count].sort_unstable_by(f32::total_cmp);
        for pair in crossings[..count].chunks_exact(2) {
            let [left, right] = [pair[0], pair[1]];
            let lo = ((left - 0.5).ceil().max(0.0) as usize).max(clip[y].0);
            let hi = ((right - 0.5).ceil().max(0.0) as usize)
                .min(clip[y].1)
                .min(grid[y].len());
            if lo >= hi {
                continue;
            }
            let tone = ((identity as usize).wrapping_add(y / grain) & 3) + 2;
            grid[y][lo..hi].fill(colors[tone]);
            if relief > 0.05 {
                for x in (lo + ((identity as usize).wrapping_add(y) % grain)..hi).step_by(grain) {
                    if (identity as usize).wrapping_add(x / grain + y / grain) % 7 == 0 {
                        grid[y][x] = colors[1];
                    }
                }
                grid[y][lo] = colors[7];
                grid[y][hi - 1] = colors[0];
            }
        }
    }
    // Terminal-sized edges remain readable where polygon fills are sub-cell.
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let n = dx.abs().max(dy.abs()).ceil() as usize;
        let ch = if dx.abs() > dy.abs() * 2.4 {
            '─'
        } else if dy.abs() > dx.abs() * 1.2 {
            '│'
        } else if dx * dy > 0.0 {
            '╲'
        } else {
            '╱'
        };
        for j in 0..=n {
            let f = j as f32 / n.max(1) as f32;
            let x = a[0] + dx * f;
            let y = a[1] + dy * f;
            if x < 0.0 || y < 0.0 {
                continue;
            }
            let (x, y) = (x as usize, y as usize);
            if y < h && x >= clip[y].0 && x < clip[y].1 && x < grid[y].len() {
                let mut c = colors[if dx - 2.0 * dy > 0.0 { 7 } else { 0 }];
                c.ch = ch;
                grid[y][x] = c;
            }
        }
    }
}

fn draw_azulejo(frame: &mut ModeFrame<'_>, k: &[f32; 11]) {
    // Resolve finite time, seed-selected architectural layout and material ramps.
    // Fill bonded brick courses, then build clipped arch spans per panel.
    // Stamp adjacent octagons, square joints and convex inlay tesserae.
    // Evaluate course travel, local folding and glaze from the current time only.
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let seed = frame.seed;
    let t = if frame.time.is_finite() {
        frame.time.rem_euclid(3600.0) * k[9]
    } else {
        0.0
    };
    let bays = if k[0] < 0.5 {
        1 + seed as usize % 3
    } else {
        k[0].round() as usize
    };
    let family = if k[2] < 0.5 {
        1 + (seed as usize / 4) % 5
    } else {
        k[2].round() as usize
    };
    let ink = darken(frame.palette[0], 14);
    let gold = frame.palette[4];
    let mortar = lerp_color(ink, gold, 0.25);
    let palettes: [[Cell; 8]; 5] = std::array::from_fn(|band| {
        let color = frame.palette[[1, 3, 2, 4, 2][band]];
        std::array::from_fn(|tone| {
            let base = lerp_color(
                ink,
                color,
                if band == 4 {
                    0.09 + tone as f32 * 0.012
                } else {
                    0.28 + tone as f32 * 0.065
                },
            );
            let fg = lerp_color(base, gold, 0.22 + tone as f32 * 0.075);
            Cell::with_bg(if tone == 1 { '·' } else { ' ' }, fg, base)
        })
    });
    let unit = (w as f32 / k[1]).max(2.0);
    let sy = 0.5 * unit;
    let grain = (w / 150).max(2);
    measure_layer("azulejo", "masonry", || {
        let course = (h / 19).max(1);
        let brick = (w / 11).max(3);
        for (y, row) in frame.grid.iter_mut().enumerate() {
            let offset = (y / course % 2) * (brick / 2);
            for (j, chunk) in row.chunks_mut(brick).enumerate() {
                let tone = hash(seed ^ (y / course) as u64 ^ (j as u64 * 991)) as usize & 3;
                chunk.fill(Cell::with_bg(
                    ' ',
                    mortar,
                    lerp_color(ink, frame.palette[3], 0.12 + tone as f32 * 0.022),
                ));
            }
            if y % course == 0 && course > 1 {
                row.fill(Cell::with_bg('─', mortar, ink));
            } else {
                for x in (offset..w).step_by(brick) {
                    row[x] = Cell::with_bg('│', mortar, ink);
                }
            }
        }
    });
    for bay in 0..bays {
        let left = w as f32 * (0.035 + bay as f32 * 0.93 / bays as f32);
        let right = w as f32 * (0.035 + (bay + 1) as f32 * 0.93 / bays as f32) - w as f32 * 0.018;
        let cx = (left + right) * 0.5;
        let half = (right - left) * 0.5;
        let top = h as f32 * (0.045 + if bays > 1 && bay % 2 == 1 { 0.07 } else { 0.0 });
        let bottom = h as f32 * 0.91;
        let arch = (bottom - top) * 0.42 * k[3];
        let border = (w as f32 * 0.017).max(1.0).min(half * 0.25);
        let mut clip = vec![(0, 0); h];
        measure_layer("azulejo", "surround", || {
            for y in top.ceil() as usize..(bottom.ceil() as usize).min(h) {
                let rise = ((top + arch - y as f32) / arch.max(0.01)).clamp(0.0, 1.0);
                let extent = half * (1.0 - rise * rise);
                let l = (cx - extent).max(0.0) as usize;
                let r = ((cx + extent).ceil() as usize).min(w);
                if l >= r {
                    continue;
                }
                let row = &mut frame.grid[y];
                row[l..r].fill(Cell::with_bg(' ', gold, mortar));
                let b = border.ceil() as usize;
                let lo = (l + b).min(r);
                let hi = r.saturating_sub(b).max(lo);
                if y as f32 > top + border * 0.5 && (y as f32) < bottom - border * 0.5 {
                    row[lo..hi].fill(Cell::with_bg(' ', mortar, ink));
                    clip[y] = (lo, hi);
                }
                let glyph = if rise > 0.02 {
                    if (y / grain) % 2 == 0 { '╱' } else { '╲' }
                } else {
                    '◆'
                };
                row[l] = Cell::with_bg(glyph, gold, ink);
                row[r - 1] = Cell::with_bg(glyph, gold, ink);
                if b > 2 {
                    row[(l + b / 2).min(r - 1)].ch =
                        if (y / grain) % 2 == 0 { '╲' } else { '╱' };
                    row[r - 1 - b / 2].ch = if (y / grain) % 2 == 0 { '╱' } else { '╲' };
                }
            }
            // Alternating diamond frieze and three horizontal stone courses.
            for band in 0..3 {
                let y = (h as f32 * (0.935 + band as f32 * 0.023)) as usize;
                if y >= h {
                    continue;
                }
                for x in left.max(0.0) as usize..(right as usize).min(w) {
                    frame.grid[y][x] = Cell::with_bg(
                        if band == 1 && (x / grain) % 3 == 0 {
                            '◆'
                        } else {
                            '─'
                        },
                        gold,
                        ink,
                    );
                }
            }
        });
        measure_layer("azulejo", "tesserae", || {
            let travel = t * k[5] * 0.35;
            let xshift = travel.rem_euclid(1.0);
            let yshift = (travel * 0.37).rem_euclid(1.0);
            let rows = ((bottom - top) / sy).ceil() as i32 + 3;
            let cols = ((right - left) / unit).ceil() as i32 + 3;
            let oct: [[f32; 2]; 8] = [
                [-0.20710678, -0.5],
                [0.20710678, -0.5],
                [0.5, -0.20710678],
                [0.5, 0.20710678],
                [0.20710678, 0.5],
                [-0.20710678, 0.5],
                [-0.5, 0.20710678],
                [-0.5, -0.20710678],
            ];
            for row in -1..rows {
                for col in -1..cols {
                    let world_row = row - (travel * 0.37).floor() as i32;
                    let world_col = col - travel.floor() as i32;
                    let id = hash(
                        seed ^ (world_row as i64 as u64).wrapping_mul(7919)
                            ^ (world_col as i64 as u64).wrapping_mul(104729)
                            ^ bay as u64,
                    );
                    let x = cx
                        + (col as f32 - cols as f32 * 0.5
                            + xshift
                            + (world_row as f32 * k[10] * 0.5).rem_euclid(1.0))
                            * unit;
                    let y = top + (row as f32 + yshift) * sy;
                    let band = ((world_row + world_col).rem_euclid(2) as usize
                        + (seed as usize / 7) % 3)
                        % 4;
                    let transform =
                        |px: f32, py: f32| [x + (px + py * k[10] * 0.5) * unit, y + py * sy];
                    let base = oct.map(|p| transform(p[0], p[1]));
                    polygon(frame.grid, &clip, &base, &palettes[4], id, k[8], grain);
                    // Shared diamond joints close the regular octagon lattice.
                    let joint = [
                        transform(0.5, 0.20710678),
                        transform(0.7928932, 0.5),
                        transform(0.5, 0.7928932),
                        transform(0.20710678, 0.5),
                    ];
                    polygon(frame.grid, &clip, &joint, &palettes[3], id, k[8], grain);
                    let joint_inlay = [[0.5, 0.34], [0.66, 0.5], [0.5, 0.66], [0.34, 0.5]]
                        .map(|p| transform(p[0], p[1]));
                    polygon(
                        frame.grid,
                        &clip,
                        &joint_inlay,
                        &palettes[(band + 2) % 3],
                        id,
                        k[8],
                        grain,
                    );
                    let fold = (t * 0.8 + bay as f32 * 0.7).sin() * k[7] * 0.17;
                    let depth = (k[4] + fold).clamp(0.12, 0.9);
                    let rotation = t * k[6] * 0.4 + (seed % 8) as f32 * PI * 0.125;
                    let (points, outer, angle) = match family {
                        1 => (8, 0.47, rotation),
                        2 => (4, 0.48, rotation + PI * 0.25),
                        3 => (
                            4,
                            0.47,
                            rotation + (world_row + world_col).rem_euclid(2) as f32 * PI * 0.25,
                        ),
                        4 => (12, 0.47, rotation),
                        _ => (4, 0.47, rotation),
                    };
                    for j in 0..points {
                        let a = angle + j as f32 * TAU / points as f32;
                        let da = PI / points as f32;
                        let point = |r: f32, a: f32| transform(a.cos() * r, a.sin() * r);
                        let tip = point(outer, a);
                        let before = point(outer * depth, a - da);
                        let after = point(outer * depth, a + da);
                        let inner = if family == 5 {
                            point(outer * depth * 0.7, a)
                        } else {
                            [x, y]
                        };
                        let shape = [inner, before, tip, after];
                        // The shoulder shares both arm edges. Its outer vertex lies
                        // on the octagon, closing the star's negative-space pockets.
                        let mid = a + da;
                        let (sn, cs) = mid.sin_cos();
                        let radius =
                            (0.5 / sn.abs().max(cs.abs())).min(0.70710678 / (sn.abs() + cs.abs()));
                        let shoulder = [after, tip, point(radius, mid), point(outer, a + 2.0 * da)];
                        let mut shoulder_glaze = palettes[(band + 2) % 4];
                        for c in &mut shoulder_glaze {
                            c.bg = lerp_color(ink, c.bg, 0.62);
                            c.fg = lerp_color(ink, c.fg, 0.70);
                        }
                        polygon(
                            frame.grid,
                            &clip,
                            &shoulder,
                            &shoulder_glaze,
                            id.wrapping_add(j as u64 + 31),
                            k[8],
                            grain,
                        );

                        let color = if family == 3 {
                            (band + j % 2) % 4
                        } else if j % 2 == 0 {
                            band
                        } else {
                            (band + 1) % 4
                        };
                        let mut glaze = palettes[color];
                        let light =
                            ((t * 0.6 + j as f32 * 0.6 + world_col as f32 * 0.4).sin() * 0.5 + 0.5)
                                * k[8];
                        for c in &mut glaze {
                            c.fg = lerp_color(c.fg, gold, light * 0.35);
                        }
                        polygon(
                            frame.grid,
                            &clip,
                            &shape,
                            &glaze,
                            id.wrapping_add(j as u64),
                            k[8],
                            grain,
                        );
                    }
                    // Seeded central enamel cabochon anchors the folding wedges.
                    let r = if family == 4 { 0.13 } else { 0.075 };
                    let center = [
                        transform(0.0, -r),
                        transform(r, 0.0),
                        transform(0.0, r),
                        transform(-r, 0.0),
                    ];
                    polygon(frame.grid, &clip, &center, &palettes[3], id, k[8], grain);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{morph::IterateFrameRenderer, render::grid_to_plain};
    fn defaults() -> Vec<f32> {
        PARAMS.iter().map(|p| p.default).collect()
    }
    #[test]
    fn azulejo_snapshots() {
        let mut renderer = IterateFrameRenderer::new("azulejo", 42, "moss", 100, 36).unwrap();
        let k = defaults();
        let a = renderer.render(0.0, Some(&k)).unwrap().clone();
        let b = renderer.render(7.0, Some(&k)).unwrap().clone();
        insta::assert_snapshot!("azulejo_seed42", grid_to_plain(&a).join("\n"));
        insta::assert_snapshot!("azulejo_time7", grid_to_plain(&b).join("\n"));
        assert_ne!(a, b);
        assert_eq!(&a, renderer.render(0.0, Some(&k)).unwrap());
        let mut stopped = k.clone();
        stopped[9] = 0.0;
        assert_eq!(&a, renderer.render(17.0, Some(&stopped)).unwrap());
        for i in 0..k.len() {
            let mut changed = k.clone();
            changed[i] = if i == 9 { 2.0 } else { PARAMS[i].max };
            assert_ne!(
                &b,
                renderer.render(7.0, Some(&changed)).unwrap(),
                "parameter {}",
                PARAMS[i].key
            );
        }
        let mut other = IterateFrameRenderer::new("azulejo", 43, "moss", 100, 36).unwrap();
        assert_ne!(&a, other.render(0.0, Some(&k)).unwrap());
    }
    #[test]
    #[ignore = "writes review artifacts to AZULEJO_PREVIEW_DIR"]
    fn azulejo_export_gallery() {
        let dir = std::env::var("AZULEJO_PREVIEW_DIR").expect("AZULEJO_PREVIEW_DIR");
        std::fs::create_dir_all(&dir).unwrap();
        let mut manifest = String::new();
        for (name, w, h, k, t) in [
            ("default100", 100, 36, defaults(), 0.0),
            ("default160", 160, 60, defaults(), 0.0),
            ("large2000", 2000, 1000, defaults(), 0.0),
        ]
        .into_iter()
        .chain((0..12u64).flat_map(|roll| {
            let s = 42 ^ roll.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let k: Vec<_> = PARAMS
                .iter()
                .map(|p| crate::opts::rand_knob(s, p))
                .collect();
            [
                ("random", 100, 36, k.clone(), 0.0),
                ("motion", 100, 36, k, 9.0),
            ]
        }))
        .enumerate()
        .map(|(i, (name, w, h, k, t))| (format!("{i:02}_{name}"), w, h, k, t))
        {
            let mut renderer = IterateFrameRenderer::new("azulejo", 42, "deep", w, h).unwrap();
            let grid = renderer.render(t, Some(&k)).unwrap();
            std::fs::write(
                format!("{dir}/{name}.grid"),
                crate::gridio::serialize_grid(grid),
            )
            .unwrap();
            manifest.push_str(&format!("{name} {w}x{h} t={t} {k:?}\n"));
        }
        std::fs::write(format!("{dir}/manifest.txt"), manifest).unwrap();
    }

    #[test]
    fn azulejo_input_contract() {
        use rand::SeedableRng;
        let k = defaults();
        let mut renderer = IterateFrameRenderer::new("azulejo", 42, "moss", 80, 24).unwrap();
        let expected = renderer.render(0.0, Some(&k)).unwrap().clone();
        assert_eq!(
            &expected,
            renderer.render(f32::NAN, Some(&[f32::NAN; 11])).unwrap()
        );
        assert_eq!(
            &expected,
            renderer
                .render(f32::INFINITY, Some(&[f32::INFINITY; 11]))
                .unwrap()
        );
        let mut themed = IterateFrameRenderer::new("azulejo", 42, "deep", 80, 24).unwrap();
        assert_ne!(&expected, themed.render(0.0, Some(&k)).unwrap());
        for (w, h) in [(0, 0), (0, 3), (3, 0)] {
            let mut grid = vec![vec![Cell::blank(); w]; h];
            let palette = crate::color::make_palette(42);
            let mut rng = rand::rngs::StdRng::seed_from_u64(42);
            MODE.render(&mut ModeFrame {
                grid: &mut grid,
                width: w,
                height: h,
                seed: 42,
                palette: &palette,
                rng: &mut rng,
                time: 0.0,
                args: &[],
                param_values: Some(&k),
            });
            assert_eq!(grid, vec![vec![Cell::blank(); w]; h]);
        }
    }

    #[test]
    #[ignore = "isolated timed 2000x1000 construction and exact demo reroll cases"]
    fn azulejo_cold_and_random_cost() {
        use std::time::Instant;
        println!("| case | construct ms | first render ms | avg ms | p50 ms | p99 ms | max ms |");
        println!("|---|---:|---:|---:|---:|---:|---:|");
        for case in 0..14u64 {
            let k = if case == 0 {
                defaults()
            } else if case == 1 {
                PARAMS.iter().map(|p| p.max).collect()
            } else {
                let s = 42 ^ (case - 2).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                PARAMS
                    .iter()
                    .map(|p| crate::opts::rand_knob(s, p))
                    .collect()
            };
            let start = Instant::now();
            let mut renderer =
                IterateFrameRenderer::new("azulejo", 42, "moss", 2000, 1000).unwrap();
            let construct = start.elapsed().as_secs_f64() * 1000.0;
            let start = Instant::now();
            std::hint::black_box(renderer.render(0.0, Some(&k)).unwrap());
            let cold = start.elapsed().as_secs_f64() * 1000.0;
            let mut samples = Vec::new();
            for i in 1..=100 {
                let start = Instant::now();
                std::hint::black_box(renderer.render(i as f32 * 0.06, Some(&k)).unwrap());
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            let avg = samples.iter().sum::<f64>() / samples.len() as f64;
            samples.sort_by(f64::total_cmp);
            println!(
                "| {case} | {construct:.3} | {cold:.3} | {avg:.3} | {:.3} | {:.3} | {:.3} |",
                samples[49], samples[98], samples[99]
            );
        }
    }

    #[test]
    fn azulejo_boundaries_and_gallery() {
        for (w, h) in [(1, 1), (2, 9), (9, 2), (80, 24)] {
            let mut renderer =
                IterateFrameRenderer::new("azulejo", u64::MAX, "moss", w, h).unwrap();
            for k in [
                PARAMS.iter().map(|p| p.min).collect::<Vec<_>>(),
                PARAMS.iter().map(|p| p.max).collect(),
                vec![f32::NAN; 11],
                vec![f32::INFINITY; 11],
            ] {
                for t in [0.0, 8.0, f32::NAN, f32::INFINITY, -100.0] {
                    let a = renderer.render(t, Some(&k)).unwrap().clone();
                    assert_eq!(&a, renderer.render(t, Some(&k)).unwrap());
                    assert_eq!(a.len(), h);
                    assert!(a.iter().all(|r| r.len() == w));
                    assert!(
                        a.iter()
                            .flatten()
                            .all(|c| crate::types::char_width(c.ch) == 1)
                    );
                }
            }
        }
        for seed in 0..20u64 {
            let mut renderer = IterateFrameRenderer::new("azulejo", seed, "moss", 100, 36).unwrap();
            for roll in 0..4u64 {
                let s = seed ^ roll.wrapping_mul(0x9E37_79B9_7F4A_7C15);
                let k: Vec<_> = PARAMS
                    .iter()
                    .map(|p| crate::opts::rand_knob(s, p))
                    .collect();
                let a = renderer.render(0.0, Some(&k)).unwrap().clone();
                let b = renderer.render(9.0, Some(&k)).unwrap().clone();
                assert_eq!(&a, renderer.render(0.0, Some(&k)).unwrap());
                if k[9] > 0.0 {
                    assert_ne!(a, b, "seed {seed} roll {roll}");
                }
            }
        }
    }
}
