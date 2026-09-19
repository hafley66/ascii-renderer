//! Sacred geometry equations from gothic lib/legacy/0_circles.js.
use super::_60_slice::{self as slice, Point, circle, polar};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use std::f64::consts::TAU;
use std::sync::OnceLock;
pub(super) struct Circles;
pub(super) static MODE: Circles = Circles;
const OWN: &[Param] = &[
    param!(
        "CIRCLE_SHAPE",
        "geometry",
        0.,
        3.,
        0.,
        1.,
        &["flower of life", "metatron", "vesica", "hypotrochoid"]
    ),
    param!("CIRCLE_RINGS", "flower lattice rings", 1., 3., 2., 1.),
    param!("CIRCLE_N", "vesica axes", 1., 12., 6., 1.),
    param!("CIRCLE_P", "rolling numerator", 1., 11., 5., 1.),
    param!("CIRCLE_Q", "rolling denominator", 1., 4., 3., 1.),
    param!("CIRCLE_D", "pen distance", 0.1, 1.5, 0.8, 0.05),
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
impl Mode for Circles {
    fn name(&self) -> &'static str {
        "gothic-circles"
    }
    fn help(&self) -> &'static str {
        "Source flower lattice, 13-node Metatron graph, opposed vesica circles or rolling hypotrochoid; shared Slice animation knobs."
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        params()
    }
    fn render(&self, f: &mut ModeFrame<'_>) {
        let v = slice::values(f, params());
        let mut animation = slice::animation_values(&v[OWN.len()..]);
        slice::animate_inputs(&mut animation, f.time);
        let sk = &animation;
        let paths = geometry(&v[..OWN.len()]);
        let st = slice::compile(paths, f.seed, sk);
        slice::paint(f, &st, sk)
    }
}
fn geometry(k: &[f64]) -> Vec<Vec<Point>> {
    let r = 117.;
    let mut out = vec![];
    match k[0].round() as usize {
        0 => {
            let rings = k[1].round() as i32;
            let rr = r / (rings as f64 + 0.5);
            for q in -rings..=rings {
                for s in -rings..=rings {
                    if (q + s).abs() > rings {
                        continue;
                    }
                    let c = Point(
                        rr * (q as f64 + s as f64 / 2.),
                        rr * s as f64 * 3f64.sqrt() / 2.,
                    );
                    clip_circle(&mut out, circle(c, rr), r);
                }
            }
            out.push(circle(Point(0., 0.), r));
        }
        1 => {
            let rr = r / 3.;
            let mut pts = vec![Point(0., 0.)];
            for ring in 1..=2 {
                for i in 0..6 {
                    pts.push(polar(rr * ring as f64, i as f64 * TAU / 6.))
                }
            }
            for &p in &pts {
                out.push(circle(p, rr * 0.5))
            }
            for i in 0..pts.len() {
                for j in i + 1..pts.len() {
                    out.push(vec![pts[i], pts[j]])
                }
            }
        }
        2 => {
            let n = k[2].round() as usize;
            for i in 0..n {
                let c = polar(r * 0.5, i as f64 * TAU / 2. / n as f64);
                out.push(circle(c, r * 0.5));
                out.push(circle(Point(-c.0, -c.1), r * 0.5));
            }
            out.push(circle(Point(0., 0.), r));
        }
        _ => {
            let p = k[3].round();
            let q = k[4].round();
            let d = k[5];
            let rr = r * p / (p + q);
            let kk = (r - rr) / rr;
            let scale = r / (r - rr + d * rr);
            out.push(
                (0..=720 * q as usize)
                    .map(|i| {
                        let t = i as f64 / 720. * TAU;
                        Point(
                            scale * ((r - rr) * t.cos() + d * rr * (kk * t).cos()),
                            scale * ((r - rr) * t.sin() - d * rr * (kk * t).sin()),
                        )
                    })
                    .collect(),
            );
            out.push(circle(Point(0., 0.), r));
        }
    }
    out
}
fn clip_circle(out: &mut Vec<Vec<Point>>, path: Vec<Point>, radius: f64) {
    let mut run = vec![];
    for pair in path.windows(2) {
        let a = pair[0];
        let b = pair[1];
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let aa = dx * dx + dy * dy;
        let bb = 2. * (a.0 * dx + a.1 * dy);
        let cc = a.0 * a.0 + a.1 * a.1 - radius * radius;
        let disc = bb * bb - 4. * aa * cc;
        if disc < 0. || aa == 0. {
            if run.len() > 1 {
                out.push(std::mem::take(&mut run))
            }
            continue;
        }
        let lo = ((-bb - disc.sqrt()) / (2. * aa)).max(0.);
        let hi = ((-bb + disc.sqrt()) / (2. * aa)).min(1.);
        if hi > lo {
            let p = Point(a.0 + dx * lo, a.1 + dy * lo);
            let q = Point(a.0 + dx * hi, a.1 + dy * hi);
            if run.is_empty() {
                run.push(p)
            }
            run.push(q);
            if hi < 1. {
                out.push(std::mem::take(&mut run))
            }
        } else if run.len() > 1 {
            out.push(std::mem::take(&mut run))
        }
    }
    if run.len() > 1 {
        out.push(run)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Cell;
    use rand::{SeedableRng, rngs::StdRng};
    fn frame(time: f32, shape: f32, max: bool, w: usize, h: usize) -> String {
        let mut v: Vec<_> = params()
            .iter()
            .map(|p| if max { p.max } else { p.default })
            .collect();
        v[0] = shape;
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(7);
        let mut rng = StdRng::seed_from_u64(7);
        MODE.render(&mut ModeFrame {
            grid: &mut g,
            width: w,
            height: h,
            seed: 7,
            palette: &p,
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(&v),
        });
        crate::render::grid_to_plain(&g).join("\n")
    }
    #[test]
    fn gothic_circles_views() {
        insta::assert_snapshot!("circles_flower_seed7_t04", frame(0.4, 0., false, 100, 36));
        insta::assert_snapshot!("circles_flower_seed7_t11", frame(1.1, 0., false, 100, 36));
        insta::assert_snapshot!("circles_metatron_seed7_t11", frame(1.1, 1., false, 100, 36));
        insta::assert_snapshot!("circles_vesica_seed7_t11", frame(1.1, 2., false, 100, 36));
        insta::assert_snapshot!("circles_spiro_seed7_t11", frame(1.1, 3., false, 100, 36));
    }
    #[test]
    fn gothic_circles_replay_and_bounds() {
        assert_eq!(frame(0.4, 0., false, 80, 24), frame(0.4, 0., false, 80, 24));
        assert_ne!(frame(0.4, 0., false, 80, 24), frame(1.1, 0., false, 80, 24));
        for shape in 0..4 {
            for (w, h) in [(0, 0), (1, 1), (3, 2), (80, 24)] {
                frame(0.7, shape as f32, true, w, h);
            }
        }
    }
}
