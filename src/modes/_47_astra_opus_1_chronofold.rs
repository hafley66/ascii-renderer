//! ASTRA / OPUS REPLAY 01: CHRONOFOLD.
//! Lease as `_N_astra_opus_1_chronofold.rs`; N is assigned by the integrator.
//! Inline snapshots travel with this file when it is renamed.

use crate::color::lerp_color;
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use std::f32::consts::TAU;

pub(super) struct ChronofoldMode;
pub(super) static MODE: ChronofoldMode = ChronofoldMode;

const PARAMS: &[Param] = &[
    param!("SPEED", "sculpture clock", 0.0, 2.0, 0.55, 0.05),
    param!("LOBES", "knot winding", 3.0, 7.0, 3.0, 1.0),
    param!("WIDTH", "faceted cable radius", 0.06, 0.28, 0.17, 0.01),
    param!("TWIST", "helical fluting", -4.0, 4.0, 2.0, 1.0),
    param!("ORBITS", "tracer hoops", 0.0, 3.0, 2.0, 1.0),
    param!("ZOOM", "sculpture magnification", 0.5, 1.5, 1.0, 0.05),
    param!("LATTICE", "perspective stage", 0.0, 1.0, 0.65, 0.05),
];

impl Mode for ChronofoldMode {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn name(&self) -> &'static str {
        "astra-opus-1-chronofold"
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn help(&self) -> &'static str {
        "ASTRA / OPUS REPLAY 01: CHRONOFOLD. Faceted torus knot, helical flutes, orbiting tracers and a moving perspective stage [speed] [lobes] [width] [twist] [orbits] [zoom] [lattice]"
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
        let knobs = std::array::from_fn(|i| {
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
        draw_chronofold(frame, &knobs);
    }
}

type V3 = [f32; 3];

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn unit(v: V3) -> V3 {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    v.map(|x| x / length)
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn hash(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

// A frame owns the raster and reciprocal-depth buffer; all geometry is rebuilt.
struct Raster<'a> {
    grid: &'a mut Grid,
    depth: Vec<f32>,
    width: usize,
    height: usize,
    scale: f32,
    yaw: (f32, f32),
    tilt: (f32, f32),
}

impl Raster<'_> {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn rotate(&self, p: V3) -> V3 {
        let (s, c) = self.yaw;
        let x = c * p[0] + s * p[2];
        let z = c * p[2] - s * p[0];
        let (s, c) = self.tilt;
        [x, c * p[1] - s * z, s * p[1] + c * z]
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn project(&self, p: V3) -> V3 {
        // Every surface lies inside radius 2.2; the eye is five units away.
        let inv = 1.0 / (5.0 - p[2]);
        [
            self.width as f32 * 0.5 + p[0] * self.scale * 10.0 * inv,
            self.height as f32 * 0.46 - p[1] * self.scale * 5.0 * inv,
            inv,
        ]
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn put(&mut self, x: i32, y: i32, z: f32, cell: Cell) {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return;
        }
        let index = y as usize * self.width + x as usize;
        if z >= self.depth[index] {
            self.depth[index] = z;
            self.grid[y as usize][x as usize] = cell;
        }
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn line(&mut self, a: V3, b: V3, mut cell: Cell) {
        let d = sub(b, a);
        if cell.ch == '\0' {
            cell.ch = if d[0].abs() > d[1].abs() * 2.2 {
                '-'
            } else if d[1].abs() > d[0].abs() * 1.2 {
                '|'
            } else if d[0] * d[1] > 0.0 {
                '\\'
            } else {
                '/'
            };
        }
        let steps = (d[0].abs().max(d[1].abs()).ceil() as usize)
            .clamp(1, 4 * (self.width + self.height).max(1));
        for j in 0..=steps {
            let f = j as f32 / steps as f32;
            self.put(
                (a[0] + f * d[0]).floor() as i32,
                (a[1] + f * d[1]).floor() as i32,
                a[2] + f * d[2] + 0.00015,
                cell,
            );
        }
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn triangle(&mut self, a: V3, b: V3, c: V3, cell: Cell) {
        let edge =
            |p: V3, q: V3, x: f32, y: f32| (q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0]);
        let area = edge(a, b, c[0], c[1]);
        if area.abs() < 1e-5 {
            return;
        }
        let x0 = a[0].min(b[0]).min(c[0]).floor().max(0.0) as usize;
        let y0 = a[1].min(b[1]).min(c[1]).floor().max(0.0) as usize;
        let x1 = (a[0].max(b[0]).max(c[0]).ceil().max(0.0) as usize).min(self.width);
        let y1 = (a[1].max(b[1]).max(c[1]).ceil().max(0.0) as usize).min(self.height);
        for y in y0..y1 {
            for x in x0..x1 {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let u = edge(b, c, px, py) / area;
                let v = edge(c, a, px, py) / area;
                let w = 1.0 - u - v;
                if u >= -0.0001 && v >= -0.0001 && w >= -0.0001 {
                    self.put(x as i32, y as i32, u * a[2] + v * b[2] + w * c[2], cell);
                }
            }
        }
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn draw_chronofold(frame: &mut ModeFrame<'_>, k: &[f32; 7]) {
    // Initialize a palette ramp, deterministic seed phases and a fresh depth buffer.
    // Evaluate the knot and its transported octagonal section at this time.
    // Rasterize lit facets and ribs; depth-test analytic orbital trails.
    // Overlay the numbered title and stage annotations inside the grid bounds.
    let height = frame.height.min(frame.grid.len());
    let width = frame.width.min(
        frame
            .grid
            .iter()
            .take(height)
            .map(Vec::len)
            .min()
            .unwrap_or(0),
    );
    if width == 0 || height == 0 {
        return;
    }
    let time = if frame.time.is_finite() {
        frame.time as f64 * k[0] as f64
    } else {
        0.0
    };
    // Reduce each phase in f64, avoiding a visible whole-scene clock reset.
    let phase = |rate: f64| (time * rate).rem_euclid(std::f64::consts::TAU) as f32;
    let identity = hash(frame.seed);
    let seed_phase = (identity & 65535) as f32 / 65536.0 * TAU;
    let p = frame.palette;
    let ink = p[0];
    let dim = lerp_color(ink, p[3], 0.23);
    let ramp: [[Cell; 10]; 4] = std::array::from_fn(|band| {
        std::array::from_fn(|light| {
            let intensity = light as f32 / 9.0;
            let color = lerp_color(p[[1, 3, 2, 3][band]], p[4], intensity.powi(3) * 0.85);
            Cell::with_bg(
                b".:-=+*#%@@"[light] as char,
                lerp_color(ink, color, 0.32 + 0.68 * intensity),
                lerp_color(ink, color, 0.06 + 0.20 * intensity),
            )
        })
    });
    for (y, row) in frame.grid.iter_mut().take(height).enumerate() {
        for (x, cell) in row.iter_mut().take(width).enumerate() {
            let mut ch = ' ';
            let ground = y as f32 / height as f32 - 0.72;
            if ground > 0.0 && k[6] > 0.0 {
                let perspective = 0.16 / (ground + 0.045);
                let across = (x as f32 - width as f32 * 0.5) / height as f32 * perspective;
                let course = perspective * 2.4 + phase(0.35) / TAU;
                if across.rem_euclid(0.55) < 0.025 {
                    ch = if x < width / 2 { '/' } else { '\\' };
                }
                if course.rem_euclid(1.0) < 0.07 {
                    ch = if ch == ' ' { '_' } else { '+' };
                }
            }
            *cell = Cell::with_bg(ch, lerp_color(ink, dim, k[6]), ink);
        }
    }
    // Stable star identities; the number of samples is bounded independently of time.
    for i in 0..(width.saturating_mul(height) / 110).min(400) {
        let v = hash(identity.wrapping_add(i as u64));
        let x = v as usize % width;
        let y = (v >> 32) as usize % height;
        if y < height * 3 / 4 {
            frame.grid[y][x] = Cell::with_bg(if i % 7 == 0 { '+' } else { '.' }, dim, ink);
        }
    }
    let mut raster = Raster {
        grid: frame.grid,
        depth: vec![0.0; width * height],
        width,
        height,
        scale: (height as f32 * 0.255).min(width as f32 * 0.13) * k[5],
        yaw: (phase(0.23) + seed_phase * 0.2).sin_cos(),
        tilt: (0.44 + 0.30 * (phase(0.17) + seed_phase).sin()).sin_cos(),
    };
    let lobes = k[1].round();
    let breathe = 0.34 + 0.055 * phase(0.63).sin();
    let knot = |u: f32| -> V3 {
        let q = lobes * u + seed_phase;
        let radius = 1.02 + breathe * q.cos();
        [
            radius * (2.0 * u).cos(),
            radius * (2.0 * u).sin(),
            0.48 * q.sin(),
        ]
    };
    let segments = (width.max(height).saturating_mul(3)).clamp(96, 640);
    let mut world = Vec::<[V3; 8]>::with_capacity(segments + 1);
    let mut projected = Vec::<[V3; 8]>::with_capacity(segments + 1);
    // Integer winding closes the fluting seam even while the twist slider moves.
    let twist = k[3].round();
    for i in 0..=segments {
        let u = (i % segments) as f32 / segments as f32 * TAU;
        let center = knot(u);
        let tangent = unit(sub(knot(u + 0.001), knot(u - 0.001)));
        let radial = [(2.0 * u).cos(), (2.0 * u).sin(), 0.0];
        let normal = unit(cross(tangent, radial));
        let binormal = unit(cross(normal, tangent));
        let vertices = std::array::from_fn(|j| {
            let v = j as f32 / 8.0 * TAU + twist * u + phase(0.4);
            let radius = k[2] * (1.0 + 0.12 * (lobes * u - phase(0.7)).sin());
            raster.rotate(std::array::from_fn(|axis| {
                center[axis] + radius * (normal[axis] * v.cos() + binormal[axis] * v.sin())
            }))
        });
        projected.push(vertices.map(|v| raster.project(v)));
        world.push(vertices);
    }
    for i in 0..segments {
        for j in 0..8 {
            let next = (j + 1) % 8;
            let a = world[i][j];
            let b = world[i + 1][j];
            let c = world[i][next];
            let n = unit(cross(sub(b, a), sub(c, a)));
            let diffuse = (n[0] * -0.38 + n[1] * 0.64 + n[2] * 0.67).abs();
            let light = ((0.15 + 0.70 * diffuse + 0.15 * n[2].abs().powi(12)) * 9.0)
                .round()
                .clamp(0.0, 9.0) as usize;
            let band = (i * 12 / segments + (identity as usize & 3)) % 4;
            let cell = ramp[band][light];
            let a = projected[i][j];
            let b = projected[i + 1][j];
            let c = projected[i][next];
            let d = projected[i + 1][next];
            raster.triangle(a, b, c, cell);
            raster.triangle(b, d, c, cell);
            if i % (segments / 24).max(1) == 0 {
                raster.line(
                    a,
                    c,
                    Cell::with_bg('\0', lerp_color(cell.fg, p[4], 0.22), cell.bg),
                );
            }
        }
    }
    for orbit in 0..k[4].round() as usize {
        let hoop = |u: f32| {
            let angle = orbit as f32 * 1.05 + 0.32;
            let radius = 1.63 + orbit as f32 * 0.13;
            raster.project(raster.rotate([
                radius * u.cos(),
                radius * u.sin() * angle.cos(),
                radius * u.sin() * angle.sin(),
            ]))
        };
        let points: Vec<_> = (0..=segments)
            .map(|i| hoop(i as f32 / segments as f32 * TAU))
            .collect();
        let head = phase(0.55 + orbit as f64 * 0.19) + seed_phase + orbit as f32 * 2.0;
        for i in 0..segments {
            let u = i as f32 / segments as f32 * TAU;
            let age = (head - u).rem_euclid(TAU);
            let strength = (-age * 2.2).exp();
            if i % 3 == 0 || strength > 0.2 {
                let color = lerp_color(ink, p[3 - orbit % 3], 0.23 + 0.77 * strength);
                raster.line(
                    points[i],
                    points[i + 1],
                    Cell::with_bg(if strength > 0.7 { '*' } else { '\0' }, color, ink),
                );
            }
        }
    }
    // Labels are ASCII and clipped; small canvases retain the sculpture alone.
    if width >= 48 && height >= 16 {
        for (y, label) in [
            (1, "ASTRA / OPUS REPLAY 01"),
            (height - 2, "C H R O N O F O L D   /   TORUS STUDY"),
        ] {
            for (i, ch) in label.chars().enumerate().take(width.saturating_sub(4)) {
                raster.grid[y][i + 2] = Cell::with_bg(ch, if y == 1 { dim } else { p[3] }, ink);
            }
        }
        for (x, y) in [
            (0, 0),
            (width - 1, 0),
            (0, height - 1),
            (width - 1, height - 1),
        ] {
            raster.grid[y][x] = Cell::with_bg('+', dim, ink);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn render(
        width: usize,
        height: usize,
        seed: u64,
        time: f32,
        values: &[f32],
        args: &[String],
    ) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = crate::color::named_theme("neon").unwrap();
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width,
            height,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args,
            param_values: Some(values),
        });
        grid
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn plain(grid: &Grid) -> String {
        grid.iter()
            .map(|row| {
                row.iter()
                    .map(|c| c.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn deterministic_motion_and_controls() {
        let defaults: Vec<_> = PARAMS.iter().map(|p| p.default).collect();
        let a = render(80, 28, 42, 0.0, &defaults, &[]);
        let b = render(80, 28, 42, 4.0, &defaults, &[]);
        assert_eq!(a, render(80, 28, 42, 0.0, &defaults, &[]));
        assert_ne!(plain(&a), plain(&b));
        assert_ne!(a, render(80, 28, 43, 0.0, &defaults, &[]));
        let mut stopped = defaults.clone();
        stopped[0] = 0.0;
        assert_eq!(
            render(80, 28, 42, 0.0, &stopped, &[]),
            render(80, 28, 42, 50.0, &stopped, &[])
        );
        let args: Vec<_> = ["renderer", "42", MODE.name(), "neon", "0"]
            .map(str::to_owned)
            .to_vec();
        assert_eq!(
            render(80, 28, 42, 50.0, &defaults, &args),
            render(80, 28, 42, 0.0, &stopped, &[])
        );
        for i in 0..PARAMS.len() {
            let mut changed = defaults.clone();
            changed[i] = PARAMS[i].max;
            assert_ne!(
                b,
                render(80, 28, 42, 4.0, &changed, &[]),
                "{} must affect the frame",
                PARAMS[i].key
            );
        }
        assert!(a.iter().flatten().all(|cell| cell.ch.is_ascii()));
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn dimensions_extrema_and_nonfinite_inputs() {
        let defaults: Vec<_> = PARAMS.iter().map(|p| p.default).collect();
        let bad = vec![f32::NAN; PARAMS.len()];
        assert_eq!(
            render(48, 18, 42, f32::NAN, &bad, &[]),
            render(48, 18, 42, 0.0, &defaults, &[])
        );
        for values in [
            PARAMS.iter().map(|p| p.min).collect::<Vec<_>>(),
            PARAMS.iter().map(|p| p.max).collect(),
        ] {
            for (w, h) in [
                (0, 0),
                (0, 5),
                (5, 0),
                (1, 1),
                (1, 24),
                (80, 1),
                (7, 3),
                (80, 28),
            ] {
                for time in [-10.0, 1e20, f32::INFINITY] {
                    let grid = render(w, h, u64::MAX, time, &values, &[]);
                    assert_eq!((grid.len(), grid.first().map_or(w, Vec::len)), (h, w));
                }
            }
        }
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn chronofold_seed_42_t0() {
        let defaults: Vec<_> = PARAMS.iter().map(|p| p.default).collect();
        let output = plain(&render(80, 28, 42, 0.0, &defaults, &[]));
        insta::assert_snapshot!(output, @r###"
        +                                                                              +
          ASTRA / OPUS REPLAY 01
                            .           %##***| ---------------
         .             .             @####-------             --\
                                  /@@#-\-:-*++/==---            \\
                                  @@**---   \ ++=---|    /%%\*+   | .
                                 @@-----       |:--==@@%####:|:-- |
                                @//**-           @@\%%%%--::--*++-|.
                               ///**=      |@@%%@*###---     \\--||
                             //%%+++=-@@%%|#%%=++*==\*##=--/*-==+|
                            //#%%++=\#%%+**|---*    =##%*+ |===*/ ------
                           // #------#--=++ -         +%--=+---/       --\
                          /---|%%%===--               --*#*- //           |
                       //**   -*##==               %@@@\=##++/            |
                      // *   ##*##==-           %%\%++*=\+##=-           //
                     || ||   **=+**==-      /%+-=+-|+  -/\*---        --/
                     |\ |    +**==++--|    /--|=|*/  --- **++     - --
                       \--    *+=====-/-=+*-#=+|  ----   ##==-- - -             .
                         -- ----+++---------/=/----- --\--=--                       .
                           \--  -+-+--=--=-/----- - #@@%%--/
                              -----+++*|####%#%%%|@@%%%-:
            /                         -/++++****##|--                        \
                           /                    \ |        \                    \
                    /             /             \
             /                 /                \                 \
                                                \                    \
          C H R O N O F O L D   /   TORUS STUDY \\                       \
        +            /                          \\                          \\         +
        "###);
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn chronofold_seed_42_t4() {
        let defaults: Vec<_> = PARAMS.iter().map(|p| p.default).collect();
        let output = plain(&render(80, 28, 42, 4.0, &defaults, &[]));
        insta::assert_snapshot!(output, @r###"
        +                                                                              +
          ASTRA / OPUS REPLAY 01                     ------
                            .          %%%++-      --     \|
         .             .             /-%%--+-/   //        |
                                     %%%%%+-/+ //      --%%-
                                     %%%%%=++/// . .  %%%+++|       .
                                     %%%%%==/--:    --%%++=-/
                                     %%%%%=/--==- @%%%-++++-       .
                                     %%%#//- :\+@%%%==+--=
                                     +%%/##--@\##%:-*#===|****--------
                                      %/%#-#--+\=-###+|=-             ----
                                  --@%//--**-*    -+*/*+                 \|
                             -----@@%/=###***- |-@%/--##+                 /
                          ---    %%%/--+###*++:%%@\=//++**|             //
                       //-     ----||*  +***++:%=++|/  ----           ---
                      |/       %%#||-    =+++==-+ //   *##=        ---
                      |        ###|:      =++=---/     ##%-  --- -
                      \--      ##|*:       ----:/-\   ----- -                   .
                        ---- ---#|*--    - --------\--@@%--                         .
                               ||-+---- -   //===+| @@##-
                                |+--+-=   ///    #%\%*#-
            /                   | =======++-**###%%*\=                       \
                           /    |\ ---=====|====+++=|      \                    \
                    /             -**-::---======
             /                 /                \                 \
                                                \                    \
          C H R O N O F O L D   /   TORUS STUDY \\                       \
        +            /                          \\                          \\         +
        "###);
    }
}
