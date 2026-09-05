use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::Cell;
use std::f32::consts::TAU;

pub(super) struct VesperMode;
pub(super) static MODE: VesperMode = VesperMode;
const PARAMS: &[Param] = &[
    param!("FOLDS", "shell folds", 2.0, 9.0, 5.0, 1.0),
    param!("THREADS", "luminous filaments", 24.0, 160.0, 72.0, 4.0),
    param!("OPEN", "shell aperture", 0.15, 0.65, 0.38, 0.02),
    param!("SPEED", "orbital clock", 0.0, 2.0, 0.4, 0.05),
];
const DOTS: [[u8; 4]; 2] = [[1, 2, 4, 64], [8, 16, 32, 128]];

impl Mode for VesperMode {
    fn name(&self) -> &'static str {
        "vesper"
    }
    fn help(&self) -> &'static str {
        "An eclipsed sun cradled in a folded luminous orbital shell [folds] [threads] [open] [speed]"
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        // Resolve bounded controls and the analytic clock for this frame.
        let read = |i: usize| {
            let p = &PARAMS[i];
            let v = frame
                .args
                .get(i + 4)
                .and_then(|s| s.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(p.key, p.default));
            if v.is_finite() {
                v.clamp(p.min, p.max)
            } else {
                p.default
            }
        };
        let folds = read(0).round();
        let threads = read(1) as usize;
        let aperture = read(2);
        let speed = read(3);
        let t = if frame.time.is_finite() {
            frame.time.rem_euclid(3600.0) * speed
        } else {
            0.0
        };
        let (w, h) = (frame.width, frame.height);
        if w == 0 || h == 0 {
            return;
        }
        let scale = (w as f32 / 2.0).min(h as f32) * 0.39;
        let ink = darken(frame.palette[0], 6);
        let pearl = frame.palette[4];
        let cold = frame.palette[2];
        let phase = (frame.seed % 997) as f32 * 0.0063;
        let mut dots = vec![0u8; w * h];
        let mut depths = vec![f32::NEG_INFINITY; w * h];
        let mut lights = vec![0.0f32; w * h];
        // Paint a dark stellar disk, hot corona, diffraction rays, and stars.
        measure_layer("vesper", "eclipse", || {
            for (y, row) in frame.grid.iter_mut().enumerate().take(h) {
                let py = (y as f32 + 0.5 - h as f32 * 0.5) / scale;
                for (x, cell) in row.iter_mut().enumerate().take(w) {
                    let px = (x as f32 + 0.5 - w as f32 * 0.5) / (2.0 * scale);
                    let r = px.hypot(py);
                    let corona = (1.0 - (r - 0.305).abs() / 0.12).max(0.0).powi(3);
                    let ray = (1.0 - py.abs() / 0.018).max(0.0) / (1.0 + px * px * 3.0);
                    *cell =
                        Cell::with_bg(' ', cold, lerp_color(ink, pearl, corona * 0.3 + ray * 0.12));
                    if r < 0.285 {
                        cell.bg = ink;
                        depths[y * w + x] = (0.285 * 0.285 - r * r).sqrt();
                    } else if corona > 0.08 {
                        cell.ch = if corona > 0.6 { '◦' } else { '·' };
                        cell.fg = lerp_color(cold, pearl, corona);
                    } else {
                        let mut n = frame.seed.wrapping_add((y * w + x) as u64 * 7919);
                        n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                        n ^= n >> 27;
                        if n % 1601 < 3 {
                            cell.ch = if n % 7 == 0 { '+' } else { '·' };
                            cell.fg = lerp_color(ink, pearl, 0.35 + (n % 40) as f32 * 0.01);
                        }
                    }
                }
            }
        });
        // Sample closed filaments on a corrugated torus and tilt into view.
        // Sample count follows projected resolution; buffers last one frame.
        measure_layer("vesper", "filaments", || {
            let steps = ((scale * 30.0) as usize).max(240);
            let (tilt_s, tilt_c) = (0.72 + (t * 0.17).sin() * 0.16).sin_cos();
            let (roll_s, roll_c) = (-0.38f32).sin_cos();
            for j in 0..threads {
                let v = j as f32 / threads as f32 * TAU;
                let (vs, vc) = v.sin_cos();
                for i in 0..=steps {
                    let u = i as f32 / steps as f32 * TAU;
                    let ripple = (folds * u + t * 0.6 + phase).sin();
                    let tube = aperture * (1.0 + 0.20 * ripple);
                    let radius = 0.78 + tube * vc;
                    let (us, uc) = (u + t * 0.12).sin_cos();
                    let a = radius * uc;
                    let b = radius * us;
                    let z = tube * vs + 0.16 * ripple;
                    let by = b * tilt_s - z * tilt_c;
                    let depth = b * tilt_c + z * tilt_s;
                    let px = a * roll_c - by * roll_s;
                    let py = a * roll_s + by * roll_c;
                    let sx = ((w as f32 * 0.5 + px * scale * 2.0) * 2.0) as isize;
                    let sy = ((h as f32 * 0.5 + py * scale) * 4.0) as isize;
                    if sx < 0 || sy < 0 || sx >= (w * 2) as isize || sy >= (h * 4) as isize {
                        continue;
                    }
                    let (sx, sy) = (sx as usize, sy as usize);
                    let index = sy / 4 * w + sx / 2;
                    if depth < depths[index] - 0.07 {
                        continue;
                    }
                    if depth > depths[index] + 0.07 {
                        dots[index] = 0;
                    }
                    depths[index] = depth;
                    dots[index] |= DOTS[sx % 2][sy % 4];
                    lights[index] = (0.48 + 0.28 * vs + 0.20 * (u * 2.0 - t).cos()).clamp(0.1, 1.0);
                }
            }
        });
        // Composite braille coverage using a precomputed palette ramp.
        measure_layer("vesper", "composite", || {
            let ramp: Vec<_> = (0..256)
                .map(|i| lerp_color(cold, pearl, i as f32 / 255.0))
                .collect();
            for (y, row) in frame.grid.iter_mut().enumerate().take(h) {
                for (x, cell) in row.iter_mut().enumerate().take(w) {
                    let k = y * w + x;
                    if dots[k] != 0 {
                        cell.ch = char::from_u32(0x2800 + dots[k] as u32).unwrap();
                        cell.fg = ramp[(lights[k] * 255.0) as usize];
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morph::IterateFrameRenderer;
    use crate::render::grid_to_plain;

    #[test]
    fn vesper_snapshots_and_frame_lifecycle() {
        let mut player = IterateFrameRenderer::new("vesper", 42, "deep", 100, 36).unwrap();
        let values: Vec<_> = PARAMS.iter().map(|p| p.default).collect();
        let first = player.render(0.0, Some(&values)).unwrap().clone();
        insta::assert_snapshot!("vesper_seed_42", grid_to_plain(&first).join("\n"));
        let moved = player.render(4.0, Some(&values)).unwrap().clone();
        insta::assert_snapshot!("vesper_motion", grid_to_plain(&moved).join("\n"));
        assert_ne!(grid_to_plain(&first), grid_to_plain(&moved));
        assert_eq!(&first, player.render(0.0, Some(&values)).unwrap());
        let mut stopped = values.clone();
        stopped[3] = 0.0;
        assert_eq!(&first, player.render(8.0, Some(&stopped)).unwrap());
        for i in 0..3 {
            let mut changed = values.clone();
            changed[i] = PARAMS[i].max;
            assert_ne!(&first, player.render(0.0, Some(&changed)).unwrap());
        }
    }

    #[test]
    fn vesper_small_grids_and_extreme_controls() {
        for (w, h) in [(1, 1), (2, 9), (9, 2), (80, 24)] {
            let mut player = IterateFrameRenderer::new("vesper", u64::MAX, "deep", w, h).unwrap();
            for values in [
                PARAMS.iter().map(|p| p.min).collect::<Vec<_>>(),
                PARAMS.iter().map(|p| p.max).collect(),
                vec![f32::NAN; 4],
            ] {
                for t in [0.0, -10.0, f32::MAX, f32::NAN] {
                    let grid = player.render(t, Some(&values)).unwrap();
                    assert_eq!(grid.len(), h);
                    assert!(
                        grid_to_plain(grid)
                            .iter()
                            .all(|r| crate::types::display_width(r) == w)
                    );
                }
            }
        }
    }
}
