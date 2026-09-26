use crate::_0_profile::measure_layer;
use crate::color::lerp_color;
use crate::opts::param_f32;
use crate::pp::{pp_put, pp_vnoise};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct Moire;
pub(super) static MODE: Moire = Moire;

const NAME: &str = "moire";
const HELP: &str = "moire: concentric-ring moire [sa] [sb] [sep] [angle] [warp] [contrast] [spin]";

const PARAMS: &[Param] = &[
    param!("SA", "ring A spacing", 3.0, 28.0, 9.0, 1.0),
    param!("SB", "ring B spacing", 3.0, 28.0, 13.0, 1.0),
    param!("SEP", "centre separation", 0.0, 90.0, 28.0, 1.0),
    param!("ANG", "axis angle", 0.0, 360.0, 25.0, 1.0),
    param!("WARP", "noise warp", 0.0, 24.0, 6.0, 1.0),
    param!("CONTRAST", "moire contrast", 0.2, 3.0, 1.3, 0.1),
    param!("SPIN", "spin degrees/s", 0.0, 180.0, 40.0, 1.0),
];

/// Deterministic per-cell value, independent of the frame RNG stream.
fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn unit(seed: u64, layer: u64, idx: u64) -> f32 {
    let h = splitmix(seed ^ splitmix(layer.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ idx));
    ((h >> 11) as f64 / (1u64 << 53) as f64) as f32
}

/// Proximity to the nearest ring: 1.0 exactly on a ring, 0.0 midway between.
fn ring(dist: f32, spacing: f32) -> f32 {
    1.0 - (std::f32::consts::PI * dist / spacing).sin().abs()
}

const RAMP: &[u8] = b".:-=+*#%@";

fn ramp(m: f32) -> char {
    let i = (m.clamp(0.0, 1.0) * (RAMP.len() as f32 - 1.0)).round() as usize;
    RAMP[i] as char
}

struct Knobs {
    sa: f32,
    sb: f32,
    sep: f32,
    ang: f32,
    warp: f32,
    contrast: f32,
    spin: f32,
}

fn knob(frame: &ModeFrame<'_>, i: usize, default: f32, lo: f32, hi: f32) -> f32 {
    frame
        .args
        .get(i + 4)
        .and_then(|s| s.parse::<f32>().ok())
        .or_else(|| frame.param_values.and_then(|v| v.get(i).copied()))
        .unwrap_or_else(|| param_f32(PARAMS[i].key, default))
        .clamp(lo, hi)
}

fn knobs(frame: &ModeFrame<'_>) -> Knobs {
    Knobs {
        sa: knob(frame, 0, 9.0, 3.0, 28.0),
        sb: knob(frame, 1, 13.0, 3.0, 28.0),
        sep: knob(frame, 2, 28.0, 0.0, 90.0),
        ang: knob(frame, 3, 25.0, 0.0, 360.0),
        warp: knob(frame, 4, 6.0, 0.0, 24.0),
        contrast: knob(frame, 5, 1.3, 0.2, 3.0),
        spin: knob(frame, 6, 40.0, 0.0, 180.0),
    }
}

fn draw(frame: &mut ModeFrame<'_>, k: &Knobs) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let seed = frame.seed;
    let grid = &mut *frame.grid;
    let pal = frame.palette;

    // Time rotates the axis between the two ring centres.
    let ang = (k.ang + frame.time * k.spin).to_radians();
    let cx = w as f32; // visual space doubles x for the 2:1 cell aspect
    let cy = h as f32 * 0.5;
    let bx = cx + k.sep * ang.cos();
    let by = cy + k.sep * ang.sin();

    let mut pa = vec![vec![0.0f32; w]; h];
    let mut pb = vec![vec![0.0f32; w]; h];
    let mut beat = vec![vec![0.0f32; w]; h];

    measure_layer(NAME, "field", || {
        for y in 0..h {
            for x in 0..w {
                let vx = x as f32 * 2.0;
                let vy = y as f32;
                let warp = if k.warp > 0.01 {
                    (pp_vnoise(x as f32 * 0.07, y as f32 * 0.14, seed) - 0.5) * k.warp
                } else {
                    0.0
                };
                let ra = ((vx - cx).powi(2) + (vy - cy).powi(2)).sqrt() + warp;
                let rb = ((vx - bx).powi(2) + (vy - by).powi(2)).sqrt() + warp;
                let qa = ring(ra, k.sa);
                let qb = ring(rb, k.sb);
                pa[y][x] = qa;
                pb[y][x] = qb;
                beat[y][x] = (qa * qb).powf(k.contrast);
            }
        }
    });

    measure_layer(NAME, "ground", || {
        for y in 0..h {
            for x in 0..w {
                let dx = (x as f32 * 2.0 - w as f32) / w as f32;
                let dy = (y as f32 - h as f32 * 0.5) / h as f32;
                let t = (dx * dx + dy * dy).sqrt().min(1.0);
                let bg = lerp_color(pal[0], pal[2], t * 0.30);
                let dust = unit(seed, 10, (y * w + x) as u64) > 0.985;
                let ch = if dust { '.' } else { ' ' };
                let fg = lerp_color(pal[0], pal[1], 0.55);
                grid[y][x] = Cell::with_bg(ch, fg, bg);
            }
        }
    });

    measure_layer(NAME, "rings", || {
        for y in 0..h {
            for x in 0..w {
                let qa = pa[y][x];
                let qb = pb[y][x];
                if qa > 0.62 {
                    let ch = if qa > 0.9 { ':' } else { '.' };
                    pp_put(grid, x as i32, y as i32, ch, lerp_color(pal[0], pal[2], qa));
                } else if qb > 0.62 {
                    let ch = if qb > 0.9 { ':' } else { '.' };
                    pp_put(grid, x as i32, y as i32, ch, lerp_color(pal[0], pal[1], qb));
                }
            }
        }
    });

    measure_layer(NAME, "beat", || {
        for y in 0..h {
            for x in 0..w {
                let m = beat[y][x];
                if m > 0.30 {
                    pp_put(
                        grid,
                        x as i32,
                        y as i32,
                        ramp(m),
                        lerp_color(pal[1], pal[3], m),
                    );
                }
            }
        }
    });

    measure_layer(NAME, "nodes", || {
        for y in 0..h {
            for x in 0..w {
                let m = beat[y][x];
                if m > 0.86 {
                    pp_put(grid, x as i32, y as i32, '@', pal[4]);
                }
            }
        }
    });
}

impl Mode for Moire {
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
        let k = knobs(frame);
        draw(frame, &k);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn render_at(w: usize, h: usize, seed: u64, time: f32, args: &[&str]) -> String {
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut frame = ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &argv,
            param_values: None,
        };
        MODE.render(&mut frame);
        grid.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_80x24() {
        insta::assert_snapshot!("moire_80x24", render_at(80, 24, 42, 0.0, &[]));
    }

    #[test]
    fn snapshot_t6_extra() {
        insta::assert_snapshot!("moire_t6", render_at(80, 24, 42, 6.0, &[]));
    }

    #[test]
    fn deterministic() {
        assert_eq!(
            render_at(80, 24, 42, 0.0, &[]),
            render_at(80, 24, 42, 0.0, &[])
        );
    }

    #[test]
    fn seed_sensitive() {
        assert_ne!(
            render_at(80, 24, 42, 0.0, &[]),
            render_at(80, 24, 43, 0.0, &[])
        );
    }

    #[test]
    fn time_sensitive() {
        assert_ne!(
            render_at(80, 24, 42, 0.0, &[]),
            render_at(80, 24, 42, 6.0, &[])
        );
    }

    #[test]
    fn frame_cost() {
        if cfg!(debug_assertions) {
            return;
        }
        let (w, h) = (200usize, 60usize);
        let args = vec![String::new(); 7];
        let palette = make_palette(42);
        let mut rng = StdRng::seed_from_u64(42);
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];
        let start = std::time::Instant::now();
        let iters = 40;
        for i in 0..iters {
            let mut frame = ModeFrame {
                grid: &mut grid,
                width: w,
                height: h,
                seed: 42,
                palette: &palette,
                rng: &mut rng,
                time: i as f32,
                args: &args,
                param_values: None,
            };
            MODE.render(&mut frame);
        }
        let avg_ms = start.elapsed().as_secs_f64() * 1000.0 / iters as f64;
        assert!(avg_ms < 6.0, "moire average frame {avg_ms:.3} ms exceeds 6 ms");
    }
}
