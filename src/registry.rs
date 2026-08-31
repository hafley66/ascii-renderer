#![allow(warnings)]

use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::BTreeMap;
use std::io::{self, IsTerminal, Read as _};
use std::sync::OnceLock;

use crate::automata::*;
use crate::biomes::*;
use crate::color::*;
use crate::content::*;
use crate::fills::*;
use crate::layout::*;
use crate::markdown::*;
use crate::mondrian::*;
use crate::render::*;
use crate::scene::*;
use crate::sprites::*;
use crate::tree_draw::*;
use crate::types::*;
use crate::walker::*;
use crate::avant::*;
use crate::automata; use crate::avant; use crate::biomes; use crate::borders; use crate::color; use crate::content; use crate::fills; use crate::layout; use crate::markdown; use crate::mondrian; use crate::render; use crate::scene; use crate::sprites; use crate::tree_draw; use crate::types; use crate::walker;
use crate::cli::*;
use crate::gridio::*;
use crate::ink::*;
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::warps::*;

/// One tunable knob. `key` is the env suffix (ASCII_P_<KEY>) and the renderer
/// reads it via `param_f32(key, default)`.
#[derive(Clone, Copy)]
pub(crate) struct Param {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) default: f32,
    pub(crate) step: f32,
}


/// How the `a` key animates a mode.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum AnimKind {
    Iterate, // native time T: re-render the mode with a live clock
    Vflow,   // flow the Voronoi sites (stained)
    Morph,   // tween across adjacent seeds (transport)
}


/// Declared config for a mode.
pub(crate) struct ModeSpec {
    pub(crate) animate: AnimKind,
    pub(crate) params: &'static [Param],
}


/// Explicit inputs for one deterministic mode frame.
pub(crate) struct ModeFrame<'a> {
    pub(crate) grid: &'a mut Grid,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) seed: u64,
    pub(crate) palette: &'a [Color; 5],
    pub(crate) rng: &'a mut StdRng,
    pub(crate) time: f32,
    pub(crate) args: &'a [String],
}


/// Standalone modes implement this object-safe surface. The generated
/// `modes/mod.rs` registers one file-owned static per implementation.
pub(crate) trait Mode: Sync {
    fn name(&self) -> &'static str;
    fn help(&self) -> &'static str;
    fn animation(&self) -> AnimKind;
    fn params(&self) -> &'static [Param];
    fn render(&self, frame: &mut ModeFrame<'_>);
}


#[derive(Default)]
pub(crate) struct ModeRegistry {
    modes: BTreeMap<&'static str, &'static dyn Mode>,
}


impl ModeRegistry {
    pub(crate) fn add(&mut self, mode: &'static dyn Mode) {
        assert!(
            self.modes.insert(mode.name(), mode).is_none(),
            "duplicate registered mode name: {}",
            mode.name(),
        );
    }

    pub(crate) fn get(&self, name: &str) -> Option<&'static dyn Mode> {
        self.modes.get(name).copied()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&'static str, &'static dyn Mode)> + '_ {
        self.modes.iter().map(|(name, mode)| (*name, *mode))
    }
}


static REGISTERED_MODES: OnceLock<ModeRegistry> = OnceLock::new();


pub(crate) fn registered_modes() -> &'static ModeRegistry {
    REGISTERED_MODES.get_or_init(|| {
        let mut registry = ModeRegistry::default();
        crate::modes::register_all(&mut registry);
        registry
    })
}


pub(crate) fn registered_mode(name: &str) -> Option<&'static dyn Mode> {
    registered_modes().get(name)
}


/// One knob: `param!(KEY, label, min, max, default, step)`. KEY is the env suffix
/// (ASCII_P_<KEY>); the renderer reads it via `param_f32(KEY, default)`.
macro_rules! param {
    ($key:literal, $label:literal, $min:expr, $max:expr, $default:expr, $step:expr) => {
        Param { key: $key, label: $label, min: $min, max: $max, default: $default, step: $step }
    };
}


/// A reusable config form: the mode name(s) it applies to, how they animate, and
/// their tunable knobs. The demo panel renders the knobs and the `a` key picks the
/// animate strategy. One row here is all a mode needs -- no per-mode wiring.
pub(crate) struct ModeForm {
    pub(crate) names: &'static [&'static str],
    pub(crate) animate: AnimKind,
    pub(crate) params: &'static [Param],
}


/// The form registry. To give a mode a config form, add ONE row: list its name(s),
/// the animate kind, and its knobs (inline via `param!`). Modes absent here get the
/// default form (iterate, no knobs).
pub(crate) static MODE_FORMS: &[ModeForm] = &[
    ModeForm {
        names: &["delta"],
        animate: AnimKind::Iterate,
        params: &[
            param!("K", "stiffness", 0.5, 12.0, 4.0, 0.5),
            param!("D", "inertia", 0.001, 0.03, 0.0055, 0.001),
            param!("ZETA", "damping", 0.02, 1.0, 0.18, 0.02),
            param!("WIND", "wind", 0.0, 3.0, 1.0, 0.1),
            param!("TURB", "turbulence", 0.0, 3.0, 1.0, 0.1),
            param!("RBOW", "rainbow", 0.0, 1.0, 0.0, 0.25),
        ],
    },
    ModeForm {
        names: &["snakes"],
        animate: AnimKind::Iterate,
        params: &[
            param!("COUNT", "count", 1.0, 60.0, 8.0, 1.0),
            param!("TURN", "turn", 0.0, 0.9, 0.35, 0.05),
            param!("SPEED", "speed", 1.0, 10.0, 4.0, 0.5),
            param!("LEN", "length", 4.0, 40.0, 22.0, 2.0),
            param!("HOP", "hop", 0.0, 1.0, 0.0, 0.05),
            param!("HOPC", "hopx", 0.2, 1.5, 0.4, 0.1),
            param!("RBOW", "rainbow", 0.0, 1.0, 0.0, 0.25),
        ],
    },
    ModeForm {
        names: &["fullmetal-eyes", "fullmetal-eyes2", "eyes3", "solar-system"],
        animate: AnimKind::Iterate,
        params: &[],
    },
    ModeForm {
        names: &["hypercube"],
        animate: AnimKind::Iterate,
        params: &[
            param!("COPIES", "copies", 1.0, 5.0, 3.0, 1.0),
            param!("SPEED", "speed", 0.1, 3.0, 1.0, 0.1),
            param!("GHOSTS", "afterimage", 0.0, 5.0, 2.0, 1.0),
        ],
    },
    ModeForm {
        names: &["flux"],
        animate: AnimKind::Iterate,
        params: &[
            param!("COUNT", "particles", 8.0, 140.0, 58.0, 4.0),
            param!("TRAIL", "trail", 1.0, 18.0, 8.0, 1.0),
            param!("SPEED", "speed", 0.1, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm {
        names: &["fireworks"],
        animate: AnimKind::Iterate,
        params: &[
            param!("BURSTS", "bursts", 1.0, 12.0, 6.0, 1.0),
            param!("SPARKS", "sparks", 6.0, 48.0, 22.0, 2.0),
            param!("SPEED", "speed", 0.1, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm {
        names: &["murmuration"],
        animate: AnimKind::Iterate,
        params: &[
            param!("BIRDS", "birds", 8.0, 500.0, 140.0, 8.0),
            param!("FLOCKS", "flocks", 1.0, 9.0, 3.0, 1.0),
            param!("SPEED", "speed", 0.1, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm {
        names: &["lanterns"],
        animate: AnimKind::Iterate,
        params: &[
            param!("COUNT", "lanterns", 1.0, 24.0, 7.0, 1.0),
            param!("RISE", "rise", 0.1, 3.0, 1.0, 0.1),
            param!("SWAY", "sway", 0.0, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm {
        names: &["tide"],
        animate: AnimKind::Iterate,
        params: &[
            param!("WAVES", "waves", 1.0, 4.0, 2.0, 1.0),
            param!("AMP", "amplitude", 0.2, 2.5, 1.0, 0.1),
            param!("SPEED", "speed", 0.1, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm {
        names: &["fireflies"],
        animate: AnimKind::Iterate,
        params: &[
            param!("COUNT", "fireflies", 2.0, 60.0, 14.0, 1.0),
            param!("GLOW", "glow", 0.2, 3.0, 1.0, 0.1),
            param!("SPEED", "speed", 0.1, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm {
        names: &["ink"],
        animate: AnimKind::Iterate,
        params: &[
            param!("DROPS", "drops", 1.0, 9.0, 5.0, 1.0),
            param!("SWIRL", "swirl", 0.0, 3.0, 1.0, 0.1),
            param!("SPEED", "speed", 0.1, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm {
        names: &["meteors"],
        animate: AnimKind::Iterate,
        params: &[
            param!("STARS", "stars", 20.0, 300.0, 90.0, 10.0),
            param!("RATE", "rate", 0.2, 4.0, 1.0, 0.1),
            param!("SPEED", "speed", 0.1, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm {
        names: &["elevator"],
        animate: AnimKind::Iterate,
        params: &[
            param!("LIFTS", "lifts", 1.0, 6.0, 3.0, 1.0),
            param!("SPEED", "speed", 0.1, 3.0, 1.0, 0.1),
            param!("CROWD", "crowd", 0.0, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm {
        names: &["ferris"],
        animate: AnimKind::Iterate,
        params: &[
            param!("RADIUS", "radius", 4.0, 12.0, 8.0, 1.0),
            param!("GONDOLAS", "gondolas", 4.0, 14.0, 10.0, 1.0),
            param!("SPEED", "speed", 0.1, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm {
        names: &["fa6", "fullmetal-alchemist6"],
        animate: AnimKind::Iterate,
        params: &[
            param!("CELLS", "chambers", 3.0, 16.0, 8.0, 1.0),
            param!("DENS", "inscriptions", 0.0, 100.0, 55.0, 5.0),
            param!("SPEED", "speed", 0.1, 3.0, 0.8, 0.1),
            param!("CHAOS", "asymmetry", 0.0, 100.0, 42.0, 5.0),
        ],
    },
    ModeForm {
        names: &["arboretum"],
        animate: AnimKind::Iterate,
        params: &[
            param!("DENS", "density", 2.0, 60.0, 16.0, 1.0),
            param!("STRATA", "layers", 1.0, 4.0, 3.0, 1.0),
            param!("GIRTH", "size span", 0.3, 3.0, 1.2, 0.1),
            param!("CLUMP", "clumping", 0.0, 1.0, 0.5, 0.05),
            param!("FERNS", "undergrowth", 0.0, 1.0, 0.5, 0.05),
            param!("RELIEF", "relief", 0.0, 1.0, 0.5, 0.05),
            param!("GALE", "wind", -1.0, 1.0, 0.15, 0.05),
            param!("DRIFT", "hue drift", -180.0, 180.0, 60.0, 10.0),
            param!("HAZE", "haze", 0.0, 1.0, 0.45, 0.05),
            param!("CLEAR", "clearings", 0.0, 1.0, 0.25, 0.05),
            param!("SPEED", "anim speed", 0.1, 3.0, 1.0, 0.1),
            param!("SPECIES", "species mix", 0.0, 1.0, 0.7, 0.05),
        ],
    },
    ModeForm {
        names: &["astrolabe"],
        animate: AnimKind::Iterate,
        params: &[
            param!("STARS", "stars", 10.0, 110.0, 42.0, 2.0),
            param!("RINGS", "almucantars", 2.0, 9.0, 5.0, 1.0),
            param!("SPOKES", "azimuths", 4.0, 16.0, 8.0, 1.0),
            param!("RATE", "rete rate", 0.0, 0.5, 0.08, 0.01),
            param!("RULEV", "rule rate", 0.0, 0.5, 0.05, 0.01),
            param!("TWINK", "twinkle", 0.0, 1.0, 0.6, 0.05),
            param!("ZOD", "ecliptic", 0.0, 1.0, 1.0, 0.05),
        ],
    },
    ModeForm {
        names: &["sauron"],
        animate: AnimKind::Iterate,
        params: &[
            param!("BLAZE", "blaze", 0.0, 2.0, 1.0, 0.1),
            param!("GAZE", "gaze", 0.0, 1.0, 0.8, 0.05),
            param!("SLIT", "slit width", 1.0, 5.0, 2.0, 1.0),
            param!("IRIS", "iris size", 0.15, 0.95, 0.7, 0.05),
            param!("EMBERS", "embers", 0.0, 90.0, 26.0, 2.0),
            param!("TURB", "turbulence", 0.0, 3.0, 1.0, 0.1),
        ],
    },
    ModeForm { names: &["stained"], animate: AnimKind::Vflow, params: &[] },
    ModeForm {
        names: &["chimera"],
        animate: AnimKind::Iterate,
        params: &[
            param!("DENS", "density", 10.0, 100.0, 50.0, 5.0),
            param!("DRIFT", "drift", 0.0, 10.0, 2.0, 0.5),
        ],
    },
];


/// Look up a mode's declared config. Unlisted modes default to iterate, no knobs:
/// T animates the mode natively if it reads it (in-process via iterate_grid),
/// otherwise the player warps the base frame over time.
pub(crate) fn mode_spec(name: &str) -> ModeSpec {
    if let Some(mode) = registered_mode(name) {
        return ModeSpec { animate: mode.animation(), params: mode.params() };
    }
    for f in MODE_FORMS {
        if f.names.contains(&name) {
            return ModeSpec { animate: f.animate, params: f.params };
        }
    }
    ModeSpec { animate: AnimKind::Iterate, params: &[] }
}


#[cfg(test)]
mod registered_mode_tests {
    use super::*;

    #[test]
    fn generated_registry_contains_the_file_owned_modes() {
        let names: Vec<_> = registered_modes().iter().map(|(name, _)| name).collect();
        assert!(names.contains(&"illuminarium"));
        assert!(names.contains(&"qwen-cathedral"));
        assert!(registered_modes().iter().all(|(_, mode)| !mode.help().is_empty()));
    }
}


/// Strategy string the morph player understands for a given animate kind.
pub(crate) fn anim_strat(k: AnimKind) -> &'static str {
    match k {
        AnimKind::Iterate => "iterate",
        AnimKind::Vflow => "vflow",
        AnimKind::Morph => "transport",
    }
}


/// Paint the options pane into the right region (columns >= `x0`). Shows the
/// mode's declared animate kind and its tunable knobs as labelled sliders, with
/// the selected knob highlighted. Positions every row with an absolute cursor
/// escape so it never disturbs the mode render in the left columns.
pub(crate) fn draw_options_pane(
    x0: usize, // 0-based column where the pane region starts (== render_w)
    th: u16,
    mode: &str,
    spec: &ModeSpec,
    pvals: &[f32],
    psel: usize,
    seed: u64,
    theme: &str,
    randomize: bool,
) {
    use std::io::Write;
    let col = x0 + 2; // 1-based content column (column x0+1 holds the divider)
    let rows = th.saturating_sub(1) as usize; // last terminal row is the status bar
    let mut out = String::new();
    for r in 0..rows {
        out.push_str(&format!("\x1b[{};{}H\x1b[90m\u{2502}\x1b[0m", r + 1, x0 + 1));
    }
    let mut line = |r: usize, text: &str| {
        if r < rows {
            out.push_str(&format!("\x1b[{};{}H{}", r + 1, col, text));
        }
    };
    let kind = match spec.animate {
        AnimKind::Iterate => "iterate (native T)",
        AnimKind::Vflow => "vflow (voronoi)",
        AnimKind::Morph => "morph (seeds)",
    };
    let theme_label = if theme.is_empty() { "auto" } else { theme };
    let knobs_mode = if randomize {
        "\x1b[1mRANDOM\x1b[0m \x1b[90m(g)\x1b[0m"
    } else {
        "manual \x1b[90m(g)\x1b[0m"
    };
    line(0, "\x1b[1mANIM OPTIONS\x1b[0m");
    line(1, &format!("mode  {}", mode));
    line(2, &format!("anim  {}", kind));
    line(3, &format!("seed  {}  theme {}", seed, theme_label));
    line(5, &format!("knobs {}", knobs_mode));
    line(4, "\x1b[90m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m");
    if spec.params.is_empty() {
        line(6, "\x1b[90mno tunables for this mode\x1b[0m");
        line(8, "press \x1b[1ma\x1b[0m to animate");
    } else {
        let bar_w = 12usize;
        for (i, p) in spec.params.iter().enumerate() {
            let v = pvals[i];
            let frac = ((v - p.min) / (p.max - p.min)).clamp(0.0, 1.0);
            let filled = (frac * bar_w as f32).round() as usize;
            let bar: String = (0..bar_w)
                .map(|k| if k < filled { '\u{2588}' } else { '\u{2591}' })
                .collect();
            let row = 6 + i * 2;
            if i == psel {
                line(row, &format!("\x1b[7m> {:<10}\x1b[0m", p.label));
            } else {
                line(row, &format!("  {:<10}", p.label));
            }
            line(row + 1, &format!("  {} {:>7.3}", bar, v));
        }
        let foot = 6 + spec.params.len() * 2 + 1;
        line(foot, "\x1b[90m<>=adjust ^v=select r=reset\x1b[0m");
        line(foot + 1, "press \x1b[1ma\x1b[0m to animate");
    }
    print!("{}", out);
    io::stdout().flush().unwrap();
}
