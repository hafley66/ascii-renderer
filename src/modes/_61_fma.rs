//! Native paths from gothic algos/2_fma.ts, animated by the shared source Slice port.
use super::_60_slice::{self as slice, Point, Random, circle, closed, foil, polar, star};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use std::f64::consts::{PI, TAU};
use std::sync::OnceLock;
pub(super) struct Fma;
pub(super) static MODE: Fma = Fma;
const OWN: &[Param] = &[
    param!("FMA_N", "symmetry 0 seeded", 0., 9., 0., 1.),
    param!("FMA_STEP", "star step 0 seeded", 0., 4., 0., 1.),
    param!("FMA_DEPTH", "recursive levels", 1., 3., 2., 1.),
    param!("FMA_SAT", "satellite fraction", 0.5, 1., 0.9, 0.05),
    param!("FMA_MIN_PX", "detail threshold", 1., 6., 2., 0.5),
    param!("FMA_SCRIPT", "ASCII inscription ring", 0., 1., 0., 1.),
    param!("FMA_PUPIL", "eye pupil", 0., 1., 1., 1.),
];
fn params() -> &'static [Param] {
    static P: OnceLock<Vec<Param>> = OnceLock::new();
    P.get_or_init(|| {
        OWN.iter()
            .chain(slice::animation_params())
            .cloned()
            .collect()
    })
}
impl Mode for Fma {
    fn name(&self) -> &'static str {
        "fma"
    }
    fn help(&self) -> &'static str {
        "Fullmetal source transmutation: recursive tangent satellites, star chords, dual polygon and eye; all Slice controls follow geometry controls."
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        params()
    }
    fn render(&self, f: &mut ModeFrame<'_>) {
        let values = slice::values(f, params());
        let mut animation = slice::animation_values(&values[OWN.len()..]);
        slice::animate_inputs(&mut animation, f.time);
        let k = &values[..OWN.len()];
        let sk = &animation;
        let paths = geometry(f.seed, k);
        let strokes = slice::compile(paths, f.seed, sk);
        slice::paint(f, &strokes, sk);
    }
}
fn nearest(n: usize, want: usize) -> usize {
    (2..=(n - 1) / 2)
        .filter(|&i| gcd(n, i) == 1)
        .min_by_key(|&i| i.abs_diff(want))
        .unwrap_or(1)
}
fn gcd(mut a: usize, mut b: usize) -> usize {
    while b > 0 {
        (a, b) = (b, a % b)
    }
    a
}
fn child_n(n: usize) -> usize {
    if n <= 6 {
        n
    } else {
        (3..=n / 2).find(|d| n % d == 0).unwrap_or(n)
    }
}
fn geometry(seed: u64, k: &[f64]) -> Vec<Vec<Point>> {
    let mut rng = Random(seed as u32);
    let n = if k[0] >= 3. {
        k[0].round() as usize
    } else {
        3 + rng.pick(6)
    };
    let ks: Vec<_> = (2..=(n - 1) / 2).filter(|&i| gcd(n, i) == 1).collect();
    let step = if k[1] >= 1. {
        nearest(n, k[1].round() as usize)
    } else if ks.is_empty() {
        1
    } else {
        ks[rng.pick(ks.len())]
    };
    let mut out = vec![];
    compose(
        &mut out,
        118.,
        seed as u32,
        n,
        step,
        k[2].round() as usize,
        Point(0., 0.),
        0.,
        k,
        true,
    );
    out
}
#[allow(clippy::too_many_arguments)]
fn compose(
    out: &mut Vec<Vec<Point>>,
    r: f64,
    seed: u32,
    n: usize,
    step: usize,
    level: usize,
    center: Point,
    rot: f64,
    k: &[f64],
    root: bool,
) {
    let transform = |p: Point| {
        Point(
            center.0 + p.0 * rot.cos() - p.1 * rot.sin(),
            center.1 + p.0 * rot.sin() + p.1 * rot.cos(),
        )
    };
    let mut local = vec![circle(Point(0., 0.), r)];
    let min = k[4];
    let r1 = r * 0.955;
    let r0 = r * 0.86;
    let h = r1 - r0;
    if r - r1 >= min {
        local.push(circle(Point(0., 0.), r1))
    }
    if h >= min * 2. {
        let count = n * 2;
        for i in 0..count {
            let a = -PI / 2. + i as f64 * TAU / count as f64;
            local.push(vec![
                polar(if i % 2 == 0 { r0 } else { r0 + h * 0.5 }, a),
                polar(r1, a),
            ]);
            if root && k[5] >= 0.5 {
                let c = polar((r0 + r1) / 2., a);
                let v = polar(h * 0.25, a + PI / 2.);
                local.push(vec![c.add(Point(-v.0, -v.1)), c.add(v)]);
            }
        }
    }
    let rp = r * 0.84;
    if 2. * rp * (PI / n as f64).sin() >= min * 1.5 {
        star(&mut local, rp, n, 1, -PI / 2.);
        if step > 1 {
            star(&mut local, rp, n, step, -PI / 2.)
        }
    }
    let rs = rp * (PI / n as f64).sin() / (1. + (PI / n as f64).sin()) * k[3];
    let rc = rp - rs;

    if rs >= min {
        for i in 0..n {
            let a = -PI / 2. + i as f64 * TAU / n as f64;
            let c = polar(rc, a);
            local.push(circle(c, rs));
            let inner = rs * 0.86;
            if level > 1 && inner * 2. >= min * 10. {
                for path in local.drain(..) {
                    out.push(path.into_iter().map(transform).collect())
                }
                let child_seed = seed
                    .wrapping_mul(0x9e3779b1)
                    .wrapping_add((i as u32 + 1).wrapping_mul(0x85ebca6b));
                let nn = child_n(n);
                compose(
                    out,
                    inner,
                    child_seed,
                    nn,
                    nearest(nn, step),
                    level - 1,
                    transform(c),
                    rot + a + PI / 2.,
                    k,
                    false,
                );
            } else if TAU * rs * 0.6 / n as f64 >= min * 1.5 {
                local.push(foil(c, rs * 0.6, n))
            }
        }
    }
    if step > 1 && 2. * rc * (PI * step as f64 / n as f64).sin() >= min * 2. {
        for i in 0..n {
            local.push(vec![
                polar(rc, -PI / 2. + i as f64 * TAU / n as f64),
                polar(rc, -PI / 2. + (i + step) as f64 * TAU / n as f64),
            ])
        }
    }
    let ri = rc - rs;
    if ri > 0. && 2. * ri * (PI / n as f64).sin() >= min * 1.5 {
        local.push(closed(
            (0..n)
                .map(|i| polar(ri, -PI / 2. + PI / n as f64 + i as f64 * TAU / n as f64))
                .collect(),
        ))
    }
    let rk = ri * (PI / n as f64).cos() * 0.92;
    if rk > 0. {
        if level > 1 && rk * 2. >= min * 10. {
            for path in local.drain(..) {
                out.push(path.into_iter().map(transform).collect())
            }
            let child_seed = seed
                .wrapping_mul(0x9e3779b1)
                .wrapping_add((n as u32 + 1).wrapping_mul(0x85ebca6b));
            let nn = child_n(n);
            compose(
                out,
                rk,
                child_seed,
                nn,
                nearest(nn, step),
                level - 1,
                transform(Point(0., 0.)),
                rot + PI / n as f64,
                k,
                false,
            );
        } else if rk >= min * 2. {
            local.push(circle(Point(0., 0.), rk));
            let ir = rk * 0.62;
            if TAU * ir / n.min(6) as f64 >= min * 1.5 {
                local.push(foil(Point(0., 0.), ir, n.min(6)))
            } else if ir >= min {
                local.push(circle(Point(0., 0.), ir))
            }
            if k[6] >= 0.5 && rk * 0.2 >= 0.6 {
                local.push(circle(Point(0., 0.), rk * 0.2))
            }
        }
    }
    for path in local {
        out.push(path.into_iter().map(transform).collect())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Cell;
    use rand::{SeedableRng, rngs::StdRng};
    fn frame(t: f32, max: bool, w: usize, h: usize) -> String {
        let values: Vec<_> = params()
            .iter()
            .map(|p| if max { p.max } else { p.default })
            .collect();
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(7);
        let mut rng = StdRng::seed_from_u64(7);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed: 7,
            palette: &palette,
            rng: &mut rng,
            time: t,
            args: &[],
            param_values: Some(&values),
        });
        crate::render::grid_to_plain(&grid).join("\n")
    }
    #[test]
    fn fma_source_views() {
        insta::assert_snapshot!("fma_seed7_t04", frame(0.4, false, 100, 36));
        insta::assert_snapshot!("fma_seed7_t11", frame(1.1, false, 100, 36));
    }
    #[test]
    fn fma_determinism_and_extrema() {
        assert_eq!(frame(0.4, false, 80, 24), frame(0.4, false, 80, 24));
        assert_ne!(frame(0.4, false, 80, 24), frame(0.8, false, 80, 24));
        for (w, h) in [(0, 0), (1, 1), (3, 2), (80, 24)] {
            frame(0.6, true, w, h);
        }
    }
}
