use crate::color::{darken, lerp_color};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::Cell;
use std::f32::consts::TAU;

pub(super) struct VoluteMode;
pub(super) static MODE: VoluteMode = VoluteMode;

const PARAMS: &[Param] = &[
    param!("SCALE", "shell radius", 0.45, 1.25, 1.0, 0.05),
    param!("COIL", "spiral expansion", 0.09, 0.22, 0.13, 0.01),
    param!("CHAMBERS", "chambers per revolution", 8.0, 32.0, 18.0, 1.0),
    param!("NACRE", "pearl striation relief", 0.0, 1.0, 0.65, 0.05),
    param!("TIDE", "surrounding current strength", 0.0, 1.0, 0.55, 0.05),
    param!("SPEED", "shell and current clock", 0.0, 2.0, 0.6, 0.05),
];

impl Mode for VoluteMode {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn name(&self) -> &'static str {
        "volute"
    }
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn help(&self) -> &'static str {
        "Chambered spiral shell in a flowing tide [scale] [coil] [chambers] [nacre] [tide] [speed]"
    }
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn render(&self, frame: &mut ModeFrame<'_>) {
        // Resolve positional arguments, native controls, then live/env defaults.
        // Sanitize each input before evaluating a frame with no retained state.
        let controls = std::array::from_fn(|i| {
            let p = &PARAMS[i];
            let value = frame
                .args
                .get(i + 4)
                .and_then(|v| v.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(p.key, p.default));
            if value.is_finite() {
                value.clamp(p.min, p.max)
            } else {
                p.default
            }
        });
        draw_volute(frame, &controls);
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn draw_volute(frame: &mut ModeFrame<'_>, controls: &[f32; 6]) {
    // Derive shell orientation and tidal phase from the explicit seed and time.
    // Evaluate four logarithmic whorls and curved chamber walls per grid cell.
    // Shade the shell cross-section; draw advecting current lines outside it.
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let [scale, coil, chambers, nacre, tide, speed] = *controls;
    let t = if frame.time.is_finite() {
        frame.time.clamp(-1_000_000.0, 1_000_000.0) * speed
    } else {
        0.0
    };
    let seed_phase =
        (frame.seed.wrapping_mul(0x9e3779b97f4a7c15) >> 40) as f32 / 16_777_216.0 * TAU;
    let orientation = -0.65 + seed_phase + 0.13 * (t * 0.37).sin();
    let radius = (w as f32 * 0.32).min(h as f32 * 0.76) * scale;
    let cx = w as f32 * 0.48;
    let cy = h as f32 * 0.49;
    // Adjacent whorls nearly touch, leaving a dark spiral suture between them.
    let thickness = (coil * TAU * 0.5).tanh() * 0.96;
    let shrink = (-coil * TAU).exp();
    let ink = darken(frame.palette[0], 18);
    let pearl = frame.palette[4];
    let current = lerp_color(ink, frame.palette[2], 0.42);
    let ramp = ['.', ':', '-', '=', '+', '*', '#', '%', '@'];
    // Quantized material ramps avoid per-cell color conversions in the shell.
    let material: [[Cell; 32]; 8] = std::array::from_fn(|band| {
        let pigment = lerp_color(frame.palette[1], frame.palette[3], band as f32 / 7.0);
        std::array::from_fn(|level| {
            let light = level as f32 / 31.0;
            Cell::with_bg(
                ramp[(light * 8.0) as usize],
                lerp_color(pigment, pearl, light),
                lerp_color(ink, pigment, 0.12 + light * 0.26),
            )
        })
    });
    for (y, row) in frame.grid.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            let px = x as f32 + 0.5 - cx;
            let py = (y as f32 + 0.5 - cy) * 2.0;
            let r = px.hypot(py);
            let angle = py.atan2(px);
            let theta = (angle - orientation).rem_euclid(TAU);
            let mut shell = None;
            let mut center_r = radius * (-coil * theta).exp();
            for turn in 0..4 {
                let d = (r - center_r) / (center_r * thickness).max(0.001);
                if d.abs() <= 1.0 {
                    shell = Some((theta + turn as f32 * TAU, d, center_r));
                    break;
                }
                center_r *= shrink;
            }
            *cell = Cell::with_bg(' ', current, ink);
            if let Some((a, d, local_radius)) = shell {
                let chamber = a * chambers / TAU + 0.32 * (d * d - d);
                let seam = chamber.rem_euclid(1.0);
                let wall = (0.17 * chambers / local_radius.max(1.0)).clamp(0.045, 0.23);
                let edge = (1.4 / (local_radius * thickness).max(1.0)).min(0.32);
                let rib = (a * 83.0 + d * 12.0 + seed_phase).sin();
                let shimmer = (a * 2.3 - d * 3.5 - t * 0.7).sin();
                let light = (0.28
                    + 0.43 * (1.0 - d * d).sqrt()
                    + 0.13 * (angle + 2.2).cos()
                    + nacre * (0.1 * rib + 0.12 * shimmer))
                    .clamp(0.0, 1.0);
                let band = ((chamber.floor() as i32).rem_euclid(8)) as usize;
                *cell = material[band][(light * 31.0) as usize];
                if d.abs() > 1.0 - edge {
                    cell.ch = if px.abs() > py.abs() * 1.4 {
                        '|'
                    } else if py.abs() > px.abs() * 1.4 {
                        '_'
                    } else if px * py > 0.0 {
                        '/'
                    } else {
                        '\\'
                    };
                    cell.fg = pearl;
                } else if seam < wall {
                    cell.ch = if py.abs() > px.abs() * 1.5 {
                        '|'
                    } else if px.abs() > py.abs() * 2.0 {
                        '-'
                    } else if px * py > 0.0 {
                        '\\'
                    } else {
                        '/'
                    };
                    cell.fg = lerp_color(pearl, frame.palette[2], 0.25);
                    cell.bg = ink;
                }
            } else if r < radius * (-coil * TAU * 4.0).exp() * 1.4 {
                *cell = Cell::with_bg('o', pearl, ink);
            } else if tide > 0.0 {
                // Stream function bends parallel currents around the shell.
                let u = px / radius.max(1.0);
                let v = py / radius.max(1.0);
                let stream = v * (1.0 - 0.65 / (u * u + v * v + 0.65))
                    + tide * 0.09 * (u * 4.0 - t * 0.8 + seed_phase).sin();
                let line = (stream * 9.0 + t * 0.16).rem_euclid(1.0);
                let dash = (u * 8.0 - t * 1.3 + stream * 3.0).rem_euclid(3.0);
                if line < 0.07 + 0.12 * tide && dash < 1.4 && r > radius * 0.32 {
                    cell.ch = if line < 0.055 { '~' } else { '.' };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{morph::IterateFrameRenderer, render::grid_to_plain};

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn volute_snapshots_and_motion() {
        let k: Vec<_> = PARAMS.iter().map(|p| p.default).collect();
        let mut renderer = IterateFrameRenderer::new("volute", 42, "deep", 100, 32).unwrap();
        let a = renderer.render(0.0, Some(&k)).unwrap().clone();
        let b = renderer.render(5.0, Some(&k)).unwrap().clone();
        insta::assert_snapshot!("volute_seed42", grid_to_plain(&a).join("\n"));
        insta::assert_snapshot!("volute_time5", grid_to_plain(&b).join("\n"));
        assert_ne!(grid_to_plain(&a), grid_to_plain(&b));
        assert_eq!(&a, renderer.render(0.0, Some(&k)).unwrap());
        let mut stopped = k.clone();
        stopped[5] = 0.0;
        assert_eq!(&a, renderer.render(19.0, Some(&stopped)).unwrap());
        for i in 0..k.len() {
            let mut changed = k.clone();
            changed[i] = PARAMS[i].max;
            assert_ne!(
                &b,
                renderer.render(5.0, Some(&changed)).unwrap(),
                "{}",
                PARAMS[i].key
            );
        }
        let mut other = IterateFrameRenderer::new("volute", 43, "deep", 100, 32).unwrap();
        assert_ne!(&a, other.render(0.0, Some(&k)).unwrap());
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn volute_boundaries_and_inputs() {
        use rand::SeedableRng;
        for (w, h) in [(0, 0), (0, 3), (3, 0), (1, 1), (2, 9), (9, 2), (80, 24)] {
            let mut grid = vec![vec![Cell::blank(); w]; h];
            let palette = crate::color::make_palette(42);
            let mut rng = rand::rngs::StdRng::seed_from_u64(42);
            for k in [
                PARAMS.iter().map(|p| p.min).collect::<Vec<_>>(),
                PARAMS.iter().map(|p| p.max).collect(),
                vec![f32::NAN; 6],
                vec![f32::INFINITY; 6],
            ] {
                for time in [
                    0.0,
                    7.0,
                    -100.0,
                    f32::MIN,
                    f32::MAX,
                    f32::NAN,
                    f32::INFINITY,
                ] {
                    MODE.render(&mut ModeFrame {
                        grid: &mut grid,
                        width: w,
                        height: h,
                        seed: u64::MAX,
                        palette: &palette,
                        rng: &mut rng,
                        time,
                        args: &[],
                        param_values: Some(&k),
                    });
                    assert_eq!(grid.len(), h);
                    assert!(grid.iter().all(|r| r.len() == w));
                    assert!(
                        grid.iter()
                            .flatten()
                            .all(|c| crate::types::char_width(c.ch) == 1)
                    );
                }
            }
        }
        let k: Vec<_> = PARAMS.iter().map(|p| p.default).collect();
        let mut renderer = IterateFrameRenderer::new("volute", 42, "deep", 80, 24).unwrap();
        let a = renderer.render(0.0, Some(&k)).unwrap().clone();
        assert_eq!(&a, renderer.render(f32::NAN, Some(&[f32::NAN; 6])).unwrap());
        assert_eq!(
            &a,
            renderer
                .render(f32::INFINITY, Some(&[f32::INFINITY; 6]))
                .unwrap()
        );
    }
}
