//! Gothic icon seals: the source composeSeal grammar, name hashes and radius LOD.
use super::_60_slice::{self as slice, Point};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use std::sync::OnceLock;
pub(super) struct Icons;
pub(super) static MODE: Icons = Icons;
const OWN: &[Param] = &[
    param!("ICON_COLS", "columns", 1., 6., 3., 1.),
    param!("ICON_ROWS", "rows", 1., 6., 3., 1.),
    param!("ICON_SIZE", "source SVG size px", 16., 96., 48., 2.),
    param!("ICON_MIN_PX", "detail threshold", 1., 6., 2., 0.5),
    param!(
        "ICON_NAMES",
        "name seed family",
        0.,
        2.,
        0.,
        1.,
        &["tools", "alchemy", "architecture"]
    ),
    param!("ICON_PUPIL", "eye pupil", 0., 1., 1., 1.),
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
impl Mode for Icons {
    fn name(&self) -> &'static str {
        "gothic-icons"
    }
    fn help(&self) -> &'static str {
        "Source name-hashed icon seals at selected SVG size/detail; arranged as ASCII art and animated with Slice knobs."
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
        let k = &v[..OWN.len()];
        let sk = &animation;
        let paths = geometry(f.seed, k, sk);
        let st = slice::compile(paths, f.seed, sk);
        slice::paint(f, &st, sk)
    }
}
fn geometry(seed: u64, k: &[f64], sk: &[f64]) -> Vec<Vec<Point>> {
    let names = match k[4].round() as usize {
        1 => "mercury salt sulphur silver iron gold",
        2 => "nave apse vault rose lancet spire",
        _ => "github rxjs hn docs mail calendar grapht boop gothic tanstack vite playwright",
    };
    let words: Vec<_> = names.split_whitespace().collect();
    let cols = k[0].round() as usize;
    let rows = k[1].round() as usize;
    let cell = 228. / cols.max(rows) as f64;
    let radius = k[2] / 2. - 1.;
    let factor = (cell / 2. - 2.) / radius;
    let mut out = vec![];
    for y in 0..rows {
        for x in 0..cols {
            let i = y * cols + x;
            let salt = slice::fnv(words[i % words.len()]) as u64;
            let center = Point(
                (x as f64 - (cols - 1) as f64 / 2.) * cell,
                (y as f64 - (rows - 1) as f64 / 2.) * cell,
            );
            for path in slice::seal_paths(
                (seed as u32).wrapping_mul(2654435761) as u64 ^ salt,
                sk,
                radius,
                k[3],
                false,
                k[5] >= 0.5,
            ) {
                out.push(
                    path.into_iter()
                        .map(|p| Point(center.0 + p.0 * factor, center.1 + p.1 * factor))
                        .collect(),
                )
            }
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Cell;
    use rand::{SeedableRng, rngs::StdRng};
    fn frame(time: f32, max: bool, w: usize, h: usize) -> String {
        let v: Vec<_> = params()
            .iter()
            .map(|p| if max { p.max } else { p.default })
            .collect();
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
    fn gothic_icons_views() {
        insta::assert_snapshot!("icons_seed7_t04", frame(0.4, false, 100, 36));
        insta::assert_snapshot!("icons_seed7_t11", frame(1.1, false, 100, 36));
    }
    #[test]
    fn gothic_icons_replay_and_bounds() {
        assert_eq!(frame(0.4, false, 80, 24), frame(0.4, false, 80, 24));
        assert_ne!(frame(0.4, false, 80, 24), frame(1.1, false, 80, 24));
        for (w, h) in [(0, 0), (1, 1), (3, 2), (80, 24)] {
            frame(0.7, true, w, h);
        }
    }
}
