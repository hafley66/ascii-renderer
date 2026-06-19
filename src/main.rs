#![allow(warnings)]

mod automata;
mod biomes;
mod borders;
mod color;
mod content;
mod fills;
mod layout;
mod markdown;
mod mondrian;
mod render;
mod scene;
mod sprites;
mod tree_draw;
mod types;
mod walker;

use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

use automata::*;
use biomes::*;
use color::*;
use content::*;
use fills::*;
use layout::*;
use markdown::*;
use mondrian::*;
use render::*;
use scene::*;
use sprites::*;
use tree_draw::*;
use types::*;
use walker::*;

// ============================================================================
// Declarative mode config. A mode declares how it animates and which runtime
// knobs it exposes. The demo panel reads this to render the options pane and to
// pick the animate strategy; renderers read the knob values from env vars
// (ASCII_P_<KEY>) so values flow demo -> env -> subprocess render with no
// per-mode wiring. Unlisted modes get the default spec (morph, no knobs).
// ============================================================================

/// One tunable knob. `key` is the env suffix (ASCII_P_<KEY>) and the renderer
/// reads it via `param_f32(key, default)`.
#[derive(Clone, Copy)]
struct Param {
    key: &'static str,
    label: &'static str,
    min: f32,
    max: f32,
    default: f32,
    step: f32,
}

/// How the `a` key animates a mode.
#[derive(Clone, Copy, PartialEq)]
enum AnimKind {
    Iterate, // native time T: re-render the mode with a live clock
    Vflow,   // flow the Voronoi sites (stained)
    Morph,   // tween across adjacent seeds (transport)
}

/// Declared config for a mode.
struct ModeSpec {
    animate: AnimKind,
    params: &'static [Param],
}

static DELTA_PARAMS: &[Param] = &[
    Param { key: "K",    label: "stiffness",  min: 0.5,   max: 12.0, default: 4.0,    step: 0.5 },
    Param { key: "D",    label: "inertia",    min: 0.001, max: 0.03, default: 0.0055, step: 0.001 },
    Param { key: "ZETA", label: "damping",    min: 0.02,  max: 1.0,  default: 0.18,   step: 0.02 },
    Param { key: "WIND", label: "wind",       min: 0.0,   max: 3.0,  default: 1.0,    step: 0.1 },
    Param { key: "TURB", label: "turbulence", min: 0.0,   max: 3.0,  default: 1.0,    step: 0.1 },
    Param { key: "RBOW", label: "rainbow",    min: 0.0,   max: 1.0,  default: 0.0,    step: 0.25 },
];

/// Look up a mode's declared config. Unlisted modes default to seed-morph, no knobs.
fn mode_spec(name: &str) -> ModeSpec {
    match name {
        "delta" => ModeSpec { animate: AnimKind::Iterate, params: DELTA_PARAMS },
        "fullmetal-eyes" | "fullmetal-eyes2" | "eyes3" | "solar-system" => {
            ModeSpec { animate: AnimKind::Iterate, params: &[] }
        }
        "stained" => ModeSpec { animate: AnimKind::Vflow, params: &[] },
        // Default: iterate. T animates the mode -- natively if the mode reads it
        // (in-process via iterate_grid), otherwise the player warps the base frame
        // over time, so every mode animates per seed with no per-frame re-render.
        _ => ModeSpec { animate: AnimKind::Iterate, params: &[] },
    }
}

/// Strategy string the morph player understands for a given animate kind.
fn anim_strat(k: AnimKind) -> &'static str {
    match k {
        AnimKind::Iterate => "iterate",
        AnimKind::Vflow => "vflow",
        AnimKind::Morph => "transport",
    }
}

/// Read a runtime knob set by the demo panel (env ASCII_P_<KEY>), or `default`.
fn param_f32(key: &str, default: f32) -> f32 {
    std::env::var(format!("ASCII_P_{}", key))
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Indices of modes whose name contains `query` (case-insensitive). Empty query
/// matches all, preserving order.
fn demo_filter_modes(all_modes: &[&str], query: &str) -> Vec<usize> {
    let ql = query.to_lowercase();
    all_modes
        .iter()
        .enumerate()
        .filter(|(_, m)| ql.is_empty() || m.to_lowercase().contains(&ql))
        .map(|(i, _)| i)
        .collect()
}

/// Full-screen list+filter picker. Type to filter (substring, case-insensitive),
/// Up/Down to move, Enter to select, Esc to cancel. Returns the chosen index into
/// `all_modes`, or None if cancelled. Caller must have raw mode enabled.
fn demo_pick_mode(all_modes: &[&str], current: usize) -> Option<usize> {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{self, ClearType},
    };
    use std::io::Write;

    let mut query = String::new();
    let mut sel: usize = current; // index into the filtered list

    loop {
        let filtered = demo_filter_modes(all_modes, &query);
        if sel >= filtered.len() {
            sel = filtered.len().saturating_sub(1);
        }

        let (tw, th) = terminal::size().unwrap_or((80, 45));
        let tw = tw as usize;
        let th = th as usize;
        // `cancel_idx` is a virtual last entry pinned below a divider. sel ranges
        // 0..=cancel_idx; landing on it and pressing enter cancels.
        let cancel_idx = filtered.len();
        if sel > cancel_idx {
            sel = cancel_idx;
        }
        // reserve 2 header rows + 2 rows for the divider and the cancel entry.
        let list_h = th.saturating_sub(5).max(1);
        let anchor = sel.min(filtered.len().saturating_sub(1));
        let offset = if filtered.len() <= list_h || anchor < list_h / 2 {
            0
        } else {
            (anchor - list_h / 2).min(filtered.len().saturating_sub(list_h))
        };

        let query_disp = if query.is_empty() {
            "\u{2026}".to_string()
        } else {
            query.clone()
        };
        let mut buf = String::new();
        buf.push_str(&format!(
            "\x1b[7m \u{1f50d} search \x1b[0m {}\u{2588}  \u{2502}  {}/{} match  \u{2502}  \u{2191}\u{2193} move \u{00b7} type to filter \u{00b7} enter select \u{00b7} esc cancel\r\n\r\n",
            query_disp,
            filtered.len(),
            all_modes.len()
        ));
        for row in 0..list_h {
            let fi = offset + row;
            if fi >= filtered.len() {
                buf.push_str("\r\n");
                continue;
            }
            let name = all_modes[filtered[fi]];
            if fi == sel {
                let pad = tw.saturating_sub(name.chars().count() + 3);
                buf.push_str(&format!("\x1b[7m \u{25b8} {}{} \x1b[0m\r\n", name, " ".repeat(pad)));
            } else {
                buf.push_str(&format!("   {}\r\n", name));
            }
        }
        // divider + pinned cancel entry.
        buf.push_str(&format!("\x1b[90m{}\x1b[0m\r\n", "\u{2500}".repeat(tw.min(40))));
        if sel == cancel_idx {
            let label = "\u{2715} cancel";
            let pad = tw.saturating_sub(label.chars().count() + 3);
            buf.push_str(&format!("\x1b[7m \u{25b8} {}{} \x1b[0m", label, " ".repeat(pad)));
        } else {
            buf.push_str("   \x1b[90m\u{2715} cancel\x1b[0m");
        }

        execute!(
            io::stdout(),
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .unwrap();
        print!("{}", buf);
        io::stdout().flush().unwrap();

        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Esc => return None,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return None,
                KeyCode::Enter => {
                    if sel == filtered.len() {
                        return None; // cancel entry
                    }
                    return filtered.get(sel).copied();
                }
                KeyCode::Up => sel = sel.saturating_sub(1),
                KeyCode::Down => {
                    if sel < filtered.len() {
                        sel += 1; // can land on the cancel entry
                    }
                }
                KeyCode::Backspace => {
                    query.pop();
                    sel = 0;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    query.push(c);
                    sel = 0;
                }
                _ => {}
            }
        }
    }
}

/// Paint the options pane into the right region (columns >= `x0`). Shows the
/// mode's declared animate kind and its tunable knobs as labelled sliders, with
/// the selected knob highlighted. Positions every row with an absolute cursor
/// escape so it never disturbs the mode render in the left columns.
fn draw_options_pane(
    x0: usize, // 0-based column where the pane region starts (== render_w)
    th: u16,
    mode: &str,
    spec: &ModeSpec,
    pvals: &[f32],
    psel: usize,
    seed: u64,
    theme: &str,
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
    line(0, "\x1b[1mANIM OPTIONS\x1b[0m");
    line(1, &format!("mode  {}", mode));
    line(2, &format!("anim  {}", kind));
    line(3, &format!("seed  {}  theme {}", seed, theme_label));
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

fn run_demo(initial_seed: u64) {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{self, ClearType},
    };
    use std::io::Write;
    use std::process::Command;

    let all_modes: &[&str] = &[
        "party",
        "soup",
        "tree",
        "trees",
        "forest",
        "forest2",
        "forest3",
        "forest4",
        "forest5",
        "forest6",
        "forest7",
        "forest8",
        "forest9",
        "aztec",
        "fret",
        "flowers",
        "fruits",
        "masks",
        "shapes",
        "tiles",
        "tiles-rand",
        "tiles-skew",
        "mondrian",
        "mondrian2",
        "bsp",
        "layout",
        "terrain",
        "flow",
        "watershed",
        "noise",
        "ca",
        "stem",
        "scene-walk",
        "scene-walk-2",
        "scene-walk-3",
        "aurora",
        "aura2",
        "solar-system",
        "harbor",
        "labyrinth",
        "eyes",
        "eyes2",
        "fullmetal-eyes",
        "fullmetal-eyes2",
        "fullmetal-alchemist",
        "fullmetal-alchemist2",
        "fa3",
        "fa4",
        "fa5",
        "spiro",
        "spiro-tile",
        "weave",
        "gears",
        "kaleido",
        "contour",
        "eyes3",
        "rainfall",
        "meadow",
        "world",
        "world2",
        "boles1",
        "boles2",
        "boles3",
        "trunks1",
        "trees1",
        "trees2",
        "trees3",
        "trees4",
        "trees8",
        "trees9",
        "trees10",
        "boles4",
        "boles5",
        "boles6",
        "bushes",
        "kintsugi",
        "constellation",
        "strata",
        "circuit",
        "quilt",
        "patchwalk",
        "eyes++",
        "fullmetal-eyes++",
        "trees++",
        "forest++",
        "phyllotaxis",
        "moire",
        "nebula",
        "delta",
        "stained",
    ];
    let all_themes: &[&str] = &[
        "",
        "ember",
        "terracotta",
        "sakura",
        "arctic",
        "deep",
        "moss",
        "bone",
        "silver",
        "neon",
        "nerv",
        "mitla",
    ];

    let mut seed = initial_seed;
    let mut mode_idx: usize = 0;
    let mut theme_idx: usize = 0;

    // Options pane state. `spec`/`pvals` mirror the current mode's declared config;
    // they reload whenever the mode changes. When the pane is open, the up/down/
    // left/right keys edit knobs instead of seed/theme.
    let mut pane_open = false;
    let mut last_mode = "";
    let mut spec = mode_spec(all_modes[mode_idx]);
    let mut pvals: Vec<f32> = spec.params.iter().map(|p| p.default).collect();
    let mut psel: usize = 0;

    let exe = std::env::current_exe().unwrap();

    terminal::enable_raw_mode().unwrap();
    execute!(io::stdout(), terminal::EnterAlternateScreen).unwrap();

    loop {
        let current_mode = all_modes[mode_idx];
        let current_theme = all_themes[theme_idx];

        // Reload the declared config when the mode changes.
        if current_mode != last_mode {
            spec = mode_spec(current_mode);
            pvals = spec.params.iter().map(|p| p.default).collect();
            psel = 0;
            last_mode = current_mode;
        }

        let (tw, th) = terminal::size().unwrap_or((80, 45));
        // When the pane is open, reserve the right columns for it and render the
        // mode narrower (ASCII_GRID_W). Closed -> full-screen, identical to before.
        let pane_w: usize = if pane_open {
            34.min((tw as usize) / 2)
        } else {
            0
        };
        let render_w = (tw as usize).saturating_sub(pane_w);

        // Push the current knob values down to the render subprocess via env.
        // Child processes (the preview and the iterate animator) inherit these.
        // SAFETY: the demo loop is single-threaded.
        for (p, v) in spec.params.iter().zip(pvals.iter()) {
            unsafe { std::env::set_var(format!("ASCII_P_{}", p.key), format!("{}", v)) };
        }

        // Disable raw mode so child process writes normal line endings
        terminal::disable_raw_mode().unwrap();
        execute!(
            io::stdout(),
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .unwrap();

        let mut cmd = Command::new(&exe);
        cmd.arg(seed.to_string()).arg(current_mode);
        if !current_theme.is_empty() {
            cmd.arg(current_theme);
        }
        if pane_open {
            cmd.env("ASCII_GRID_W", render_w.to_string());
            cmd.env("ASCII_GRID_H", th.saturating_sub(1).to_string());
        } else {
            // SAFETY: single-threaded demo loop.
            unsafe {
                std::env::remove_var("ASCII_GRID_W");
                std::env::remove_var("ASCII_GRID_H");
            }
        }
        let _ = cmd.status();

        // Re-enable raw mode for keyboard input
        terminal::enable_raw_mode().unwrap();

        if pane_open {
            draw_options_pane(
                render_w, th, current_mode, &spec, &pvals, psel, seed, current_theme,
            );
        }

        execute!(io::stdout(), cursor::MoveTo(0, th.saturating_sub(1))).unwrap();
        let theme_label = if current_theme.is_empty() {
            "auto"
        } else {
            current_theme
        };
        let status = if pane_open {
            format!(
                " {} | o=close opts  \u{2191}\u{2193}=select  \u{2190}\u{2192}=adjust  r=reset  a=animate  q=quit ",
                current_mode
            )
        } else {
            format!(
                " {} | seed:{} | theme:{} | /=find  o=opts  a=animate  f/j=prev/next  \u{2191}\u{2193}=seed  \u{2190}\u{2192}=theme  enter=rand  q=quit ",
                current_mode, seed, theme_label
            )
        };
        // Pad to terminal width, inverse video (char-safe truncation)
        let status_w = status.chars().count();
        let padded: String = if status_w < tw as usize {
            format!("{}{}", status, " ".repeat(tw as usize - status_w))
        } else {
            status.chars().take(tw as usize).collect()
        };
        print!("\x1b[7m{}\x1b[0m", padded);
        io::stdout().flush().unwrap();

        let has_params = !spec.params.is_empty();
        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Char('o') => pane_open = !pane_open,
                KeyCode::Char('j') => mode_idx = (mode_idx + 1) % all_modes.len(),
                KeyCode::Char('f') => mode_idx = (mode_idx + all_modes.len() - 1) % all_modes.len(),
                KeyCode::Char('/') | KeyCode::Char('m') => {
                    if let Some(idx) = demo_pick_mode(all_modes, mode_idx) {
                        mode_idx = idx;
                    }
                }
                KeyCode::Char('r') if pane_open && has_params => {
                    pvals[psel] = spec.params[psel].default;
                }
                KeyCode::Char('a') => {
                    // Animate via the declared strategy. Knob env is already set, so
                    // the iterate subprocess inherits the tuned values.
                    morph_session(
                        current_mode,
                        seed,
                        current_mode,
                        seed.wrapping_add(1),
                        anim_strat(spec.animate),
                        current_theme,
                    );
                }
                KeyCode::Up => {
                    if pane_open && has_params {
                        psel = (psel + spec.params.len() - 1) % spec.params.len();
                    } else {
                        seed = seed.wrapping_add(1);
                    }
                }
                KeyCode::Down => {
                    if pane_open && has_params {
                        psel = (psel + 1) % spec.params.len();
                    } else {
                        seed = seed.wrapping_sub(1);
                    }
                }
                KeyCode::Right => {
                    if pane_open && has_params {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] + p.step).min(p.max);
                    } else {
                        theme_idx = (theme_idx + 1) % all_themes.len();
                    }
                }
                KeyCode::Left => {
                    if pane_open && has_params {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] - p.step).max(p.min);
                    } else {
                        theme_idx = (theme_idx + all_themes.len() - 1) % all_themes.len();
                    }
                }
                KeyCode::Enter => {
                    seed = rand::rng().random_range(0..10000u64);
                }
                _ => {}
            }
        }
    }

    execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
    terminal::disable_raw_mode().unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        eprintln!("ascii-renderer <seed> [mode] [theme]");
        eprintln!();
        eprintln!("ARGS:");
        eprintln!("  seed     Integer seed for deterministic RNG (default: 42)");
        eprintln!("  mode     Rendering mode (default: full demo)");
        eprintln!("  theme    Named color theme (default: seed-derived palette)");
        eprintln!();
        eprintln!("MODES:");
        eprintln!(
            "  demo      Interactive browser: f/j=mode, arrows=seed/theme, enter=random, q=quit"
        );
        eprintln!("  (none)    Full demo: Truchet bg, trees, content, flowers");
        eprintln!("  tree      GRIS-style binary trees with flowers");
        eprintln!("  forest    Mixed scene: pine, willow, palm, GRIS tree, fruits");
        eprintln!("  aztec     Aztec diamond domino tiling");
        eprintln!("  fret      Stepped fret spirals and border bands");
        eprintln!("  flowers   All 5 flower stamp styles with labels");
        eprintln!("  fruits    All 5 fruit stamp styles with labels");
        eprintln!("  layout    Two-column layout engine demo");
        eprintln!("  md        Render markdown from stdin");
        eprintln!("  bsp       BSP randomized layout demo");
        eprintln!("  mondrian  Mondrian-style colored grid layout");
        eprintln!("  tiles     Showcase all 10 tile patterns (pure deterministic)");
        eprintln!("  tiles-rand  Same patterns with randomized params");
        eprintln!("  noise     Showcase all 5 noise variants (truchet, higaki, etc.)");
        eprintln!(
            "  terrain   Layered landscape: mountains, foothills, ground with contour boundaries"
        );
        eprintln!("  flow      Vertical flow: fills morph through tapered zones");
        eprintln!(
            "  watershed Contour landscape cut by tapered, dissolving flow channels [channels]"
        );
        eprintln!(
            "  solar-system  3D-ish orbital diagram with planets, cubes, and space hardware [bodies]"
        );
        eprintln!("  masks     All 4 mask/firework sprite styles");
        eprintln!("  ca        Cellular automata: life|cave|maze|coral [style] [primitives]");
        eprintln!("  ca-layout CA as organic layout engine (text in largest regions)");
        eprintln!("  world     Vertical biome strips: forest, garden, temple, noise, geometric");
        eprintln!(
            "  party     Node islands along a path [gap] [nodes] [scale] [detail] [weather] [path]"
        );
        eprintln!("            weather: rain|snow|fog|stars|none (default: random)");
        eprintln!("            path:    line|dots|vine|river|double (default: random)");
        eprintln!("  soup      Dense overlapping node scenes along a path");
        eprintln!("  stem      Sinuous stalk with alternating shape-masked tile leaves");
        eprintln!("  boles1    Bole styles at 3 energy levels (low/mid/high)");
        eprintln!("  boles2    Experimental bole styles v2");
        eprintln!("  boles3    Refined bole styles with descriptive names");
        eprintln!("  boles4    Winding bole styles: Serpent/Braid/Coil/Taproot");
        eprintln!("  boles5    Structural bole styles: Stilts/Cairn/Hollow/Talon/Tiers/Tussock");
        eprintln!("  trunks1   Horizontal trunk algorithms + direction-aware branching");
        eprintln!(
            "  trees1    Full pipeline: tree+trunk+bole combos [energy] [fruit] [branch] [bole]"
        );
        eprintln!("  trees2    Squat horizontal boles (1-2 rows) [energy] [fruit] [branch]");
        eprintln!("  trees3    Vertical catalog: all tree types, trunks, tapers, boles");
        eprintln!("  trees4    All 17 TreeDrawer types with boles and fruit");
        eprintln!("  bushes    Full-size bole patterns as standalone bush sprites");
        eprintln!(
            "  trees8    Oak/Fountain/Windswept drawers at two energies [energy] [fruit] [branch]"
        );
        eprintln!(
            "  trees9    Fractal/L-System/Dragon/Helix drawers, winding boles [energy] [fruit] [branch]"
        );
        eprintln!("  forest7   Layered showcase forest with boles, tapers, fruit");
        eprintln!("  kintsugi  Shattered tile shards repaired with gold seams [cracks]");
        eprintln!("  constellation  Night sky with named, line-connected star clusters [count]");
        eprintln!("  strata    Geological cross-section with fossils [layers]");
        eprintln!("  circuit   PCB traces with pads, Manhattan routing [traces]");
        eprintln!("  quilt     Stitched patchwork of tile patterns [min_patch] [max_patch]");
        eprintln!("  patchwalk Quilted mondrian crossed by a waypoint trail [stops] [line_w]");
        eprintln!("  aurora    Layered night-sky ribbons over a snowy horizon [bands]");
        eprintln!("  aura2     Sparse rain behind aurora ribbons and snowfields [rain]");
        eprintln!("  harbor    Moonlit harbor with boats, piers, and blocky shoreline [boats]");
        eprintln!("  labyrinth Carved maze with entrance, exit, and glyph markers [sparkles]");
        eprintln!(
            "  rainfall  Wind-sheared rain, gutters, puddles, and bright strikes [intensity]"
        );
        eprintln!("  meadow    Windy wildflower field with stems, seed heads, and grass [density]");
        eprintln!(
            "  world2    Cracked/leaking biome shards with aurora and scene-walk islands [shards]"
        );
        eprintln!("  swatch    Color swatches for all named themes");
        eprintln!();
        eprintln!("THEMES:");
        eprintln!("  warm:  ember, terracotta, sakura");
        eprintln!("  cool:  arctic, deep, moss");
        eprintln!("  mono:  bone, silver");
        eprintln!("  vivid: neon, nerv, mitla");
        eprintln!();
        eprintln!("EXAMPLES:");
        eprintln!("  ascii-renderer 42");
        eprintln!("  ascii-renderer 42 tree mitla");
        eprintln!("  ascii-renderer 99 forest moss");
        eprintln!("  ascii-renderer 7 aztec nerv");
        eprintln!("  ascii-renderer 0 fret neon");
        eprintln!("  ascii-renderer 42 fruits");
        eprintln!("  ascii-renderer 42 layout ember");
        eprintln!("  echo '# Hello' | ascii-renderer 42 md nerv");
        eprintln!("  cat notes.md | ascii-renderer 42 md moss");
        eprintln!("  ascii-renderer 42 bsp nerv");
        eprintln!("  ascii-renderer 42 mondrian");
        eprintln!("  ascii-renderer 42 swatch");
        eprintln!();
        eprintln!("MORPH/ANIMATE (eases in/out, adapts to resize):");
        eprintln!("  keys: space=play  \u{2190}\u{2192}=scrub  w=walk  n=next  q=quit");
        eprintln!("  morph:  1 dissolve  2 field  3 transport  4 sdf");
        eprintln!("  warp:   5 wind  6 vflow(voronoi)  7 swirl  8 ripple  9 breathe  0 drift");
        eprintln!("  native: i = iterate (re-render the mode with a time T -- true motion)");
        eprintln!("  ascii-renderer 1 morph forest            # forest seed 1 \u{2194} 2, walks seeds");
        eprintln!("  ascii-renderer 1 morph forest 1 forest 1 wind   # sway one scene in the wind");
        eprintln!("  ascii-renderer 1 morph stained           # voronoi cells flow (auto)");
        eprintln!("  ascii-renderer 3 morph fullmetal-eyes2   # then press i -- the seal rotates");
        std::process::exit(0);
    }

    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(42);

    let mode = args.get(2).map(|s| s.as_str()).unwrap_or("");
    let theme_name = args.get(3).map(|s| s.as_str()).unwrap_or("");

    if mode == "demo" {
        run_demo(seed);
        return;
    }

    if mode == "morph" {
        run_morph(&args, seed, theme_name);
        return;
    }

    let (term_w, term_h) = crossterm::terminal::size().unwrap_or((80, 45));
    // ASCII_GRID_W/H override the render size (used by the morph driver to dump
    // frames at a fixed size regardless of the child's piped terminal).
    let width = std::env::var("ASCII_GRID_W")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(term_w as usize);
    let height = std::env::var("ASCII_GRID_H")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(term_h as usize);
    // Animation time for parametric modes (Tier A). Defaults to 0.0 so a normal
    // render is identical to before; the morph player's "iterate" strategy sweeps
    // it. Any inline mode branch can fold `t_anim` into its phase.
    let t_anim: f32 = std::env::var("ASCII_T")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let mut grid = vec![vec![Cell::blank(); width]; height];
    let mut rng = StdRng::seed_from_u64(seed);

    let palette = if !theme_name.is_empty() {
        named_theme(&theme_name).unwrap_or_else(|| {
            let themes = [
                "ember",
                "terracotta",
                "sakura",
                "arctic",
                "deep",
                "moss",
                "bone",
                "silver",
                "neon",
                "nerv",
                "mitla",
            ];
            eprintln!(
                "unknown theme '{}'. available: {}",
                theme_name,
                themes.join(", ")
            );
            make_palette(seed)
        })
    } else {
        make_palette(seed)
    };

    if mode == "swatch" {
        let themes = [
            "ember",
            "terracotta",
            "sakura",
            "arctic",
            "deep",
            "moss",
            "bone",
            "silver",
            "neon",
            "nerv",
            "mitla",
        ];
        let mut swatch_grid = vec![vec![Cell::blank(); 80]; themes.len() * 3 + 1];
        for (ti, name) in themes.iter().enumerate() {
            let p = named_theme(name).unwrap();
            let row = ti * 3;

            for (j, ch) in name.chars().enumerate() {
                if j < 12 {
                    swatch_grid[row][j] = Cell::new(ch, p[4]);
                }
            }

            let labels = ["bg", "pri", "sec", "acc", "txt"];
            for (ci, &color) in p.iter().enumerate() {
                let x_start = 13 + ci * 13;
                for (j, ch) in labels[ci].chars().enumerate() {
                    if x_start + j < 80 {
                        swatch_grid[row][x_start + j] = Cell::new(ch, color);
                    }
                }
                for x in x_start..x_start + 10 {
                    if x < 80 {
                        swatch_grid[row + 1][x] = Cell::with_bg('█', color, Color::Reset);
                    }
                }
                let sample = ['╱', '╲', '│', '─', '┌', '┐', '◆', '✦', '▀', '▄'];
                for (j, &ch) in sample.iter().enumerate() {
                    if x_start + j < 80 {
                        swatch_grid[row + 2][x_start + j] = Cell::new(ch, color);
                    }
                }
            }
        }
        emit_grid(&swatch_grid);
        return;
    } else if mode == "tree" {
        grow_tree(&mut grid, 20, 40, 5, 16, palette[1], &mut rng);
        grow_tree(&mut grid, 55, 42, 8, 12, palette[2], &mut rng);

        draw_flower(&mut grid, 10, 42, 0, palette[3]);
        draw_flower(&mut grid, 70, 43, 1, palette[3]);
        draw_flower(&mut grid, 38, 38, 2, palette[3]);
        draw_flower(&mut grid, 45, 20, 3, palette[1]);
        draw_flower(&mut grid, 5, 10, 4, palette[2]);
    } else if mode == "trees" {
        // Grid of all 12 tree variants. 4 columns x 3 rows.
        let cols = 4usize;
        let rows = 3usize;
        let cell_w = width / cols;
        let cell_h = height / rows;
        for row in 0..rows {
            for col in 0..cols {
                let kind = row * cols + col;
                let cx = col * cell_w + cell_w / 2;
                let root_y = (row + 1) * cell_h - 2;
                let canopy_y = row * cell_h + 2;
                let spread = (cell_w / 4).max(3);
                let color = palette[(kind % 3) + 1];
                draw_tree(
                    &mut grid, cx, root_y, canopy_y, spread, kind, color, &mut rng,
                );
                // kind label
                let label = format!("{}", kind);
                let lx = col * cell_w + 1;
                let ly = row * cell_h + 1;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < width && ly < height {
                        grid[ly][lx + j] = Cell::new(ch, darken(palette[4], 20));
                    }
                }
            }
        }
    } else if mode == "aztec" {
        draw_aztec_diamond(
            &mut grid,
            width / 2,
            height / 2,
            height / 2 - 2,
            &palette,
            &mut rng,
        );
    } else if mode == "fret" {
        draw_stepped_fret(&mut grid, 5, 5, 3, Dir::Right, palette[1]);
        draw_stepped_fret(&mut grid, 25, 5, 5, Dir::Right, palette[2]);
        draw_stepped_fret(&mut grid, 50, 5, 7, Dir::Right, palette[3]);

        draw_stepped_fret(&mut grid, 10, 20, 5, Dir::Right, palette[1]);
        draw_stepped_fret(&mut grid, 30, 30, 5, Dir::Left, palette[2]);

        draw_fret_border(&mut grid, 0, 0, width, height, 4, 0, palette[1]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 1, palette[2]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 2, palette[3]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 3, palette[1]);
    } else if mode == "flowers" {
        for i in 0..5 {
            let color = [palette[1], palette[2], palette[3], palette[1], palette[2]][i];
            draw_flower(&mut grid, 8 + i * 15, 5, i, color);
            let labels = ["diamond", "circle", "star", "box", "braille"];
            for (j, ch) in labels[i].chars().enumerate() {
                if 8 + i * 15 - 2 + j < width {
                    grid[9][8 + i * 15 - 2 + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "fruits" {
        let fruit_colors = [
            rgb(220, 50, 50),
            rgb(180, 30, 60),
            rgb(240, 180, 30),
            rgb(100, 50, 160),
            rgb(180, 200, 40),
        ];
        let labels = ["apple", "cherry", "citrus", "berry", "pear"];
        for i in 0..5 {
            draw_fruit(&mut grid, 8 + i * 15, 5, i, fruit_colors[i]);
            for (j, ch) in labels[i].chars().enumerate() {
                if 8 + i * 15 - 2 + j < width {
                    grid[9][8 + i * 15 - 2 + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "forest" {
        let ground_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], ground_color);
            }
        }

        let ground_y = height - 4;

        for y in 3..ground_y {
            for x in 2..22 {
                grid[y][x] = Cell::blank();
            }
        }
        grow_tree(&mut grid, 12, ground_y - 1, 4, 8, palette[1], &mut rng);

        for y in 5..(ground_y + 1) {
            for x in 24..40 {
                grid[y][x] = Cell::blank();
            }
        }
        draw_pine(&mut grid, 32, ground_y - 1, 3, 10, palette[2]);

        for y in 3..(ground_y + 3) {
            for x in 42..62 {
                grid[y][x] = Cell::blank();
            }
        }
        draw_willow(&mut grid, 52, ground_y - 1, 6, 8, palette[1]);

        for y in 2..(ground_y + 1) {
            for x in 64..78 {
                grid[y][x] = Cell::blank();
            }
        }
        draw_palm(&mut grid, 71, ground_y - 1, 20, palette[3], &mut rng);

        draw_fruit(&mut grid, 8, 12, 0, rgb(220, 50, 50));
        draw_fruit(&mut grid, 15, 10, 0, rgb(200, 60, 40));
        draw_fruit(&mut grid, 11, 8, 1, rgb(180, 30, 60));

        draw_fruit(&mut grid, 30, 25, 3, rgb(100, 50, 160));
        draw_fruit(&mut grid, 35, 28, 3, rgb(120, 40, 140));

        draw_fruit(&mut grid, 48, 20, 2, rgb(240, 180, 30));
        draw_fruit(&mut grid, 55, 18, 4, rgb(180, 200, 40));

        for i in 0..6 {
            let fx = 5 + i * 13;
            if fx < width - 2 {
                draw_flower(
                    &mut grid,
                    fx,
                    ground_y + 1,
                    rng.random_range(0..5),
                    palette[3],
                );
            }
        }
    } else if mode == "layout" {
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }

        let left = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational. Last deploy 2h ago.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar {
                        label: "cpu".into(),
                        value: 72.0,
                        max: 100.0,
                    },
                    ContentItem::Bar {
                        label: "mem".into(),
                        value: 4.8,
                        max: 8.0,
                    },
                    ContentItem::Bar {
                        label: "disk".into(),
                        value: 120.0,
                        max: 500.0,
                    },
                    ContentItem::Bar {
                        label: "net".into(),
                        value: 340.0,
                        max: 1000.0,
                    },
                ],
                padding: 1,
            },
        ];

        let right = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                    ContentItem::Text("tree-sit ···· 3".into()),
                    ContentItem::Text("alloy    ···· 2".into()),
                    ContentItem::Rule,
                    ContentItem::Text("◁━━ 43 LOADED".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("TASKS".into()),
                    ContentItem::Rule,
                    ContentItem::Text("▪ layout engine".into()),
                    ContentItem::Text("▪ masonry fills".into()),
                    ContentItem::Text("▪ yaml parsing".into()),
                    ContentItem::Text("▫ snapshot tests".into()),
                    ContentItem::Text("▫ fret connect".into()),
                ],
                padding: 1,
            },
        ];

        let _rects = layout_two_col(&mut grid, &left, &right, 4, 2, palette[4], palette[3]);

        draw_flower(&mut grid, width / 2, 3, rng.random_range(0..5), palette[3]);
        draw_flower(
            &mut grid,
            width / 2,
            height - 4,
            rng.random_range(0..5),
            palette[3],
        );
        draw_flower(&mut grid, 1, height / 2, rng.random_range(0..5), palette[2]);
        draw_flower(
            &mut grid,
            width - 2,
            height / 2,
            rng.random_range(0..5),
            palette[2],
        );
    } else if mode == "md" {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input).unwrap_or_default();
        let blocks = parse_markdown(&input);

        if blocks.is_empty() {
            eprintln!("no content on stdin. usage: echo '# Title' | ascii-renderer 42 md [theme]");
            std::process::exit(1);
        }

        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }

        let border_band = if width > 40 && height > 20 { 3 } else { 0 };
        let content_margin = border_band + 1;

        let rects = if blocks.len() <= 2 {
            let col_w = width.saturating_sub(content_margin * 2);
            let mut cy = content_margin;
            let mut rects = Vec::new();
            for block in &blocks {
                let (_, h) = measure_block(block, col_w);
                let h = h.min(height.saturating_sub(cy + content_margin));
                if h == 0 {
                    break;
                }
                let rect = Rect {
                    x: content_margin,
                    y: cy,
                    w: col_w,
                    h,
                };
                render_block(&mut grid, block, &rect, palette[4], palette[3]);
                rects.push(rect);
                cy += h + 1;
            }
            rects
        } else {
            layout_bsp(
                &mut grid,
                &blocks,
                content_margin,
                14,
                4,
                palette[4],
                palette[3],
                &mut rng,
            )
        };

        let content_count = blocks.len().min(rects.len());
        for i in 0..content_count {
            let style = borders::pick_border_style(&mut rng, rects[i].w, rects[i].h);
            borders::draw_box_border(&mut grid, &rects[i], &style, palette[4]);
        }

        let empty_leaves: Vec<Rect> = rects.into_iter().skip(content_count).collect();
        walk_and_fill_leaves(&mut grid, &empty_leaves, &palette, &mut rng);

        if width > 40 && height > 20 {
            let band = 3;
            draw_fret_border(&mut grid, 0, 0, width, height, band, 0, palette[2]);
            draw_fret_border(&mut grid, 0, 0, width, height, band, 1, palette[2]);
            draw_fret_border(&mut grid, 0, 0, width, height, band, 2, palette[2]);
            draw_fret_border(&mut grid, 0, 0, width, height, band, 3, palette[2]);
        }
    } else if mode == "bsp" {
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }

        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                    ContentItem::Text("Last deploy 2h ago.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar { label: "cpu".into(), value: 72.0, max: 100.0 },
                    ContentItem::Bar { label: "mem".into(), value: 4.8, max: 8.0 },
                    ContentItem::Bar { label: "disk".into(), value: 120.0, max: 500.0 },
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                    ContentItem::Text("tree-sit ···· 3".into()),
                    ContentItem::Text("alloy    ···· 2".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("TASKS".into()),
                    ContentItem::Rule,
                    ContentItem::Text("▪ layout engine".into()),
                    ContentItem::Text("▪ masonry fills".into()),
                    ContentItem::Text("▫ yaml parsing".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("NOTES".into()),
                    ContentItem::Rule,
                    ContentItem::Text("BSP splits the canvas into randomized regions. Each content block gets assigned to the largest available leaf. Remaining leaves stay as pattern fill.".into()),
                ],
                padding: 1,
            },
        ];

        let rects = layout_bsp(
            &mut grid, &blocks, 1, 12, 5, palette[4], palette[3], &mut rng,
        );

        for rect in rects.iter().skip(blocks.len()) {
            let cx = rect.x + rect.w / 2;
            let cy = rect.y + rect.h / 2;
            if rect.w >= 5 && rect.h >= 3 {
                draw_flower(&mut grid, cx, cy, rng.random_range(0..5), palette[3]);
            }
        }
    } else if mode == "mondrian" {
        let line_w = 2;

        let mut stdin_buf = String::new();
        let has_stdin = !std::io::stdin().is_terminal();
        if has_stdin {
            io::stdin()
                .read_to_string(&mut stdin_buf)
                .unwrap_or_default();
        }

        let blocks = if !stdin_buf.is_empty() {
            parse_markdown(&stdin_buf)
        } else {
            let status_msgs = [
                "All systems nominal.",
                "Drift detected. Compensating.",
                "Awaiting signal.",
                "Calibrating.",
                "Standing by.",
                "Online.",
                "Synchronizing.",
                "Lattice stable.",
            ];
            let task_sets: [&[&str]; 4] = [
                &["▪ layout engine", "▪ masonry fills", "▫ fret connect"],
                &["▪ wave collapse", "▪ L-systems", "▫ snapshot tests"],
                &["▪ signal graph", "▪ render pass", "▫ cache layer"],
                &["▪ parse phase", "▪ emit codegen", "▫ type resolve"],
            ];
            let stat = status_msgs[rng.random_range(0..status_msgs.len())];
            let tasks = task_sets[rng.random_range(0..task_sets.len())];

            let cpu_v = rng.random_range(20..95) as f64;
            let mem_v = rng.random_range(10..80) as f64 / 10.0;
            let disk_v = rng.random_range(30..450) as f64;
            let net_v = rng.random_range(50..900) as f64;

            let mut b = vec![
                ContentBlock {
                    items: vec![
                        ContentItem::Text("「 STATUS 」".into()),
                        ContentItem::Rule,
                        ContentItem::Text(stat.into()),
                    ],
                    padding: 1,
                },
                ContentBlock {
                    items: vec![
                        ContentItem::Text("METRICS".into()),
                        ContentItem::Rule,
                        ContentItem::Bar {
                            label: "cpu".into(),
                            value: cpu_v,
                            max: 100.0,
                        },
                        ContentItem::Bar {
                            label: "mem".into(),
                            value: mem_v,
                            max: 8.0,
                        },
                        ContentItem::Bar {
                            label: "disk".into(),
                            value: disk_v,
                            max: 500.0,
                        },
                        ContentItem::Bar {
                            label: "net".into(),
                            value: net_v,
                            max: 1000.0,
                        },
                    ],
                    padding: 1,
                },
            ];
            let mut task_items = vec![ContentItem::Text("TASKS".into()), ContentItem::Rule];
            for t in tasks {
                task_items.push(ContentItem::Text((*t).into()));
            }
            b.push(ContentBlock {
                items: task_items,
                padding: 1,
            });

            if rng.random_range(0..3) == 0 {
                let notes = [
                    "The map is not the territory.",
                    "Form follows function, but function follows context.",
                    "Every system is perfectly designed to produce the results it gets.",
                    "Constraints breed creativity.",
                ];
                b.push(ContentBlock {
                    items: vec![
                        ContentItem::Text("NOTES".into()),
                        ContentItem::Rule,
                        ContentItem::Text(notes[rng.random_range(0..notes.len())].into()),
                    ],
                    padding: 1,
                });
            }
            b
        };

        let fill_colors = if theme_name.is_empty() {
            let (fills, _) = mondrian_colors();
            fills
        } else {
            [
                lighten(palette[0], 40),
                palette[1],
                palette[2],
                palette[3],
                lighten(palette[0], 40),
            ]
        };
        let line_color = if theme_name.is_empty() {
            rgb(20, 20, 20)
        } else {
            darken(palette[0], 60)
        };
        let text_fg = if theme_name.is_empty() {
            rgb(20, 20, 20)
        } else {
            palette[4]
        };

        let rects = layout_mondrian(
            &mut grid,
            &blocks,
            0,
            line_w,
            12,
            5,
            text_fg,
            text_fg,
            &fill_colors,
            line_color,
            &mut rng,
        );

        let content_count = blocks.len().min(rects.len());
        let empty_leaves: Vec<Rect> = rects.into_iter().skip(content_count).collect();
        walk_and_fill_leaves(&mut grid, &empty_leaves, &palette, &mut rng);
    } else if mode == "tiles" {
        let names = [
            "asanoha",
            "seigaiha",
            "shippo",
            "bishamon",
            "yabane",
            "nowaki",
            "higaki",
            "shell",
            "granny",
            "crocodile",
        ];
        let cols = 5.min(TILE_VARIANT_COUNT);
        let rows = (TILE_VARIANT_COUNT + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = height / rows;
        for i in 0..TILE_VARIANT_COUNT {
            let col = i % cols;
            let row = i / cols;
            let x0 = col * cell_w;
            let y0 = row * cell_h;
            let r = Rect {
                x: x0,
                y: y0 + 1,
                w: cell_w,
                h: cell_h.saturating_sub(1),
            };
            let variant = tile_variant_from_index(i);
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            fill_tile_pure(&mut grid, &r, variant, c1, c2);
            for (j, ch) in names[i].chars().enumerate() {
                if x0 + j < width && y0 < height {
                    grid[y0][x0 + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "tiles-rand" {
        let names = [
            "asanoha",
            "seigaiha",
            "shippo",
            "bishamon",
            "yabane",
            "nowaki",
            "higaki",
            "shell",
            "granny",
            "crocodile",
        ];
        let cols = 5.min(TILE_VARIANT_COUNT);
        let rows = (TILE_VARIANT_COUNT + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = height / rows;
        for i in 0..TILE_VARIANT_COUNT {
            let col = i % cols;
            let row = i / cols;
            let x0 = col * cell_w;
            let y0 = row * cell_h;
            let r = Rect {
                x: x0,
                y: y0 + 1,
                w: cell_w,
                h: cell_h.saturating_sub(1),
            };
            let mut params = TileParams::randomized(&mut rng);
            params.variant = tile_variant_from_index(i);
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            let jitter = rng.random_range(0..15) as f32 / 100.0;
            fill_tile_ex(&mut grid, &r, &params, c1, c2, jitter, None, &mut rng);
            let label = format!(
                "{} d{:.0} s{} r{}",
                names[i],
                params.density * 100.0,
                params.stagger_override,
                params.rhythm_override,
            );
            for (j, ch) in label.chars().enumerate() {
                if x0 + j < width && y0 < height {
                    grid[y0][x0 + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "tiles-skew" {
        let names = [
            "asanoha",
            "seigaiha",
            "shippo",
            "bishamon",
            "yabane",
            "nowaki",
            "higaki",
            "shell",
            "granny",
            "crocodile",
        ];
        let cols = 5.min(TILE_VARIANT_COUNT);
        let rows = (TILE_VARIANT_COUNT + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = height / rows;
        let inset = 4; // shrink rect so bleed has room to show
        for i in 0..TILE_VARIANT_COUNT {
            let col = i % cols;
            let row = i / cols;
            let x0 = col * cell_w + inset;
            let y0 = row * cell_h + 2;
            let r = Rect {
                x: x0,
                y: y0,
                w: cell_w.saturating_sub(inset * 2),
                h: cell_h.saturating_sub(4),
            };
            let mut params = TileParams::new(tile_variant_from_index(i));
            params.skew = 80;
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            fill_tile_ex(&mut grid, &r, &params, c1, c2, 0.0, None, &mut rng);
            let label = format!("{} skew=80", names[i]);
            let lx = col * cell_w;
            let ly = row * cell_h;
            for (j, ch) in label.chars().enumerate() {
                if lx + j < width && ly < height {
                    grid[ly][lx + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "terrain" {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        render_terrain(&mut grid, &rect, &palette, &mut rng);
    } else if mode == "flow" {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let zones = random_flow(&rect, &palette, &mut rng);
        render_flow(&mut grid, &rect, &zones, &palette, &mut rng);
    } else if mode == "watershed" {
        // watershed [channels] -- terrain contours carved by tapered, dissolving flow strips
        let channel_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(3);
        let channel_count = channel_count.clamp(1, 6);
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };

        let ctx = terrain_scene(&rect, &palette, &mut rng);
        render_scene(&mut grid, &rect, &ctx.scene, &mut rng);
        terrain_post_pass(&mut grid, &rect, &ctx, &palette, &mut rng);

        let water_palette = [
            palette[0],
            lighten(palette[2], 28),
            lighten(palette[3], 18),
            lighten(shift_hue(palette[1], 45.0), 10),
            palette[4],
        ];

        let mut channel_centers: Vec<(usize, usize, f32, f32, f32)> = Vec::new();
        for ci in 0..channel_count {
            let slot_w = width / (channel_count + 1);
            let base_x = (slot_w * (ci + 1)
                + rng
                    .random_range(0..slot_w.max(1))
                    .saturating_sub(slot_w / 2))
            .clamp(4, width.saturating_sub(5).max(4));
            let channel_w = rng.random_range(12..24usize).min(width.max(1)).max(6);
            let phase = rng.random::<f32>() * std::f32::consts::TAU;
            let amp = rng.random_range(3..(width / 9).max(5) as u32) as f32;
            let freq = rng.random_range(9..21u32) as f32;
            channel_centers.push((base_x, channel_w, phase, amp, freq));

            let mut tile = TileParams::new(TileVariant::Seigaiha);
            tile.density = rng.random_range(70..95) as f32 / 100.0;
            tile.jitter = 0.04;
            tile.skew = 35;
            let runnel_fill = match ci % 4 {
                0 => FillGen::Tile(tile),
                1 => FillGen::Zigzag,
                2 => FillGen::Noise(NoiseVariant::Higaki),
                _ => FillGen::Weave,
            };
            let zones = vec![
                FlowZone {
                    fill: FillGen::Noise(NoiseVariant::Dot),
                    height_frac: 0.18,
                    taper: Taper::Opening,
                    width_start: 0.18,
                    width_end: 0.55,
                },
                FlowZone {
                    fill: runnel_fill,
                    height_frac: 0.27,
                    taper: Taper::Diamond,
                    width_start: 0.42,
                    width_end: 0.96,
                },
                FlowZone {
                    fill: FillGen::Tile(tile),
                    height_frac: 0.27,
                    taper: Taper::Constant,
                    width_start: 0.68,
                    width_end: 0.68,
                },
                FlowZone {
                    fill: FillGen::Noise(NoiseVariant::Grass),
                    height_frac: 0.28,
                    taper: Taper::Closing,
                    width_start: 0.95,
                    width_end: 0.48,
                },
            ];

            let mut flow_grid = vec![vec![Cell::blank(); channel_w]; height];
            let flow_rect = Rect {
                x: 0,
                y: 0,
                w: channel_w,
                h: height,
            };
            render_flow(&mut flow_grid, &flow_rect, &zones, &water_palette, &mut rng);

            for y in 0..height {
                let depth_bias = if y > height * 2 / 3 { 1.35 } else { 1.0 };
                let center = base_x as f32
                    + (y as f32 / freq + phase).sin() * amp * depth_bias
                    + (y as f32 / (freq * 0.57) + phase * 0.3).sin() * amp * 0.35;
                let center = center.round() as i32;
                for lx in 0..channel_w {
                    let cell = flow_grid[y][lx];
                    let local = lx as i32 - channel_w as i32 / 2;
                    let x = center + local;
                    if x < 0 || (x as usize) >= width || cell.ch == ' ' {
                        continue;
                    }
                    let edge = local.unsigned_abs() as f32 / (channel_w as f32 / 2.0).max(1.0);
                    if edge > 0.82 && rng.random::<f32>() < edge - 0.45 {
                        continue;
                    }
                    grid[y][x as usize] = cell;

                    if edge > 0.62 && rng.random::<f32>() < 0.28 {
                        let bank_x = x + if local < 0 { -1 } else { 1 };
                        if bank_x >= 0 && (bank_x as usize) < width {
                            let ch = DISSOLVE[rng.random_range(2..6)];
                            grid[y][bank_x as usize] = Cell::new(ch, darken(water_palette[1], 35));
                        }
                    }
                }
            }
        }

        // Pools and brighter shelves where flow channels meet terrain contours.
        for &(base_x, channel_w, phase, amp, freq) in &channel_centers {
            let col = base_x.min(width - 1);
            let crossings = [
                ctx.mountain_contour[col],
                ctx.foothill_contour[col],
                ctx.ground_contour[col],
            ];
            for (i, &cy) in crossings.iter().enumerate() {
                if cy >= height {
                    continue;
                }
                let center = base_x as f32
                    + (cy as f32 / freq + phase).sin() * amp
                    + (cy as f32 / (freq * 0.57) + phase * 0.3).sin() * amp * 0.35;
                let center = center.round() as i32;
                let rx = (channel_w as i32 / 3).max(3) + i as i32;
                let ry = 1 + i as i32;
                for dy in -ry..=ry {
                    for dx in -rx..=rx {
                        let x = center + dx;
                        let y = cy as i32 + dy;
                        if x < 0 || y < 0 || (x as usize) >= width || (y as usize) >= height {
                            continue;
                        }
                        let nx = dx as f32 / rx as f32;
                        let ny = dy as f32 / ry.max(1) as f32;
                        if nx * nx + ny * ny <= 1.0 {
                            let ch = ['≈', '~', '∿', '─'][rng.random_range(0..4usize)];
                            grid[y as usize][x as usize] =
                                Cell::new(ch, lighten(water_palette[2], 10));
                        }
                    }
                }
            }
        }

        draw_contour_ridge(
            &mut grid,
            &rect,
            &ctx.mountain_contour,
            lighten(palette[1], 35),
        );
        draw_contour_ridge(
            &mut grid,
            &rect,
            &ctx.foothill_contour,
            lighten(palette[2], 15),
        );
        draw_contour_ridge(
            &mut grid,
            &rect,
            &ctx.ground_contour,
            lighten(palette[3], 25),
        );
    } else if mode == "masks" {
        // background: diamond lattice to recreate the emergent effect
        let bg_rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        draw_diamond_lattice(
            &mut grid,
            &bg_rect,
            darken(palette[1], 60),
            darken(palette[1], 80),
        );
        let labels = ["circle", "eye", "diamond", "square"];
        for i in 0..MASK_STYLE_COUNT {
            let cx = (width / (MASK_STYLE_COUNT + 1)) * (i + 1);
            let cy = height / 2;
            let size = (height / 6).max(2).min(4);
            draw_mask(&mut grid, cx, cy, size, i, palette[(i % 3) + 1]);
            for (j, ch) in labels[i].chars().enumerate() {
                let lx = cx.saturating_sub(labels[i].len() / 2) + j;
                let ly = cy + size + 4;
                if lx < width && ly < height {
                    grid[ly][lx] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "ca" || (mode.starts_with("ca-") && mode != "ca-layout") {
        // ca, ca-life, ca-cave, ca-maze, ca-coral, ca-B3/S23
        let rule_name = if mode == "ca" { "life" } else { &mode[3..] };

        // Derive style from seed for variety
        let style = match seed % 4 {
            0 => GlyphStyle::Box,
            1 => GlyphStyle::Round,
            2 => GlyphStyle::Diagonal,
            _ => GlyphStyle::Heavy,
        };

        let (density, gens) = match rule_name {
            "cave" => (0.50, 5),
            "maze" => (0.38, 12),
            "coral" => (0.50, 8),
            _ => (0.30, 8),
        };

        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        render_automata(
            &mut grid, &rect, rule_name, density, gens, style, &palette, true, &mut rng,
        );
    } else if mode == "ca-layout" {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };

        // Content blocks to place in the largest CA regions
        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar {
                        label: "cpu".into(),
                        value: 72.0,
                        max: 100.0,
                    },
                    ContentItem::Bar {
                        label: "mem".into(),
                        value: 4.8,
                        max: 8.0,
                    },
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                ],
                padding: 1,
            },
        ];

        let text_rects = ca_layout(&mut grid, &rect, "life", 0.35, 6, &palette, &mut rng);

        // Render text content into the largest CA regions
        let mut placed = 0;
        for block in &blocks {
            // Find next region large enough for this block
            let (min_w, min_h) = measure_block(block, 40);
            let min_w = min_w.max(12);
            while placed < text_rects.len() {
                let r = &text_rects[placed];
                placed += 1;
                if r.w >= min_w && r.h >= min_h + 2 {
                    // Clear and render
                    for y in r.y..r.y + r.h {
                        for x in r.x..r.x + r.w {
                            if y < height && x < width {
                                grid[y][x] = Cell::blank();
                            }
                        }
                    }
                    render_block(&mut grid, block, r, palette[4], palette[3]);
                    let style = borders::pick_border_style(&mut rng, r.w, r.h);
                    borders::draw_box_border(&mut grid, r, &style, palette[4]);
                    break;
                }
            }
        }
    } else if mode == "shapes" {
        // 2x2 grid, shapes sized to ~30% of each quadrant, hard edges (dissolve=0).
        // rx = 2*ry throughout to correct for 2:1 terminal cell aspect ratio.
        let hw = width / 2;
        let hh = height / 2;
        let cxs = [hw / 2, hw + hw / 2];
        let cys = [hh / 2, hh + hh / 2];

        // label just above the shape
        let write_label = |grid: &mut Grid, lx: usize, ly: usize, text: &str, color: Color| {
            for (j, ch) in text.chars().enumerate() {
                if lx + j < width && ly < grid.len() {
                    grid[ly][lx + j] = Cell::new(ch, color);
                }
            }
        };

        // 1 -- Diamond (top-left)
        {
            let cx = cxs[0] as f32;
            let cy = cys[0] as f32;
            let ry = hh as f32 * 0.30;
            let rx = ry * 2.0;
            let r = Rect {
                x: 1,
                y: 1,
                w: hw - 2,
                h: hh - 2,
            };
            let scene = Scene {
                layers: vec![Layer {
                    fill: FillGen::Tile(TileParams::new(TileVariant::BishamonKikko)),
                    mask: Some(Box::new(mask_diamond(cx, cy, rx, ry, 0.0))),
                    palette,
                }],
            };
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[0].saturating_sub(3);
            let ly = (cy - ry - 2.0).max(1.0) as usize;
            write_label(&mut grid, lx, ly, "diamond", palette[4]);
        }

        // 2 -- Parallelogram (top-right)
        {
            let cx = cxs[1] as f32;
            let cy = cys[0] as f32;
            let w = hw as f32 * 0.50;
            let h = hh as f32 * 0.55;
            let r = Rect {
                x: hw + 1,
                y: 1,
                w: hw - 2,
                h: hh - 2,
            };
            let scene = Scene {
                layers: vec![Layer {
                    fill: FillGen::Tile(TileParams::new(TileVariant::Asanoha)),
                    mask: Some(Box::new(mask_parallelogram(cx, cy, w, h, 8.0, 0.0))),
                    palette,
                }],
            };
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[1].saturating_sub(6);
            let ly = (cy - h * 0.5 - 2.0).max(1.0) as usize;
            write_label(&mut grid, lx, ly, "parallelogram", palette[4]);
        }

        // 3 -- Triangle apex-up (bottom-left)
        {
            let cx = cxs[0] as f32;
            let cy = cys[1] as f32;
            let ry = hh as f32 * 0.35;
            let rx = ry * 2.0;
            let r = Rect {
                x: 1,
                y: hh + 1,
                w: hw - 2,
                h: hh - 2,
            };
            let scene = Scene {
                layers: vec![Layer {
                    fill: FillGen::Tile(TileParams::new(TileVariant::Yabane)),
                    mask: Some(Box::new(mask_triangle(cx, cy, rx, ry, TriDir::Up, 0.0))),
                    palette,
                }],
            };
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[0].saturating_sub(3);
            let ly = (cy - ry - 2.0).max((hh + 1) as f32) as usize;
            write_label(&mut grid, lx, ly, "triangle", palette[4]);
        }

        // 4 -- Trapezoid wide-at-bottom (bottom-right)
        {
            let cx = cxs[1] as f32;
            let cy = cys[1] as f32;
            let h = hh as f32 * 0.55;
            let w_top = hw as f32 * 0.12;
            let w_bot = hw as f32 * 0.55;
            let r = Rect {
                x: hw + 1,
                y: hh + 1,
                w: hw - 2,
                h: hh - 2,
            };
            let scene = Scene {
                layers: vec![Layer {
                    fill: FillGen::Tile(TileParams::new(TileVariant::Higaki)),
                    mask: Some(Box::new(mask_trapezoid(cx, cy, w_top, w_bot, h, 0.0))),
                    palette,
                }],
            };
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[1].saturating_sub(4);
            let ly = (cy - h * 0.5 - 2.0).max((hh + 1) as f32) as usize;
            write_label(&mut grid, lx, ly, "trapezoid", palette[4]);
        }

        // grid dividers
        for y in 0..height {
            if y < grid.len() {
                grid[y][hw] = Cell::new('│', darken(palette[2], 50));
            }
        }
        for x in 0..width {
            if hh < grid.len() {
                grid[hh][x] = Cell::new('─', darken(palette[2], 50));
            }
        }
        if hh < grid.len() {
            grid[hh][hw] = Cell::new('┼', darken(palette[2], 50));
        }
    } else if mode == "party" {
        // party [gap] [nodes] [scale] [detail] [weather] [path] [atmo]
        let pp = PartyParams {
            gap: args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
            nodes: args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
            scale: args.get(6).and_then(|s| s.parse().ok()).unwrap_or(50),
            detail: args.get(7).and_then(|s| s.parse().ok()).unwrap_or(50),
        };
        let weather = args
            .get(8)
            .and_then(|s| Weather::from_name(s))
            .unwrap_or_else(|| Weather::pick(&mut rng));
        let path_style = args
            .get(9)
            .and_then(|s| PathStyle::from_name(s))
            .unwrap_or_else(|| PathStyle::pick(&mut rng));
        let atmo_intensity: u32 = args.get(10).and_then(|s| s.parse().ok()).unwrap_or(50);
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let (layers, stops, boxes) = party_walk(width, height, &palette, &pp, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        // Draw connecting path between node centers
        draw_styled_path(
            &mut grid,
            &stops,
            path_style,
            darken(palette[2], 30),
            &mut rng,
        );
        // Draw box borders around each node
        let border_color = palette[4];
        for &(bx, by, bw, bh) in &boxes {
            draw_box_border(&mut grid, bx, by, bw, bh, border_color);
        }
        // Weather overlay
        apply_atmosphere(&mut grid, weather, atmo_intensity, &palette, &mut rng);
    } else if mode == "soup" {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let (layers, stops) = soup_walk(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_path_trail(&mut grid, &stops, palette[2], &mut rng);
    } else if mode == "stem" {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let (layers, spine) = path_walk_stem(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_stalk(&mut grid, &spine, palette[2]);
    } else if mode == "scene-walk" {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let layers = path_walk_layers(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
    } else if mode == "scene-walk-2" {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let (layers, stops) = path_walk_layers_2(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_path_trail(&mut grid, &stops, palette[2], &mut rng);
    } else if mode == "scene-walk-3" {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let density = 50u32;
        let (layers, stops, _boxes) =
            path_walk_layers_3(width, height, &palette, density, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_path_trail(&mut grid, &stops, palette[2], &mut rng);
    } else if mode == "forest2" {
        // Ground: truchet dirt
        let ground_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], ground_color);
            }
        }

        let ground_y = height.saturating_sub(4);

        // Place trees with varied size, position, and type
        let tree_count = rng.random_range(4..9u32) as usize;
        struct TreeSlot {
            x: usize,
            kind: usize,
            spread: usize,
            canopy_y: usize,
        }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One big centerpiece tree
        let big_x = rng.random_range((width / 4) as u32..(width * 3 / 4) as u32) as usize;
        let big_spread = rng.random_range(8..14u32) as usize;
        let big_canopy = rng.random_range(3..6u32) as usize;
        let big_kind = rng.random_range(0..12u32) as usize;
        slots.push(TreeSlot {
            x: big_x,
            kind: big_kind,
            spread: big_spread,
            canopy_y: big_canopy,
        });

        // Remaining trees scattered, varied sizes
        for _ in 0..tree_count - 1 {
            let tx = rng.random_range(6..(width - 6) as u32) as usize;
            let spread = rng.random_range(3..9u32) as usize;
            let canopy = rng.random_range(4..ground_y.saturating_sub(6).max(5) as u32) as usize;
            let kind = rng.random_range(0..12u32) as usize;
            slots.push(TreeSlot {
                x: tx,
                kind: kind,
                spread: spread,
                canopy_y: canopy,
            });
        }

        // Sort by x so they layer left to right
        slots.sort_by_key(|s| s.x);

        for slot in &slots {
            // Clear space for this tree
            let clear_left = slot.x.saturating_sub(slot.spread + 2);
            let clear_right = (slot.x + slot.spread + 2).min(width);
            for y in slot.canopy_y.saturating_sub(1)..ground_y + 2 {
                for x in clear_left..clear_right {
                    if y < height && x < width {
                        grid[y][x] = Cell::blank();
                    }
                }
            }
            let color = palette[rng.random_range(1..4)];
            draw_tree(
                &mut grid,
                slot.x,
                ground_y - 1,
                slot.canopy_y,
                slot.spread,
                slot.kind,
                color,
                &mut rng,
            );
        }

        // Flower/fruit burst radiating from the biggest tree's base
        let burst_cx = big_x;
        let burst_cy = ground_y + 1;
        let burst_count = rng.random_range(5..12u32);
        // One big flower at center
        draw_flower(
            &mut grid,
            burst_cx,
            burst_cy,
            rng.random_range(0..5),
            palette[3],
        );
        // Radial scatter around it
        for _ in 0..burst_count {
            let angle = rng.random::<f32>() * std::f32::consts::TAU;
            let radius = rng.random_range(3..16u32) as f32;
            let fx = (burst_cx as f32 + angle.cos() * radius * 1.8) as i32; // aspect correction
            let fy = (burst_cy as f32 + angle.sin() * radius * 0.6) as i32;
            if fx >= 2 && fy >= 2 && (fx as usize) < width - 2 && (fy as usize) < height - 2 {
                if rng.random_range(0..3u32) == 0 {
                    draw_fruit(
                        &mut grid,
                        fx as usize,
                        fy as usize,
                        rng.random_range(0..5),
                        palette[rng.random_range(2..4)],
                    );
                } else {
                    draw_flower(
                        &mut grid,
                        fx as usize,
                        fy as usize,
                        rng.random_range(0..5),
                        palette[rng.random_range(2..4)],
                    );
                }
            }
        }

        // Scatter a few more flower clusters near other trees
        for slot in &slots {
            let count = rng.random_range(1..4u32);
            for _ in 0..count {
                let fx = (slot.x as i32 + rng.random_range(-6..7i32)) as usize;
                let fy = ground_y + rng.random_range(0..2u32) as usize;
                if fx >= 2 && fx < width - 2 && fy < height - 2 {
                    if rng.random_range(0..2u32) == 0 {
                        draw_flower(
                            &mut grid,
                            fx,
                            fy,
                            rng.random_range(0..5),
                            palette[rng.random_range(2..4)],
                        );
                    } else {
                        draw_fruit(
                            &mut grid,
                            fx,
                            fy,
                            rng.random_range(0..5),
                            palette[rng.random_range(2..4)],
                        );
                    }
                }
            }
        }
    } else if mode == "forest3" {
        // Background: sky (sparse dots) above horizon, ground (truchet) below
        let horizon = height * 2 / 3 + rng.random_range(0..(height / 8).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 85);
        let ground_tiles = ['╱', '╲', '·', '·'];

        // Sky: sparse scattered dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..12u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Ground: truchet with some grass chars mixed in
        let grass_chars = ['╌', '╌', '∿', '~', '·'];
        for y in horizon..height {
            for x in 0..width {
                let depth = y - horizon;
                if depth < 2 {
                    // Grass transition line
                    grid[y][x] = Cell::new(
                        grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                        lighten(ground_color, 20),
                    );
                } else {
                    grid[y][x] = Cell::new(
                        ground_tiles[rng.random_range(0..ground_tiles.len() as u32) as usize],
                        darken(ground_color, (depth * 3) as u8),
                    );
                }
            }
        }

        // Tree placement: staggered roots, varied sizes
        let tree_count = rng.random_range(5..10u32) as usize;
        struct TreeSlot {
            x: usize,
            root_y: usize,
            kind: usize,
            spread: usize,
            canopy_y: usize,
        }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One kaiju tree (kind 13 = grow_kaiju_tree in the dispatch)
        let kaiju_x = rng.random_range((width / 6) as u32..(width * 5 / 6) as u32) as usize;
        let kaiju_root = horizon + rng.random_range(0..3u32) as usize;
        let kaiju_spread = rng.random_range(12..20u32) as usize;
        let kaiju_canopy = rng.random_range(2..5u32) as usize;
        slots.push(TreeSlot {
            x: kaiju_x,
            root_y: kaiju_root,
            kind: 13,
            spread: kaiju_spread,
            canopy_y: kaiju_canopy,
        });

        // Remaining trees: staggered roots along horizon zone
        for _ in 0..tree_count - 1 {
            let tx = rng.random_range(4..(width - 4) as u32) as usize;
            let root_offset = rng.random_range(0..5u32) as usize; // roots at different depths
            let root_y = horizon + root_offset;
            if root_y >= height - 1 {
                continue;
            }
            let spread = rng.random_range(3..10u32) as usize;
            let tree_height =
                rng.random_range(8..(root_y.saturating_sub(2).max(9)) as u32) as usize;
            let canopy_y = root_y.saturating_sub(tree_height).max(1);
            // Favor asymmetric/storm/dead kinds (9, 7, 12) alongside others
            let kind = rng.random_range(0..14u32) as usize;
            slots.push(TreeSlot {
                x: tx,
                root_y,
                kind,
                spread,
                canopy_y,
            });
        }

        // Sort by root_y descending so farther trees draw first (back to front)
        slots.sort_by(|a, b| a.root_y.cmp(&b.root_y).then(a.x.cmp(&b.x)));

        // Draw trees directly on background -- no clearing rectangles
        for slot in &slots {
            let color = palette[rng.random_range(1..5)];
            draw_tree(
                &mut grid,
                slot.x,
                slot.root_y,
                slot.canopy_y,
                slot.spread,
                slot.kind,
                color,
                &mut rng,
            );
        }

        // Scatter flowers/fruit along the ground, clustering near tree bases
        for slot in &slots {
            let burst_count = rng.random_range(1..5u32);
            for _ in 0..burst_count {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let radius = rng.random_range(2..10u32) as f32;
                let fx = (slot.x as f32 + angle.cos() * radius * 1.5) as i32;
                let fy = (slot.root_y as f32 + angle.sin() * radius * 0.4 + 1.0) as i32;
                if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                    let c = palette[rng.random_range(2..5)];
                    if rng.random_range(0..3u32) == 0 {
                        draw_fruit(
                            &mut grid,
                            fx as usize,
                            fy as usize,
                            rng.random_range(0..5),
                            c,
                        );
                    } else {
                        draw_flower(
                            &mut grid,
                            fx as usize,
                            fy as usize,
                            rng.random_range(0..5),
                            c,
                        );
                    }
                }
            }
        }
    } else if mode == "forest4" {
        // Like forest3 but with wild/unbalanced trees and algorithmic sprites.
        // More trees planted lower, more ground coverage.
        // Horizon at 60-80% down the screen (more sky, less grass domination)
        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky: sparse dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Clouds: 1-4 in the upper sky
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }
        // Per-column ground height: random walk so the grass edge is ragged
        let jitter_range = rng.random_range(2..6u32) as i32; // how wild the edge gets
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1; // random walk: -1, 0, or +1
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground: hue gradient with random direction sweeping across
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        // Random gradient direction
        let grad_dir = rng.random_range(0..6u32);
        // Base hue from palette
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else {
            120.0
        };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];

                // Gradient parameter t: 0.0 to 1.0, direction varies per seed
                let t = match grad_dir {
                    0 => x as f64 / width as f64,            // left to right
                    1 => 1.0 - x as f64 / width as f64,      // right to left
                    2 => depth as f64 / ground_depth as f64, // top to bottom
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0, // diagonal ↘
                    4 => {
                        ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0
                    } // diagonal ↙
                    _ => {
                        // Radial from center of ground
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // Tree placement: more trees, wider root stagger
        let tree_count = rng.random_range(5..10u32) as usize;
        struct TreeSlot {
            x: usize,
            root_y: usize,
            kind: usize,
            spread: usize,
            canopy_y: usize,
        }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One kaiju tree -- root at the grass line
        let kaiju_x = rng.random_range((width / 8) as u32..(width * 7 / 8) as u32) as usize;
        let kaiju_root =
            ground_heights[kaiju_x.min(width - 1)] + rng.random_range(0..3u32) as usize;
        let kaiju_root = kaiju_root.min(height - 2);
        slots.push(TreeSlot {
            x: kaiju_x,
            root_y: kaiju_root,
            kind: 13,
            spread: rng.random_range(14..22u32) as usize,
            canopy_y: rng.random_range(1..4u32) as usize,
        });

        // Remaining trees: favor wild (14), asymmetric (9), storm (7), dead (12)
        // Enforce minimum spacing so trees don't pile on top of each other
        let unbalanced_kinds = [14, 14, 9, 9, 7, 7, 12, 13, 15, 15, 16, 17, 17, 4, 5, 6, 11];
        let min_spacing = (width / (tree_count + 1)).max(14);
        for _ in 0..tree_count - 1 {
            let mut tx = 0usize;
            let mut placed = false;
            for _ in 0..10 {
                tx = rng.random_range(3..(width - 3) as u32) as usize;
                let too_close = slots
                    .iter()
                    .any(|s| ((s.x as i32 - tx as i32).unsigned_abs() as usize) < min_spacing);
                if !too_close {
                    placed = true;
                    break;
                }
            }
            if !placed {
                tx = rng.random_range(3..(width - 3) as u32) as usize;
            }

            // Root at grass line + small offset so trunk meets the ground
            let grass_y = ground_heights[tx.min(width - 1)];
            let root_offset = rng.random_range(0..4u32) as usize;
            let root_y = (grass_y + root_offset).min(height - 2);

            // Height tiers: some scrubby (3-8), some medium (8-20), some towering (20-root_y)
            let max_possible = root_y.saturating_sub(1).max(4);
            let tree_height = match rng.random_range(0..10u32) {
                0..=2 => rng.random_range(3..8u32.min(max_possible as u32 + 1)) as usize, // scrubby
                3..=6 => rng.random_range(8..20u32.min(max_possible as u32 + 1)) as usize, // medium
                _ => rng.random_range(20u32.min(max_possible as u32)..max_possible as u32 + 1)
                    as usize, // towering
            };
            let canopy_y = root_y.saturating_sub(tree_height).max(1);

            // Spread also tiered: narrow (1-4), medium (4-10), wide (10-20)
            let spread = match rng.random_range(0..6u32) {
                0..=1 => rng.random_range(1..5u32) as usize,
                2..=4 => rng.random_range(4..11u32) as usize,
                _ => rng.random_range(10..21u32) as usize,
            };

            let kind =
                unbalanced_kinds[rng.random_range(0..unbalanced_kinds.len() as u32) as usize];
            slots.push(TreeSlot {
                x: tx,
                root_y,
                kind,
                spread,
                canopy_y,
            });
        }

        // Back-to-front
        slots.sort_by(|a, b| a.root_y.cmp(&b.root_y).then(a.x.cmp(&b.x)));

        // Give each tree a distinct hue + depth-based brightness
        // Slots are sorted back-to-front (ascending root_y), so earlier = farther = dimmer
        let slot_count = slots.len();
        for (i, slot) in slots.iter().enumerate() {
            let base_hue =
                (i as f64 * 360.0 / slot_count as f64 + rng.random_range(0..30u32) as f64) % 360.0;
            // Depth factor: 0.0 = farthest (dim), 1.0 = closest (bright)
            let depth_t = i as f64 / (slot_count - 1).max(1) as f64;
            let lightness = 0.2 + depth_t * 0.3; // 0.2 (far) to 0.5 (near)
            let saturation = 0.4 + depth_t * 0.3;
            let color = hsl_to_rgb(base_hue, saturation, lightness);
            draw_tree(
                &mut grid,
                slot.x,
                slot.root_y,
                slot.canopy_y,
                slot.spread,
                slot.kind,
                color,
                &mut rng,
            );
        }

        // Sprout braille leaf clusters at branch tips (~50% of tips)
        let leaf_hue = rng.random_range(60..180u32) as f64; // green-ish range
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 50, &mut rng);

        // Tighter flower/fruit scatter: fewer per tree, smaller radius, only at ground level
        for slot in &slots {
            let burst = rng.random_range(0..3u32); // 0-2 instead of 2-5
            for _ in 0..burst {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let radius = rng.random_range(1..6u32) as f32; // tighter radius
                let fx = (slot.x as f32 + angle.cos() * radius * 1.5) as i32;
                // Keep at or just below root, not floating in the sky
                let fy = slot.root_y as i32 + rng.random_range(1..3u32) as i32;
                if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                    let c = palette[rng.random_range(2..5)];
                    match rng.random_range(0..3u32) {
                        0 => grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        _ => draw_flower(
                            &mut grid,
                            fx as usize,
                            fy as usize,
                            rng.random_range(0..5),
                            c,
                        ),
                    }
                }
            }
        }

        // Foreground trees: 1-3 trees planted deep in the ground, drawn last (in front)
        let fg_count = rng.random_range(1..4u32);
        for _ in 0..fg_count {
            let tx = rng.random_range(3..(width - 3) as u32) as usize;
            let grass_y = ground_heights[tx.min(width - 1)];
            let root_y = (grass_y + rng.random_range(2..6u32) as usize).min(height - 2);
            let tree_height = rng.random_range(4..12u32) as usize;
            let canopy_y = root_y.saturating_sub(tree_height).max(1);
            let spread = rng.random_range(3..10u32) as usize;
            let kind = rng.random_range(0..18u32) as usize;
            let fg_hue = rng.random_range(0..360u32) as f64;
            let color = hsl_to_rgb(fg_hue, 0.6, 0.4);
            draw_tree(
                &mut grid, tx, root_y, canopy_y, spread, kind, color, &mut rng,
            );
        }
    } else if mode == "forest5" {
        // Clustered forest: groups of same-family trees with slight color variation.
        // Center tree tallest in each cluster, edges taper. Per-tree tip decoration.
        // Root systems at trunk bases.
        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky: sparse dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Clouds
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }

        // Per-column ground height via random walk
        let jitter_range = rng.random_range(2..6u32) as i32;
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1;
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground fill with hue gradient
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        let grad_dir = rng.random_range(0..6u32);
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else {
            120.0
        };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                let t = match grad_dir {
                    0 => x as f64 / width as f64,
                    1 => 1.0 - x as f64 / width as f64,
                    2 => depth as f64 / ground_depth as f64,
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0,
                    4 => {
                        ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0
                    }
                    _ => {
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // --- Cluster placement: 1 dominant tree + 0-2 small companions ---
        // Fewer trees, more breathing room. Each cluster owns a wide horizontal zone.
        let cluster_count = rng.random_range(2..5u32) as usize;
        let zone_width = width / cluster_count.max(1);

        // Mix old tree algos (visual personality) with pen trees (connectivity).
        // Dominant trees use the interesting old kinds, companions use pen trees.
        let dominant_kinds = [0, 7, 9, 13, 14, 15, 17]; // grow_tree, storm, asymmetric, kaiju, wild, zigzag, tendril
        let family_decos = [
            TipDeco::Fruit,
            TipDeco::Drip,
            TipDeco::Flower,
            TipDeco::Fruit,
            TipDeco::Fruit,
            TipDeco::Drip,
            TipDeco::Flower,
        ];

        struct PlacedTree {
            x: usize,
            root_y: usize,
            canopy_y: usize,
            spread: usize,
            kind: usize,
            use_pen: bool,
            is_dominant: bool,
        }
        let mut all_trees: Vec<(PlacedTree, f64, usize)> = Vec::new();

        for ci in 0..cluster_count {
            let dom_kind_idx = rng.random_range(0..dominant_kinds.len() as u32) as usize;
            let dom_kind = dominant_kinds[dom_kind_idx];
            let base_hue = (ci as f64 * 360.0 / cluster_count as f64
                + rng.random_range(0..30u32) as f64)
                % 360.0;

            // Dominant tree: old algo with visual personality
            let zone_start = zone_width * ci;
            let dom_x = zone_start
                + zone_width / 2
                + rng.random_range(0..(zone_width / 4).max(1) as u32) as usize;
            let dom_x = dom_x.clamp(5, width - 5);
            let grass_y = ground_heights[dom_x.min(width - 1)];
            let dom_root = (grass_y + rng.random_range(2..8u32) as usize).min(height - 2);
            let max_h = dom_root.saturating_sub(3).max(6);
            let dom_h = rng
                .random_range((max_h as u32 / 2).max(8).min(max_h as u32)..max_h as u32 + 1)
                as usize;
            let dom_canopy = dom_root.saturating_sub(dom_h).max(1);
            let dom_spread = rng.random_range(8..16u32) as usize;

            all_trees.push((
                PlacedTree {
                    x: dom_x,
                    root_y: dom_root,
                    canopy_y: dom_canopy,
                    spread: dom_spread,
                    kind: dom_kind,
                    use_pen: false,
                    is_dominant: true,
                },
                base_hue,
                dom_kind_idx,
            ));

            // 0-2 small companion trees: pen trees (connected, small)
            let companion_count = rng.random_range(0..3u32);
            for _ in 0..companion_count {
                let offset = rng.random_range(12..25u32) as i32
                    * if rng.random_range(0..2u32) == 0 {
                        -1
                    } else {
                        1
                    };
                let cx = (dom_x as i32 + offset).clamp(3, width as i32 - 3) as usize;
                let cgrass = ground_heights[cx.min(width - 1)];
                let croot = (cgrass + rng.random_range(1..6u32) as usize).min(height - 2);
                let cmax = croot.saturating_sub(2).max(3);
                let lo = 3u32.min(cmax as u32);
                let hi = (cmax as u32 / 2 + 4).max(lo + 1);
                let ch = rng.random_range(lo..hi) as usize;
                let ccanopy = croot.saturating_sub(ch).max(1);
                let cspread = rng.random_range(2..7u32) as usize;
                let hue_jitter = rng.random_range(0..20u32) as f64 - 10.0;

                all_trees.push((
                    PlacedTree {
                        x: cx,
                        root_y: croot,
                        canopy_y: ccanopy,
                        spread: cspread,
                        kind: 0,
                        use_pen: true,
                        is_dominant: false,
                    },
                    base_hue + hue_jitter,
                    dom_kind_idx,
                ));
            }
        }

        // Sort back-to-front
        all_trees.sort_by(|a, b| a.0.root_y.cmp(&b.0.root_y).then(a.0.x.cmp(&b.0.x)));

        let total = all_trees.len();
        for (i, (tree, hue, family_idx)) in all_trees.iter().enumerate() {
            let depth_t = i as f64 / total.max(1) as f64;
            let lightness = 0.22 + depth_t * 0.28;
            let saturation = 0.40 + depth_t * 0.25;
            let color = hsl_to_rgb(*hue, saturation, lightness);

            if tree.use_pen {
                // Companion: pen tree (connected, small)
                let recipe = if rng.random_range(0..2u32) == 0 {
                    TreeRecipe::dead()
                } else {
                    TreeRecipe::columnar()
                };
                grow_pen_tree(
                    &mut grid,
                    tree.x,
                    tree.root_y,
                    tree.canopy_y,
                    tree.spread,
                    color,
                    &recipe,
                    &mut rng,
                );
            } else {
                // Dominant: old algo with visual personality
                draw_tree(
                    &mut grid,
                    tree.x,
                    tree.root_y,
                    tree.canopy_y,
                    tree.spread,
                    tree.kind,
                    color,
                    &mut rng,
                );
            }

            // Collect and decorate tips
            let x0 = tree.x.saturating_sub(tree.spread + 5);
            let x1 = (tree.x + tree.spread + 5).min(width);
            let tips = collect_tips_in_rect(&grid, x0, tree.canopy_y, x1, tree.root_y + 1);
            let deco = family_decos[*family_idx];
            let fruit_color = shift_hue(color, 60.0 + rng.random_range(0..40u32) as f64);
            decorate_tips(&mut grid, &tips, deco, fruit_color, 15, &mut rng);
        }

        // Sprout braille leaf clusters
        let leaf_hue = rng.random_range(60..180u32) as f64;
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 35, &mut rng);
    } else if mode == "forest6" {
        // Forest6: bespoke pen trees drawn next to their old equivalents for comparison.
        // Reuses forest5 sky/grass/ground layout.

        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky: sparse dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Clouds
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }

        // Per-column ground height via random walk
        let jitter_range = rng.random_range(2..6u32) as i32;
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1;
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground fill with hue gradient (same as forest5)
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        let grad_dir = rng.random_range(0..6u32);
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else {
            120.0
        };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                let t = match grad_dir {
                    0 => x as f64 / width as f64,
                    1 => 1.0 - x as f64 / width as f64,
                    2 => depth as f64 / ground_depth as f64,
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0,
                    4 => {
                        ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0
                    }
                    _ => {
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // --- Forest of trait trees (forest4-style composition) ---
        let tree_count = rng.random_range(6..12u32) as usize;
        let trait_kinds: [usize; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        struct TreeSlot {
            x: usize,
            root_y: usize,
            canopy_y: usize,
            spread: usize,
            kind: usize,
            hue: f64,
            energy: f32,
        }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One anchor tree -- tallest, widest, planted near center
        let anchor_x = rng.random_range((width / 8) as u32..(width * 7 / 8) as u32) as usize;
        let anchor_grass = ground_heights[anchor_x.min(width - 1)];
        let anchor_root = (anchor_grass + rng.random_range(0..3u32) as usize).min(height - 2);
        slots.push(TreeSlot {
            x: anchor_x,
            root_y: anchor_root,
            canopy_y: rng.random_range(1..4u32) as usize,
            spread: rng.random_range(14..22u32) as usize,
            kind: trait_kinds[rng.random_range(0..trait_kinds.len() as u32) as usize],
            hue: rng.random_range(0..360u32) as f64,
            energy: 0.95,
        });

        // Remaining trees with min spacing, height/spread tiers
        let min_spacing = (width / (tree_count + 1)).max(12);
        for _ in 0..tree_count - 1 {
            let mut tx = 0usize;
            let mut placed = false;
            for _ in 0..10 {
                tx = rng.random_range(3..(width - 3) as u32) as usize;
                let too_close = slots
                    .iter()
                    .any(|s| ((s.x as i32 - tx as i32).unsigned_abs() as usize) < min_spacing);
                if !too_close {
                    placed = true;
                    break;
                }
            }
            if !placed {
                tx = rng.random_range(3..(width - 3) as u32) as usize;
            }

            let grass_y = ground_heights[tx.min(width - 1)];
            let root_y = (grass_y + rng.random_range(0..4u32) as usize).min(height - 2);

            // Height tiers: scrubby / medium / towering
            let max_possible = root_y.saturating_sub(1).max(4);
            let tree_height = match rng.random_range(0..10u32) {
                0..=2 => rng.random_range(3..8u32.min(max_possible as u32 + 1)) as usize,
                3..=6 => rng.random_range(8..20u32.min(max_possible as u32 + 1)) as usize,
                _ => rng.random_range(20u32.min(max_possible as u32)..max_possible as u32 + 1)
                    as usize,
            };
            let canopy_y = root_y.saturating_sub(tree_height).max(1);

            // Spread tiers: narrow / medium / wide
            let spread = match rng.random_range(0..6u32) {
                0..=1 => rng.random_range(2..6u32) as usize,
                2..=4 => rng.random_range(5..12u32) as usize,
                _ => rng.random_range(10..20u32) as usize,
            };

            let kind = trait_kinds[rng.random_range(0..trait_kinds.len() as u32) as usize];
            let energy = match tree_height {
                0..=7 => rng.random_range(40..65u32) as f32 / 100.0,
                8..=19 => rng.random_range(65..85u32) as f32 / 100.0,
                _ => rng.random_range(85..100u32) as f32 / 100.0,
            };

            slots.push(TreeSlot {
                x: tx,
                root_y,
                canopy_y,
                spread,
                kind,
                hue: rng.random_range(0..360u32) as f64,
                energy,
            });
        }

        // Back-to-front depth sort
        slots.sort_by(|a, b| a.root_y.cmp(&b.root_y).then(a.x.cmp(&b.x)));

        // Depth-based brightness: farther (lower root_y) = dimmer
        let slot_count = slots.len();
        for (i, slot) in slots.iter().enumerate() {
            let depth_t = i as f64 / (slot_count - 1).max(1) as f64;
            let lightness = 0.2 + depth_t * 0.3;
            let saturation = 0.4 + depth_t * 0.3;
            let color = hsl_to_rgb(slot.hue, saturation, lightness);

            let plot_w = slot.spread * 2 + 6;
            let plot = Rect {
                x: slot.x.saturating_sub(plot_w / 2),
                y: slot.canopy_y,
                w: plot_w,
                h: slot.root_y - slot.canopy_y + 2,
            };
            let tp = TreeParams {
                plot,
                energy: slot.energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: color,
                tip_color: lighten(color, 30),
                fruit_color: shift_hue(color, 60.0),
                fruit_factor: 0.3,
                branch_factor: 0.8,
                direction: GrowDir::Up,
                bole: None,
                taper: TaperKind::default(),
            };
            match slot.kind {
                0 => SplitTree.grow(&mut grid, &tp, &mut rng),
                1 => SpiralTree.grow(&mut grid, &tp, &mut rng),
                2 => CandelabraTree.grow(&mut grid, &tp, &mut rng),
                3 => BirchTree.grow(&mut grid, &tp, &mut rng),
                4 => StormTree::new().grow(&mut grid, &tp, &mut rng),
                5 => DroopingTree.grow(&mut grid, &tp, &mut rng),
                6 => DeadTree.grow(&mut grid, &tp, &mut rng),
                7 => WavyBirch.grow(&mut grid, &tp, &mut rng),
                8 => PineTree.grow(&mut grid, &tp, &mut rng),
                9 => WillowTree.grow(&mut grid, &tp, &mut rng),
                10 => PalmTree.grow(&mut grid, &tp, &mut rng),
                _ => SpiralTree.grow(&mut grid, &tp, &mut rng),
            }
        }

        // Braille leaf clusters at branch tips
        let leaf_hue = rng.random_range(60..180u32) as f64;
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 45, &mut rng);

        // Flower/fruit scatter at ground level near tree bases
        for slot in &slots {
            let burst = rng.random_range(0..3u32);
            for _ in 0..burst {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let radius = rng.random_range(1..6u32) as f32;
                let fx = (slot.x as f32 + angle.cos() * radius * 1.5) as i32;
                let fy = slot.root_y as i32 + rng.random_range(1..3u32) as i32;
                if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                    let c = palette[rng.random_range(2..5)];
                    match rng.random_range(0..3u32) {
                        0 => grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        _ => draw_flower(
                            &mut grid,
                            fx as usize,
                            fy as usize,
                            rng.random_range(0..5),
                            c,
                        ),
                    }
                }
            }
        }

        // Foreground trees: 1-3 drawn last (in front of everything)
        let fg_count = rng.random_range(1..4u32);
        for _ in 0..fg_count {
            let tx = rng.random_range(3..(width - 3) as u32) as usize;
            let grass_y = ground_heights[tx.min(width - 1)];
            let root_y = (grass_y + rng.random_range(2..6u32) as usize).min(height - 2);
            let tree_height = rng.random_range(4..12u32) as usize;
            let canopy_y = root_y.saturating_sub(tree_height).max(1);
            let spread = rng.random_range(3..10u32) as usize;
            let kind = trait_kinds[rng.random_range(0..trait_kinds.len() as u32) as usize];
            let fg_hue = rng.random_range(0..360u32) as f64;
            let color = hsl_to_rgb(fg_hue, 0.6, 0.4);

            let plot_w = spread * 2 + 6;
            let plot = Rect {
                x: tx.saturating_sub(plot_w / 2),
                y: canopy_y,
                w: plot_w,
                h: root_y - canopy_y + 2,
            };
            let tp = TreeParams {
                plot,
                energy: 0.75,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: color,
                tip_color: lighten(color, 30),
                fruit_color: shift_hue(color, 60.0),
                fruit_factor: 0.2,
                branch_factor: 0.7,
                direction: GrowDir::Up,
                bole: None,
                taper: TaperKind::default(),
            };
            match kind {
                0 => SplitTree.grow(&mut grid, &tp, &mut rng),
                1 => SpiralTree.grow(&mut grid, &tp, &mut rng),
                2 => CandelabraTree.grow(&mut grid, &tp, &mut rng),
                3 => BirchTree.grow(&mut grid, &tp, &mut rng),
                4 => StormTree::new().grow(&mut grid, &tp, &mut rng),
                5 => DroopingTree.grow(&mut grid, &tp, &mut rng),
                6 => DeadTree.grow(&mut grid, &tp, &mut rng),
                7 => WavyBirch.grow(&mut grid, &tp, &mut rng),
                8 => PineTree.grow(&mut grid, &tp, &mut rng),
                9 => WillowTree.grow(&mut grid, &tp, &mut rng),
                10 => PalmTree.grow(&mut grid, &tp, &mut rng),
                _ => SpiralTree.grow(&mut grid, &tp, &mut rng),
            }
        }
    } else if mode == "boles1" {
        // boles1: bole styles at 3 energy levels (low/mid/high)
        let styles = [
            "Crescent", "Braille", "Frame", "Diamond", "Chevron", "Frame2",
        ];
        let energies: [f32; 3] = [0.3, 0.6, 1.0];
        let energy_labels = ["Low", "Mid", "High"];
        let col_w = width / styles.len();
        let row_h = (height - 2) / energies.len(); // 2 rows for labels

        for (si, style_name) in styles.iter().enumerate() {
            let cx = (si * col_w + col_w / 2) as i32;
            let color = lighten(palette[si % palette.len()], 40);

            // Column label at bottom
            let lx = (cx - style_name.len() as i32 / 2).max(0) as usize;
            for (j, ch) in style_name.chars().enumerate() {
                if lx + j < width {
                    grid[height - 1][lx + j] = Cell::new(ch, lighten(color, 40));
                }
            }

            for (ei, &energy) in energies.iter().enumerate() {
                let ground_y = ((ei + 1) * row_h - 2) as i32;
                if ground_y < 2 || ground_y as usize >= height - 2 {
                    continue;
                }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect {
                        x: (cx - plot_w / 2).max(0) as usize,
                        y: 0,
                        w: plot_w as usize,
                        h: (ground_y + 1) as usize,
                    },
                    energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: color,
                    tip_color: color,
                    fruit_color: color,
                    fruit_factor: 0.0,
                    branch_factor: 0.5,
                    direction: GrowDir::Up,
                    bole: None,
                    taper: TaperKind::default(),
                };

                let bole = Bole { style: si };
                let exit = bole.draw(&mut grid, &tp, &mut rng);
                let (tx, ty) = (exit.x, exit.y);

                // Short trunk stub above bole
                for y in (ground_y - (row_h as i32 / 2))..ty {
                    if y >= 0 && (y as usize) < height && (tx as usize) < width {
                        grid[y as usize][tx as usize] = Cell::new('│', color);
                    }
                }

                // Energy label to the left of each row (only in first column)
                if si == 0 {
                    let elabel = energy_labels[ei];
                    let ly = ground_y as usize;
                    if ly < height {
                        for (j, ch) in elabel.chars().enumerate() {
                            if j < cx as usize - 1 {
                                grid[ly][j] = Cell::new(ch, rgb(120, 120, 120));
                            }
                        }
                    }
                }
            }
        }
    } else if mode == "boles2" {
        // boles2: experimental bole styles v2
        let styles = [
            "Crescent2",
            "Braille2",
            "Frame3",
            "Diamond2",
            "Chevron2",
            "Frame4",
        ];
        let energies: [f32; 3] = [0.3, 0.6, 1.0];
        let energy_labels = ["Low", "Mid", "High"];
        let col_w = width / styles.len();
        let row_h = (height - 2) / energies.len();

        for (si, style_name) in styles.iter().enumerate() {
            let cx = (si * col_w + col_w / 2) as i32;
            let color = lighten(palette[si % palette.len()], 40);

            let lx = (cx - style_name.len() as i32 / 2).max(0) as usize;
            for (j, ch) in style_name.chars().enumerate() {
                if lx + j < width {
                    grid[height - 1][lx + j] = Cell::new(ch, lighten(color, 40));
                }
            }

            for (ei, &energy) in energies.iter().enumerate() {
                let ground_y = ((ei + 1) * row_h - 2) as i32;
                if ground_y < 2 || ground_y as usize >= height - 2 {
                    continue;
                }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect {
                        x: (cx - plot_w / 2).max(0) as usize,
                        y: 0,
                        w: plot_w as usize,
                        h: (ground_y + 1) as usize,
                    },
                    energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: color,
                    tip_color: color,
                    fruit_color: color,
                    fruit_factor: 0.0,
                    branch_factor: 0.5,
                    direction: GrowDir::Up,
                    bole: None,
                    taper: TaperKind::default(),
                };

                let bole = Bole { style: si + 6 };
                let exit = bole.draw(&mut grid, &tp, &mut rng);
                let (tx, ty) = (exit.x, exit.y);

                for y in (ground_y - (row_h as i32 / 2))..ty {
                    if y >= 0 && (y as usize) < height && (tx as usize) < width {
                        grid[y as usize][tx as usize] = Cell::new('│', color);
                    }
                }

                if si == 0 {
                    let elabel = energy_labels[ei];
                    let ly = ground_y as usize;
                    if ly < height {
                        for (j, ch) in elabel.chars().enumerate() {
                            if j < cx as usize - 1 {
                                grid[ly][j] = Cell::new(ch, rgb(120, 120, 120));
                            }
                        }
                    }
                }
            }
        }
    } else if mode == "boles3" {
        // boles3: refined bole styles with descriptive names
        let styles = [
            "Croissant",
            "Braille",
            "Frame",
            "Keel",
            "Chevron",
            "Buttress",
        ];
        let energies: [f32; 3] = [0.3, 0.6, 1.0];
        let energy_labels = ["Low", "Mid", "High"];
        let col_w = width / styles.len();
        let row_h = (height - 2) / energies.len();

        for (si, style_name) in styles.iter().enumerate() {
            let cx = (si * col_w + col_w / 2) as i32;
            let color = lighten(palette[si % palette.len()], 40);

            let lx = (cx - style_name.len() as i32 / 2).max(0) as usize;
            for (j, ch) in style_name.chars().enumerate() {
                if lx + j < width {
                    grid[height - 1][lx + j] = Cell::new(ch, lighten(color, 40));
                }
            }

            for (ei, &energy) in energies.iter().enumerate() {
                let ground_y = ((ei + 1) * row_h - 2) as i32;
                if ground_y < 2 || ground_y as usize >= height - 2 {
                    continue;
                }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect {
                        x: (cx - plot_w / 2).max(0) as usize,
                        y: 0,
                        w: plot_w as usize,
                        h: (ground_y + 1) as usize,
                    },
                    energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: color,
                    tip_color: color,
                    fruit_color: color,
                    fruit_factor: 0.0,
                    branch_factor: 0.5,
                    direction: GrowDir::Up,
                    bole: None,
                    taper: TaperKind::default(),
                };

                let bole = Bole { style: si + 12 };
                let exit = bole.draw(&mut grid, &tp, &mut rng);
                let (tx, ty) = (exit.x, exit.y);

                for y in (ground_y - (row_h as i32 / 2))..ty {
                    if y >= 0 && (y as usize) < height && (tx as usize) < width {
                        grid[y as usize][tx as usize] = Cell::new('│', color);
                    }
                }

                if si == 0 {
                    let elabel = energy_labels[ei];
                    let ly = ground_y as usize;
                    if ly < height {
                        for (j, ch) in elabel.chars().enumerate() {
                            if j < cx as usize - 1 {
                                grid[ly][j] = Cell::new(ch, rgb(120, 120, 120));
                            }
                        }
                    }
                }
            }
        }
    } else if mode == "boles4" {
        // boles4: winding bole styles (24-27)
        let styles = ["Serpent", "Braid", "Coil", "Taproot"];
        let energies: [f32; 3] = [0.3, 0.6, 1.0];
        let energy_labels = ["Low", "Mid", "High"];
        let col_w = width / styles.len();
        let row_h = (height - 2) / energies.len();

        for (si, style_name) in styles.iter().enumerate() {
            let cx = (si * col_w + col_w / 2) as i32;
            let color = lighten(palette[si % palette.len()], 40);

            let lx = (cx - style_name.len() as i32 / 2).max(0) as usize;
            for (j, ch) in style_name.chars().enumerate() {
                if lx + j < width {
                    grid[height - 1][lx + j] = Cell::new(ch, lighten(color, 40));
                }
            }

            for (ei, &energy) in energies.iter().enumerate() {
                let ground_y = ((ei + 1) * row_h - 3) as i32;
                if ground_y < 2 || ground_y as usize >= height - 2 {
                    continue;
                }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect {
                        x: (cx - plot_w / 2).max(0) as usize,
                        y: 0,
                        w: plot_w as usize,
                        h: (ground_y + 1) as usize,
                    },
                    energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: color,
                    tip_color: color,
                    fruit_color: color,
                    fruit_factor: 0.0,
                    branch_factor: 0.5,
                    direction: GrowDir::Up,
                    bole: None,
                    taper: TaperKind::default(),
                };

                let bole = Bole { style: si + 24 };
                let exit = bole.draw(&mut grid, &tp, &mut rng);
                let (tx, ty) = (exit.x, exit.y);

                for y in (ground_y - (row_h as i32 / 2))..ty {
                    if y >= 0 && (y as usize) < height && (tx as usize) < width {
                        grid[y as usize][tx as usize] = Cell::new('│', color);
                    }
                }

                if si == 0 {
                    let elabel = energy_labels[ei];
                    let ly = ground_y as usize;
                    if ly < height {
                        for (j, ch) in elabel.chars().enumerate() {
                            if j < cx as usize - 1 {
                                grid[ly][j] = Cell::new(ch, rgb(120, 120, 120));
                            }
                        }
                    }
                }
            }
        }
    } else if mode == "boles5" {
        // boles5: structural bole styles (28-33)
        let styles = ["Stilts", "Cairn", "Hollow", "Talon", "Tiers", "Tussock"];
        let energies: [f32; 3] = [0.3, 0.6, 1.0];
        let energy_labels = ["Low", "Mid", "High"];
        let col_w = width / styles.len();
        let row_h = (height - 2) / energies.len();

        for (si, style_name) in styles.iter().enumerate() {
            let cx = (si * col_w + col_w / 2) as i32;
            let color = lighten(palette[si % palette.len()], 40);

            let lx = (cx - style_name.len() as i32 / 2).max(0) as usize;
            for (j, ch) in style_name.chars().enumerate() {
                if lx + j < width {
                    grid[height - 1][lx + j] = Cell::new(ch, lighten(color, 40));
                }
            }

            for (ei, &energy) in energies.iter().enumerate() {
                let ground_y = ((ei + 1) * row_h - 4) as i32;
                if ground_y < 2 || ground_y as usize >= height - 2 {
                    continue;
                }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect {
                        x: (cx - plot_w / 2).max(0) as usize,
                        y: 0,
                        w: plot_w as usize,
                        h: (ground_y + 1) as usize,
                    },
                    energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: color,
                    tip_color: color,
                    fruit_color: color,
                    fruit_factor: 0.0,
                    branch_factor: 0.5,
                    direction: GrowDir::Up,
                    bole: None,
                    taper: TaperKind::default(),
                };

                let bole = Bole { style: si + 28 };
                let exit = bole.draw(&mut grid, &tp, &mut rng);
                let (tx, ty) = (exit.x, exit.y);

                for y in (ground_y - (row_h as i32 / 2) + 1)..ty {
                    if y >= 0 && (y as usize) < height && (tx as usize) < width {
                        grid[y as usize][tx as usize] = Cell::new('│', color);
                    }
                }

                if si == 0 {
                    let elabel = energy_labels[ei];
                    let ly = ground_y as usize;
                    if ly < height {
                        for (j, ch) in elabel.chars().enumerate() {
                            if j < cx as usize - 5 {
                                grid[ly][j] = Cell::new(ch, rgb(120, 120, 120));
                            }
                        }
                    }
                }
            }
        }
    } else if mode == "trunks1" {
        // trunks1: horizontal trunk algorithms + direction-aware branching
        let labels = [
            "Straight", "Wobble", "Organic", "Sine(2)", "Sine(4)", "Gnarled",
        ];
        let col_w = width / labels.len();
        let ground_y = (height as i32) - 3;

        for (i, label) in labels.iter().enumerate() {
            let cx = (i * col_w + col_w / 2) as i32;
            let color = palette[i % palette.len()];

            let plot = Rect {
                x: (i * col_w).max(1),
                y: 2,
                w: col_w.min(20),
                h: (ground_y as usize).saturating_sub(2),
            };
            let params = TreeParams {
                plot,
                energy: 0.7,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: color,
                fruit_factor: 0.0,
                branch_factor: 0.5,
                direction: GrowDir::Up,
                bole: None,
                taper: TaperKind::default(),
            };

            // Select trunk algo for this column
            use tree_draw::{
                GnarledTrunk, OrganicTrunk, SineTrunk, StraightTrunk, TreeWithTrunk, WobbleTrunk,
            };

            let tree = SpiralTree;
            match i {
                0 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(StraightTrunk {
                        height_fraction: 0.5,
                    }),
                }
                .grow(&mut grid, &params, &mut rng),
                1 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(WobbleTrunk {
                        height_fraction: 0.5,
                    }),
                }
                .grow(&mut grid, &params, &mut rng),
                2 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(OrganicTrunk {
                        height_fraction: 0.5,
                    }),
                }
                .grow(&mut grid, &params, &mut rng),
                3 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 2,
                    }),
                }
                .grow(&mut grid, &params, &mut rng),
                4 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 3,
                    }),
                }
                .grow(&mut grid, &params, &mut rng),
                5 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(GnarledTrunk),
                }
                .grow(&mut grid, &params, &mut rng),
                _ => {}
            }

            // Label
            let lx = (cx - label.len() as i32 / 2).max(0) as usize;
            for (j, ch) in label.chars().enumerate() {
                if lx + j < width {
                    grid[height - 1][lx + j] = Cell::new(ch, lighten(color, 40));
                }
            }
        }
    } else if mode == "trees1" {
        // trees1: full pipeline demo -- tree + trunk algo + bole
        // args: [energy] [fruit_factor] [branch_factor] [bole_override]
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);
        let fruit_factor: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.3);
        let branch_factor: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.5);
        let bole_override: Option<usize> = args.get(7).and_then(|s| s.parse().ok());

        let combos: Vec<(&str, Box<dyn TreeDrawer>, usize)> = vec![
            (
                "Spiral+Straight\n+Frame",
                Box::new(SpiralTree) as Box<dyn TreeDrawer>,
                14,
            ),
            (
                "Spiral+Wobble\n+Chevron",
                Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(WobbleTrunk {
                        height_fraction: 0.6,
                    }),
                }),
                16,
            ),
            (
                "Candelabra+Organic\n+Keel",
                Box::new(TreeWithTrunk {
                    tree: CandelabraTree,
                    trunk: Box::new(OrganicTrunk {
                        height_fraction: 0.5,
                    }),
                }),
                15,
            ),
            (
                "Split+Sine\n+Buttress",
                Box::new(TreeWithTrunk {
                    tree: SplitTree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 2,
                    }),
                }),
                17,
            ),
            (
                "Birch+Gnarled\n+Braille",
                Box::new(TreeWithTrunk {
                    tree: BirchTree,
                    trunk: Box::new(GnarledTrunk),
                }),
                13,
            ),
            (
                "Drooping+Sine\n+Frame",
                Box::new(TreeWithTrunk {
                    tree: DroopingTree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 3,
                    }),
                }),
                14,
            ),
        ];
        let cols = combos.len();
        let col_w = width / cols;
        let ground_y = (height as i32) - 4;

        for (i, (label, drawer, default_bole)) in combos.iter().enumerate() {
            let cx = (i * col_w + col_w / 2) as i32;
            let color = palette[i % palette.len()];
            let bole_idx = bole_override.unwrap_or(*default_bole);

            let plot = Rect {
                x: (i * col_w + 1).min(width - 2),
                y: 2,
                w: (col_w - 2).max(4),
                h: (ground_y as usize).saturating_sub(2),
            };
            let params = TreeParams {
                plot,
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(i + 2) % palette.len()],
                fruit_factor,
                branch_factor,
                direction: GrowDir::Up,
                bole: Some(Bole { style: bole_idx }),
                taper: TaperKind::default(),
            };

            drawer.grow(&mut grid, &params, &mut rng);

            // Multi-line label at bottom
            for (li, line) in label.split('\n').enumerate() {
                let lx = (cx - line.len() as i32 / 2).max(0) as usize;
                let ly = height - 2 + li;
                for (j, ch) in line.chars().enumerate() {
                    if lx + j < width && ly < height {
                        grid[ly][lx + j] = Cell::new(ch, lighten(color, 40));
                    }
                }
            }
        }
    } else if mode == "trees2" {
        // trees2: squat horizontal boles (styles 18-23) + tree combos
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);
        let fruit_factor: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.2);
        let branch_factor: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.5);

        let combos: Vec<(&str, Box<dyn TreeDrawer>, usize)> = vec![
            (
                "Spiral\n+SqCrescent",
                Box::new(SpiralTree) as Box<dyn TreeDrawer>,
                18,
            ),
            (
                "Spiral+Wobble\n+SqBraille",
                Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(WobbleTrunk {
                        height_fraction: 0.6,
                    }),
                }),
                19,
            ),
            (
                "Candelabra\n+SqFrame",
                Box::new(CandelabraTree) as Box<dyn TreeDrawer>,
                20,
            ),
            (
                "Split+Sine\n+SqDiamond",
                Box::new(TreeWithTrunk {
                    tree: SplitTree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 2,
                    }),
                }),
                21,
            ),
            (
                "Birch\n+SqChevron",
                Box::new(BirchTree) as Box<dyn TreeDrawer>,
                22,
            ),
            (
                "Drooping\n+SqButtress",
                Box::new(DroopingTree) as Box<dyn TreeDrawer>,
                23,
            ),
            (
                "WavyBirch\n+SqCrescent",
                Box::new(WavyBirch) as Box<dyn TreeDrawer>,
                18,
            ),
        ];
        let cols = combos.len();
        let col_w = width / cols;
        let ground_y = (height as i32) - 4;

        for (i, (label, drawer, bole_idx)) in combos.iter().enumerate() {
            let cx = (i * col_w + col_w / 2) as i32;
            let color = palette[i % palette.len()];

            let plot = Rect {
                x: (i * col_w + 1).min(width - 2),
                y: 2,
                w: (col_w - 2).max(4),
                h: (ground_y as usize).saturating_sub(2),
            };
            let params = TreeParams {
                plot,
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(i + 2) % palette.len()],
                fruit_factor,
                branch_factor,
                direction: GrowDir::Up,
                bole: Some(Bole { style: *bole_idx }),
                taper: [
                    TaperKind::Diagonal,
                    TaperKind::Shelf,
                    TaperKind::Bracket,
                    TaperKind::Step,
                    TaperKind::Melt,
                    TaperKind::Shelf,
                    TaperKind::Bracket,
                ][i % 7],
            };

            drawer.grow(&mut grid, &params, &mut rng);

            for (li, line) in label.split('\n').enumerate() {
                let lx = (cx - line.len() as i32 / 2).max(0) as usize;
                let ly = height - 2 + li;
                for (j, ch) in line.chars().enumerate() {
                    if lx + j < width && ly < height {
                        grid[ly][lx + j] = Cell::new(ch, lighten(color, 40));
                    }
                }
            }
        }
    } else if mode == "trees3" {
        // trees3: vertical catalog -- all tree types, trunk algos, taper styles, bole styles
        let page_w = 80usize;
        let tree_h = 28usize;
        let label_h = 2usize;
        let section_gap = 2usize;
        let header_h = 2usize;

        // Section heights
        let sec1_h = header_h + 2 * (tree_h + label_h) + section_gap; // 8 tree types
        let sec2_h = header_h + tree_h + label_h + section_gap; // 7 trunk algos
        let sec3_h = header_h + tree_h + label_h + section_gap; // 5 taper styles
        let bole_tree_h = 20usize;
        let sec4_h = header_h + 3 * (bole_tree_h + label_h) + section_gap; // 24 bole styles

        let page_h = sec1_h + sec2_h + sec3_h + sec4_h + 4;
        let mut pg = vec![vec![Cell::blank(); page_w]; page_h];

        let energy = 0.8f32;

        let write_header = |pg: &mut Vec<Vec<Cell>>, y: usize, text: &str, color: Color| {
            let lx = (page_w / 2).saturating_sub(text.len() / 2);
            for (j, ch) in text.chars().enumerate() {
                if lx + j < page_w {
                    pg[y][lx + j] = Cell::new(ch, color);
                }
            }
            for x in 0..page_w {
                pg[y + 1][x] = Cell::new('─', darken(color, 30));
            }
        };

        let write_label =
            |pg: &mut Vec<Vec<Cell>>, row_y: usize, cx: i32, label: &str, color: Color| {
                let lx = (cx - label.len() as i32 / 2).max(0) as usize;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < page_w && row_y < pg.len() {
                        pg[row_y][lx + j] = Cell::new(ch, color);
                    }
                }
            };

        // ── Section 1: Tree Types ─────────────────────────────────
        let mut cy = 1usize;
        write_header(&mut pg, cy, "── TREE TYPES ──", palette[4]);
        cy += header_h;

        let tree_labels = [
            "Spiral",
            "Candelabra",
            "Split",
            "Birch",
            "WavyBirch",
            "Storm",
            "Dead",
            "Drooping",
        ];
        let cols8 = 4usize;
        let col_w8 = page_w / cols8;

        for idx in 0..8usize {
            let row = idx / cols8;
            let col = idx % cols8;
            let row_y = cy + row * (tree_h + label_h);
            let color = palette[idx % palette.len()];
            let cx_i = (col * col_w8 + col_w8 / 2) as i32;
            let params = TreeParams {
                plot: Rect {
                    x: col * col_w8 + 1,
                    y: row_y,
                    w: col_w8 - 2,
                    h: tree_h,
                },
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(idx + 2) % palette.len()],
                fruit_factor: 0.2,
                branch_factor: 0.5,
                direction: GrowDir::Up,
                bole: Some(Bole { style: idx % 8 }),
                taper: TaperKind::Bracket,
            };
            match idx {
                0 => SpiralTree.grow(&mut pg, &params, &mut rng),
                1 => CandelabraTree.grow(&mut pg, &params, &mut rng),
                2 => SplitTree.grow(&mut pg, &params, &mut rng),
                3 => BirchTree.grow(&mut pg, &params, &mut rng),
                4 => WavyBirch.grow(&mut pg, &params, &mut rng),
                5 => StormTree::new().grow(&mut pg, &params, &mut rng),
                6 => DeadTree.grow(&mut pg, &params, &mut rng),
                _ => DroopingTree.grow(&mut pg, &params, &mut rng),
            }
            write_label(
                &mut pg,
                row_y + tree_h,
                cx_i,
                tree_labels[idx],
                lighten(color, 40),
            );
        }
        cy += 2 * (tree_h + label_h) + section_gap;

        // ── Section 2: Trunk Algorithms ───────────────────────────
        write_header(&mut pg, cy, "── TRUNK ALGORITHMS ──", palette[4]);
        cy += header_h;

        let trunk_labels = [
            "Straight", "Thick", "Wobble", "Lean", "Gnarled", "Organic", "Sine",
        ];
        let cols7 = 7usize;
        let col_w7 = page_w / cols7;

        for i in 0..7usize {
            let color = palette[i % palette.len()];
            let cx_i = (i * col_w7 + col_w7 / 2) as i32;
            let params = TreeParams {
                plot: Rect {
                    x: i * col_w7 + 1,
                    y: cy,
                    w: col_w7 - 2,
                    h: tree_h,
                },
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(i + 2) % palette.len()],
                fruit_factor: 0.2,
                branch_factor: 0.5,
                direction: GrowDir::Up,
                bole: Some(Bole { style: 14 }),
                taper: TaperKind::default(),
            };
            let drawer: Box<dyn TreeDrawer> = match i {
                0 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(StraightTrunk {
                        height_fraction: 0.5,
                    }),
                }),
                1 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(ThickTrunk {
                        height_fraction: 0.5,
                    }),
                }),
                2 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(WobbleTrunk {
                        height_fraction: 0.5,
                    }),
                }),
                3 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(LeanTrunk::new()),
                }),
                4 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(GnarledTrunk),
                }),
                5 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(OrganicTrunk {
                        height_fraction: 0.5,
                    }),
                }),
                _ => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 2,
                    }),
                }),
            };
            drawer.grow(&mut pg, &params, &mut rng);
            write_label(
                &mut pg,
                cy + tree_h,
                cx_i,
                trunk_labels[i],
                lighten(color, 40),
            );
        }
        cy += tree_h + label_h + section_gap;

        // ── Section 3: Taper Styles ───────────────────────────────
        write_header(&mut pg, cy, "── TAPER STYLES ──", palette[4]);
        cy += header_h;

        let taper_data = [
            ("Diagonal", TaperKind::Diagonal),
            ("Shelf", TaperKind::Shelf),
            ("Bracket", TaperKind::Bracket),
            ("Step", TaperKind::Step),
            ("Melt", TaperKind::Melt),
        ];
        let cols5 = 5usize;
        let col_w5 = page_w / cols5;

        for (i, (label, taper)) in taper_data.iter().enumerate() {
            let color = palette[i % palette.len()];
            let cx_i = (i * col_w5 + col_w5 / 2) as i32;
            let params = TreeParams {
                plot: Rect {
                    x: i * col_w5 + 1,
                    y: cy,
                    w: col_w5 - 2,
                    h: tree_h,
                },
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(i + 2) % palette.len()],
                fruit_factor: 0.2,
                branch_factor: 0.5,
                direction: GrowDir::Up,
                bole: Some(Bole { style: 0 }),
                taper: *taper,
            };
            SpiralTree.grow(&mut pg, &params, &mut rng);
            write_label(&mut pg, cy + tree_h, cx_i, label, lighten(color, 40));
        }
        cy += tree_h + label_h + section_gap;

        // ── Section 4: Bole Styles ────────────────────────────────
        write_header(&mut pg, cy, "── BOLE STYLES ──", palette[4]);
        cy += header_h;

        let boles_per_row = 8usize;
        let bole_col_w = page_w / boles_per_row;

        for bole_i in 0..24usize {
            let row = bole_i / boles_per_row;
            let col = bole_i % boles_per_row;
            let row_y = cy + row * (bole_tree_h + label_h);
            let color = palette[bole_i % palette.len()];
            let cx_i = (col * bole_col_w + bole_col_w / 2) as i32;
            let label = format!("{}", bole_i);
            let params = TreeParams {
                plot: Rect {
                    x: col * bole_col_w + 1,
                    y: row_y,
                    w: bole_col_w - 2,
                    h: bole_tree_h,
                },
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(bole_i + 2) % palette.len()],
                fruit_factor: 0.2,
                branch_factor: 0.5,
                direction: GrowDir::Up,
                bole: Some(Bole { style: bole_i }),
                taper: TaperKind::Bracket,
            };
            SpiralTree.grow(&mut pg, &params, &mut rng);
            write_label(
                &mut pg,
                row_y + bole_tree_h,
                cx_i,
                &label,
                lighten(color, 40),
            );
        }

        emit_grid(&pg);
        return;
    } else if mode == "trees4" {
        // trees4: showcase all TreeDrawer types including new ports
        // One tree per slot, labeled, with boles and fruit
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);

        let all_trees: Vec<(&str, Box<dyn TreeDrawer>)> = vec![
            ("Spiral", Box::new(SpiralTree)),
            ("Candelabra", Box::new(CandelabraTree)),
            ("Split", Box::new(SplitTree)),
            ("Birch", Box::new(BirchTree)),
            ("WavyBirch", Box::new(WavyBirch)),
            ("Storm", Box::new(StormTree::new())),
            ("Dead", Box::new(DeadTree)),
            ("Drooping", Box::new(DroopingTree)),
            ("Pine", Box::new(PineTree)),
            ("Willow", Box::new(WillowTree)),
            ("Palm", Box::new(PalmTree)),
            ("Wide", Box::new(WideTree)),
            ("Asymmetric", Box::new(AsymmetricTree)),
            ("Kaiju", Box::new(KaijuTree)),
            ("Zigzag", Box::new(ZigzagTree)),
            ("BrailleCanopy", Box::new(BrailleCanopyTree)),
            ("Tendril", Box::new(TendrilTree)),
        ];

        let count = all_trees.len();
        let cols = 6usize;
        let rows = (count + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = 28usize; // tall cells like trees3
        let page_h = rows * cell_h + 2;
        let mut grid = vec![vec![Cell::blank(); width]; page_h];

        for (i, (label, drawer)) in all_trees.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let px = col * cell_w;
            let py = row * cell_h;
            let color = palette[i % palette.len()];

            let params = TreeParams {
                plot: Rect {
                    x: px + 1,
                    y: py + 1,
                    w: cell_w - 2,
                    h: cell_h - 3,
                },
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(i + 3) % palette.len()],
                fruit_factor: 0.3,
                branch_factor: 0.7,
                direction: GrowDir::Up,
                bole: Some(Bole { style: i }),
                taper: TaperKind::Bracket,
            };
            drawer.grow(&mut grid, &params, &mut rng);

            // Label
            let lx = px + cell_w / 2 - label.len() / 2;
            let ly = py + cell_h - 1;
            for (j, ch) in label.chars().enumerate() {
                if lx + j < width && ly < page_h {
                    grid[ly][lx + j] = Cell::new(ch, darken(color, 20));
                }
            }
        }

        emit_grid(&grid);
        return;
    } else if mode == "trees8" {
        // trees8: [energy] [fruit] [branch]
        // Three new TreeDrawers (Oak, Fountain, Windswept), each shown at
        // full and low energy with cycling boles and tapers.
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.85);
        let fruit: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.3);
        let branch: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.7);

        let drawers: Vec<(&str, Box<dyn TreeDrawer>)> = vec![
            ("Oak", Box::new(OakTree)),
            ("Fountain", Box::new(FountainTree)),
            ("Windswept", Box::new(WindsweptTree::new(&mut rng))),
        ];
        let tapers = [TaperKind::Bracket, TaperKind::Diagonal, TaperKind::Melt];

        let cols = drawers.len();
        let cell_w = width / cols;
        let cell_h = 24usize;
        let rows = 2usize;
        let page_h = rows * cell_h + 2;
        let mut pg = vec![vec![Cell::blank(); width]; page_h];

        for row in 0..rows {
            // top row full energy, bottom row scrub-sized
            let row_energy = if row == 0 { energy } else { energy * 0.6 };
            for (i, (label, drawer)) in drawers.iter().enumerate() {
                let px = i * cell_w;
                let py = row * cell_h;
                let color = palette[(i + row * 3) % palette.len()];

                let params = TreeParams {
                    plot: Rect {
                        x: px + 1,
                        y: py + 1,
                        w: cell_w - 2,
                        h: cell_h - 3,
                    },
                    energy: row_energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: lighten(color, 20),
                    tip_color: lighten(color, 40),
                    fruit_color: palette[(i + 3) % palette.len()],
                    fruit_factor: fruit,
                    branch_factor: branch,
                    direction: GrowDir::Up,
                    bole: Some(Bole { style: i * 2 + row }),
                    taper: tapers[(i + row) % tapers.len()],
                };
                drawer.grow(&mut pg, &params, &mut rng);

                let lx = px + cell_w / 2 - label.len() / 2;
                let ly = py + cell_h - 1;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < width && ly < page_h {
                        pg[ly][lx + j] = Cell::new(ch, darken(color, 20));
                    }
                }
            }
        }

        emit_grid(&pg);
        return;
    } else if mode == "trees9" {
        // trees9: [energy] [fruit] [branch]
        // Esoteric drawers (Fractal, L-System, Dragon, Helix) at two
        // energies, planted on the winding boles (24-27).
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.85);
        let fruit: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.25);
        let branch: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.7);

        let drawers: Vec<(&str, Box<dyn TreeDrawer>)> = vec![
            ("Fractal", Box::new(FractalTree)),
            ("L-System", Box::new(LSystemTree)),
            ("Dragon", Box::new(DragonTree)),
            ("Helix", Box::new(HelixTree)),
        ];
        let tapers = [TaperKind::Diagonal, TaperKind::Bracket, TaperKind::Shelf];

        let cols = drawers.len();
        let cell_w = width / cols;
        let cell_h = 24usize;
        let rows = 2usize;
        let page_h = rows * cell_h + 2;
        let mut pg = vec![vec![Cell::blank(); width]; page_h];

        for row in 0..rows {
            // top row full energy, bottom row scrub-sized
            let row_energy = if row == 0 { energy } else { energy * 0.6 };
            for (i, (label, drawer)) in drawers.iter().enumerate() {
                let px = i * cell_w;
                let py = row * cell_h;
                let color = palette[(i + row * 3) % palette.len()];

                let params = TreeParams {
                    plot: Rect {
                        x: px + 1,
                        y: py + 1,
                        w: cell_w - 2,
                        h: cell_h - 5,
                    },
                    energy: row_energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: lighten(color, 20),
                    tip_color: lighten(color, 40),
                    fruit_color: palette[(i + 3) % palette.len()],
                    fruit_factor: fruit,
                    branch_factor: branch,
                    direction: GrowDir::Up,
                    bole: Some(Bole {
                        style: 24 + (i + row) % 4,
                    }),
                    taper: tapers[(i + row) % tapers.len()],
                };
                drawer.grow(&mut pg, &params, &mut rng);

                let lx = px + cell_w / 2 - label.len() / 2;
                let ly = py + cell_h - 1;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < width && ly < page_h {
                        pg[ly][lx + j] = Cell::new(ch, darken(color, 20));
                    }
                }
            }
        }

        emit_grid(&pg);
        return;
    } else if mode == "bushes" {
        // bushes: showcase full-size bole patterns as standalone bush sprites
        // args: [energy]
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);

        // Styles 0-17 only (squat styles 18-23 are too minimal as standalone bushes)
        let styles: Vec<usize> = (0..18).collect();
        let cols = 6usize;
        let rows = 3usize;
        let cell_w = width / cols;
        let cell_h = height / rows;

        for (i, &style_idx) in styles.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let cx = (col * cell_w + cell_w / 2) as i32;
            let cy = (row * cell_h + cell_h * 3 / 4) as i32;
            let bush_w = (cell_w as i32 / 3).max(3);
            let color = palette[style_idx % palette.len()];

            // Rotate through fade directions
            let fade = match style_idx % 3 {
                0 => FadeDir::Down,
                1 => FadeDir::CenterOut,
                _ => FadeDir::Up,
            };
            // Ground: dark version of the palette for contrast
            let ground = darken(palette[(style_idx + 3) % palette.len()], 40);

            let bush = BushSprite {
                style: style_idx,
                x: cx,
                y: cy,
                width: bush_w,
                color,
                ground,
                fade,
                energy,
            };
            bush.draw(&mut grid, &mut rng);

            // Label
            let label = format!("{}", style_idx);
            let lx = (cx - label.len() as i32 / 2).max(0) as usize;
            let label_y = (row * cell_h + cell_h - 1).min(height - 1);
            for (j, ch) in label.chars().enumerate() {
                if lx + j < width {
                    grid[label_y][lx + j] = Cell::new(ch, darken(color, 20));
                }
            }
        }
    } else if mode == "forest7" {
        // forest7: production layered forest with boles, tapers, fruit
        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }

        // Per-column ground height
        let jitter_range = rng.random_range(2..6u32) as i32;
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1;
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground fill with hue gradient
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        let grad_dir = rng.random_range(0..6u32);
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else {
            120.0
        };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                let t = match grad_dir {
                    0 => x as f64 / width as f64,
                    1 => 1.0 - x as f64 / width as f64,
                    2 => depth as f64 / ground_depth as f64,
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0,
                    4 => {
                        ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0
                    }
                    _ => {
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // ── Scene walk placement ──────────────────────────────────────
        // Walk across the terrain, placing elements at each stop.
        // Element types: tree, bush, flower cluster, fruit vine, empty gap.
        let all_tapers = [
            TaperKind::Diagonal,
            TaperKind::Shelf,
            TaperKind::Bracket,
            TaperKind::Step,
            TaperKind::Melt,
        ];

        #[derive(Clone, Copy)]
        enum F7Element {
            Tree {
                kind: usize,
                spread: usize,
                tree_h: usize,
                bole_style: Option<usize>,
                taper: TaperKind,
            },
            Bush {
                style: usize,
                bush_w: i32,
            },
            Flowers,
            FruitVine,
        }

        struct F7Stop {
            x: usize,
            root_y: usize,
            hue: f64,
            layer: u8,
            element: F7Element,
        }

        let mut stops: Vec<F7Stop> = Vec::new();

        // Walk: start at random x, hop 8-20 cells each step, wrap around
        let stop_count = rng.random_range(12..22u32) as usize;
        let min_spacing = (width / (stop_count + 1)).max(6);
        let mut wx = rng.random_range(4..(width - 4) as u32) as usize;

        for si in 0..stop_count {
            // Hop forward with some jitter
            if si > 0 {
                let hop = rng.random_range(
                    min_spacing as u32..(min_spacing as u32 * 3).min(width as u32 / 2),
                );
                wx = (wx + hop as usize) % width;
                wx = wx.clamp(3, width - 4);
            }

            let grass_y = ground_heights[wx.min(width - 1)];
            // Layer assignment: first third bg, middle third mid, last third fg
            let layer = match si * 3 / stop_count {
                0 => 0u8,
                1 => 1,
                _ => 2,
            };
            let root_offset = match layer {
                0 => rng.random_range(0..2u32) as usize,
                1 => rng.random_range(1..5u32) as usize,
                _ => rng.random_range(2..7u32) as usize,
            };
            let root_y = (grass_y + root_offset).min(height - 2);

            // Pick element type: trees most common, bushes and flowers fill gaps
            let element = match rng.random_range(0..10u32) {
                0..=5 => {
                    let kind = rng.random_range(0..17u32) as usize;
                    let spread = match layer {
                        0 => rng.random_range(2..6u32) as usize,
                        1 => rng.random_range(5..14u32) as usize,
                        _ => rng.random_range(10..22u32) as usize,
                    };
                    let tree_h = match layer {
                        0 => rng.random_range(3..10u32) as usize,
                        1 => rng.random_range(10..25u32) as usize,
                        _ => rng.random_range(20..40u32.min(root_y.max(21) as u32)) as usize,
                    };
                    // ~40% of trees get a bole, rest go straight trunk into ground
                    let bole_style = if rng.random_range(0..10u32) < 4 {
                        Some(rng.random_range(0..10u32) as usize) // simpler styles only
                    } else {
                        None
                    };
                    F7Element::Tree {
                        kind,
                        spread,
                        tree_h,
                        bole_style,
                        taper: all_tapers[rng.random_range(0..all_tapers.len() as u32) as usize],
                    }
                }
                // 6..=7 => F7Element::Bush {
                //     style: rng.random_range(0..18u32) as usize,
                //     bush_w: rng.random_range(3..8u32) as i32,
                // },
                6..=7 => F7Element::Flowers,
                8 => F7Element::Flowers,
                _ => F7Element::FruitVine,
            };

            stops.push(F7Stop {
                x: wx,
                root_y,
                hue: rng.random_range(0..360u32) as f64,
                layer,
                element,
            });
        }

        // Sort back-to-front: bg (layer 0) first, then mid, then fg
        stops.sort_by(|a, b| a.layer.cmp(&b.layer).then(a.root_y.cmp(&b.root_y)));

        // ── Draw each stop ───────────────────────────────────────────
        for stop in &stops {
            let lightness = match stop.layer {
                0 => 0.15 + rng.random::<f64>() * 0.10,
                1 => 0.25 + rng.random::<f64>() * 0.10,
                _ => 0.35 + rng.random::<f64>() * 0.10,
            };
            let saturation = match stop.layer {
                0 => 0.30,
                1 => 0.45,
                _ => 0.60,
            };
            let energy = match stop.layer {
                0 => rng.random_range(30..55u32) as f32 / 100.0,
                1 => rng.random_range(60..85u32) as f32 / 100.0,
                _ => rng.random_range(85..100u32) as f32 / 100.0,
            };
            let color = hsl_to_rgb(stop.hue, saturation, lightness);

            match stop.element {
                F7Element::Tree {
                    kind,
                    spread,
                    tree_h,
                    bole_style,
                    taper,
                } => {
                    let canopy_y = stop.root_y.saturating_sub(tree_h).max(1);
                    let plot_w = spread * 2 + 6;
                    let plot = Rect {
                        x: stop.x.saturating_sub(plot_w / 2),
                        y: canopy_y,
                        w: plot_w.min(width),
                        h: stop.root_y.saturating_sub(canopy_y) + 2,
                    };
                    let tp = TreeParams {
                        plot,
                        energy,
                        trunk_color: color,
                        bark_color: darken(color, 15),
                        branch_color: color,
                        tip_color: lighten(color, 30),
                        fruit_color: shift_hue(color, 60.0),
                        fruit_factor: 0.3,
                        branch_factor: 0.8,
                        direction: GrowDir::Up,
                        bole: bole_style.map(|s| Bole { style: s }),
                        taper,
                    };
                    match kind % 17 {
                        0 => SpiralTree.grow(&mut grid, &tp, &mut rng),
                        1 => CandelabraTree.grow(&mut grid, &tp, &mut rng),
                        2 => SplitTree.grow(&mut grid, &tp, &mut rng),
                        3 => BirchTree.grow(&mut grid, &tp, &mut rng),
                        4 => WavyBirch.grow(&mut grid, &tp, &mut rng),
                        5 => StormTree::new().grow(&mut grid, &tp, &mut rng),
                        6 => DeadTree.grow(&mut grid, &tp, &mut rng),
                        7 => DroopingTree.grow(&mut grid, &tp, &mut rng),
                        8 => PineTree.grow(&mut grid, &tp, &mut rng),
                        9 => WillowTree.grow(&mut grid, &tp, &mut rng),
                        10 => PalmTree.grow(&mut grid, &tp, &mut rng),
                        11 => WideTree.grow(&mut grid, &tp, &mut rng),
                        12 => AsymmetricTree.grow(&mut grid, &tp, &mut rng),
                        13 => KaijuTree.grow(&mut grid, &tp, &mut rng),
                        14 => ZigzagTree.grow(&mut grid, &tp, &mut rng),
                        15 => BrailleCanopyTree.grow(&mut grid, &tp, &mut rng),
                        16 => TendrilTree.grow(&mut grid, &tp, &mut rng),
                        _ => SpiralTree.grow(&mut grid, &tp, &mut rng),
                    }
                }
                F7Element::Bush { style, bush_w } => {
                    let fade = match rng.random_range(0..3u32) {
                        0 => FadeDir::Down,
                        1 => FadeDir::CenterOut,
                        _ => FadeDir::Up,
                    };
                    let bush = BushSprite {
                        style,
                        x: stop.x as i32,
                        y: stop.root_y as i32,
                        width: bush_w,
                        color,
                        ground: color, // no fade -- preserve ground colors
                        fade,
                        energy,
                    };
                    bush.draw(&mut grid, &mut rng);
                }
                F7Element::Flowers => {
                    let burst = rng.random_range(3..7u32);
                    for _ in 0..burst {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..8u32) as f32;
                        let fx = (stop.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = stop.root_y as i32 + rng.random_range(0..3u32) as i32;
                        if fx >= 1
                            && fy >= 1
                            && (fx as usize) < width - 1
                            && (fy as usize) < height - 1
                        {
                            grow_flower_spiral(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                color,
                                &mut rng,
                            );
                        }
                    }
                }
                F7Element::FruitVine => {
                    let burst = rng.random_range(2..5u32);
                    for _ in 0..burst {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..6u32) as f32;
                        let fx = (stop.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = stop.root_y as i32 + rng.random_range(0..2u32) as i32;
                        if fx >= 1
                            && fy >= 1
                            && (fx as usize) < width - 1
                            && (fy as usize) < height - 1
                        {
                            let c = shift_hue(color, rng.random_range(20..80u32) as f64);
                            grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng);
                        }
                    }
                    // Braille fruit dots near the vines
                    for _ in 0..rng.random_range(1..4u32) {
                        let fx = stop.x as i32 + rng.random_range(-4..5i32);
                        let fy = stop.root_y as i32 + rng.random_range(-2..3i32);
                        if fx >= 0 && fy >= 0 && (fx as usize) < width && (fy as usize) < height {
                            let fruit_c = shift_hue(color, 60.0);
                            draw_fruit(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                rng.random_range(0..5),
                                fruit_c,
                            );
                        }
                    }
                }
            }
        }

        // Braille leaf clusters on branch tips
        let leaf_hue = rng.random_range(60..180u32) as f64;
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 45, &mut rng);

        // Extra ground-level flower/fruit scatter near tree stops
        for stop in &stops {
            if stop.layer == 0 {
                continue;
            }
            if let F7Element::Tree { .. } = stop.element {
                let burst = rng.random_range(0..3u32);
                for _ in 0..burst {
                    let angle = rng.random::<f32>() * std::f32::consts::TAU;
                    let radius = rng.random_range(1..6u32) as f32;
                    let fx = (stop.x as f32 + angle.cos() * radius * 1.5) as i32;
                    let fy = stop.root_y as i32 + rng.random_range(1..3u32) as i32;
                    if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                    {
                        let c = palette[rng.random_range(2..5)];
                        match rng.random_range(0..3u32) {
                            0 => {
                                grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng)
                            }
                            1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                            _ => draw_flower(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                rng.random_range(0..5),
                                c,
                            ),
                        }
                    }
                }
            }
        }
    } else if mode == "forest8" {
        // forest8 [layers=0] [density=0] -- high-entropy scene-walk forest: trees, bushes, flowers, fruit, grass
        let layers_arg: u8 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let density_arg: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let layer_count = if layers_arg == 0 {
            (3 + seed % 2) as u8
        } else {
            layers_arg.clamp(2, 5)
        };
        let density = if density_arg == 0.0 {
            0.4
        } else {
            density_arg.clamp(0.2, 1.0)
        };

        let opts = SceneOpts {
            layer_count,
            density,
            tree_rate: 0.62,
            bole_rate: 0.4,
            ground_frac: 0.42,
            kind_filter: None,
            vines: true,
            hue_range: 35.0,
        };
        let (ground_y, stops) = scene_walk(width, height, &mut rng, &opts);

        // sky
        let sky_color = darken(palette[0], 90);
        for y in 0..ground_y {
            for x in 0..width {
                if rng.random_range(0..20u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        let cloud_color = lighten(palette[0], 12);
        for _ in 0..rng.random_range(0..3u32) {
            let cx = rng.random_range(4..(width - 4) as u32) as usize;
            let cy = rng.random_range(2..(ground_y / 3).max(3) as u32) as usize;
            draw_cloud(
                &mut grid,
                cx,
                cy,
                rng.random_range(8..16u32) as usize,
                cloud_color,
                &mut rng,
            );
        }

        // ground with depth hue gradient + grass
        let ground_chars = ['╱', '╲', '·', '∿', '~', '˜'];
        let grass_chars = ['"', '"', '\'', '·', '˙', '‚'];
        let gdepth = (height - ground_y).max(1);
        for x in 0..width {
            for y in ground_y..height {
                let depth = y - ground_y;
                let t = depth as f64 / gdepth as f64;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                grid[y][x] = Cell::new(ch, hsl_to_rgb(110.0, 0.35, (0.22 - t * 0.005).max(0.08)));
            }
        }
        // grass tufts, denser near the horizon
        for _ in 0..(width as u32) {
            let gx = rng.random_range(0..width as u32) as usize;
            let gy = ground_y + rng.random_range(0..(gdepth / 3).max(1) as u32) as usize;
            if gy < height {
                grid[gy][gx] = Cell::new(
                    grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                    lighten(palette[2], 14),
                );
            }
        }

        // draw stops back-to-front: trees, bushes, flowers, fruit vines, grass tufts
        for s in &stops {
            let color = hsl_to_rgb(s.hue, s.sat as f64, s.light as f64);
            match s.el {
                SceneEl::Tree {
                    kind,
                    energy,
                    spread,
                    tree_h,
                    bole,
                    taper,
                } => {
                    let canopy_y = s.root_y.saturating_sub(tree_h).max(1);
                    let plot_w = (spread * 2 + 6).min(width);
                    let plot = Rect {
                        x: s.x.saturating_sub(plot_w / 2),
                        y: canopy_y,
                        w: plot_w,
                        h: s.root_y.saturating_sub(canopy_y) + 2,
                    };
                    let tp = TreeParams {
                        plot,
                        energy,
                        trunk_color: color,
                        bark_color: darken(color, 18),
                        branch_color: color,
                        tip_color: lighten(color, 30),
                        fruit_color: shift_hue(color, 60.0),
                        fruit_factor: 0.3,
                        branch_factor: 0.8,
                        direction: GrowDir::Up,
                        bole,
                        taper,
                    };
                    grow_tree_by_index(kind, &mut grid, &tp, &mut rng);
                }
                SceneEl::Bush {
                    style,
                    bush_w,
                    fade,
                } => {
                    let fadedir = match fade % 3 {
                        0 => FadeDir::Down,
                        1 => FadeDir::CenterOut,
                        _ => FadeDir::Up,
                    };
                    BushSprite {
                        style,
                        x: s.x as i32,
                        y: s.root_y as i32,
                        width: bush_w,
                        color,
                        ground: color,
                        fade: fadedir,
                        energy: 0.7,
                    }
                    .draw(&mut grid, &mut rng);
                }
                SceneEl::Flowers => {
                    for _ in 0..rng.random_range(3..7u32) {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..8u32) as f32;
                        let fx = (s.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = s.root_y as i32 + rng.random_range(0..3u32) as i32;
                        if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                        {
                            grow_flower_spiral(&mut grid, fx as usize, fy as usize, color, &mut rng);
                        }
                    }
                }
                SceneEl::FruitVine => {
                    for _ in 0..rng.random_range(2..5u32) {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..6u32) as f32;
                        let fx = (s.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = s.root_y as i32 + rng.random_range(0..2u32) as i32;
                        if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                        {
                            grow_fruit_vine(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                shift_hue(color, rng.random_range(20..80u32) as f64),
                                &mut rng,
                            );
                        }
                    }
                    for _ in 0..rng.random_range(1..4u32) {
                        let fx = s.x as i32 + rng.random_range(-4..5i32);
                        let fy = s.root_y as i32 + rng.random_range(-2..3i32);
                        if fx >= 0 && fy >= 0 && (fx as usize) < width && (fy as usize) < height {
                            draw_fruit(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                rng.random_range(0..5),
                                shift_hue(color, 60.0),
                            );
                        }
                    }
                }
                SceneEl::Grass => {
                    for _ in 0..rng.random_range(2..6u32) {
                        let gx = s.x as i32 + rng.random_range(-3..4i32);
                        let gy = s.root_y as i32 + rng.random_range(0..2i32);
                        if gx >= 0 && gy >= 0 && (gx as usize) < width && (gy as usize) < height {
                            grid[gy as usize][gx as usize] = Cell::new(
                                grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                                lighten(color, 18),
                            );
                        }
                    }
                }
                SceneEl::Gap => {}
            }
        }

        // braille leaf clusters on branch tips
        let leaf_color = hsl_to_rgb(rng.random_range(60..180u32) as f64, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 15, &mut rng);

        // ground-level flower/fruit scatter near mid + front trees
        for s in &stops {
            if s.layer == 0 {
                continue;
            }
            if let SceneEl::Tree { .. } = s.el {
                for _ in 0..rng.random_range(0..3u32) {
                    let angle = rng.random::<f32>() * std::f32::consts::TAU;
                    let radius = rng.random_range(1..6u32) as f32;
                    let fx = (s.x as f32 + angle.cos() * radius * 1.5) as i32;
                    let fy = s.root_y as i32 + rng.random_range(1..3u32) as i32;
                    if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                    {
                        let c = palette[rng.random_range(2..5)];
                        match rng.random_range(0..3u32) {
                            0 => grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng),
                            1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                            _ => draw_flower(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                rng.random_range(0..5),
                                c,
                            ),
                        }
                    }
                }
            }
        }
    } else if mode == "forest9" {
        // forest9 [layers=0] [fog=0] -- misty high-entropy forest with fog drifts
        let layers_arg: u8 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let fog_arg: u64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let layer_count = if layers_arg == 0 {
            (4 + seed % 2) as u8
        } else {
            layers_arg.clamp(3, 6)
        };
        let fog = if fog_arg == 0 {
            6 + seed % 5
        } else {
            fog_arg.clamp(0, 16)
        };

        let opts = SceneOpts {
            layer_count,
            density: 0.42,
            tree_rate: 0.58,
            bole_rate: 0.5,
            ground_frac: 0.5,
            kind_filter: None,
            vines: true,
            hue_range: 30.0,
        };
        let (ground_y, stops) = scene_walk(width, height, &mut rng, &opts);

        // dusky sky
        let sky_color = darken(palette[0], 70);
        for y in 0..ground_y {
            for x in 0..width {
                if rng.random_range(0..24u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }

        // muted ground + grass
        let ground_chars = ['·', '·', '∼', '˜', '╱'];
        let grass_chars = ['"', '\'', '·', '˙', '‚'];
        let gdepth = (height - ground_y).max(1);
        for x in 0..width {
            for y in ground_y..height {
                let depth = y - ground_y;
                let t = depth as f64 / gdepth as f64;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                grid[y][x] = Cell::new(ch, hsl_to_rgb(95.0, 0.18, (0.16 - t * 0.004).max(0.07)));
            }
        }
        for _ in 0..(width as u32 / 2) {
            let gx = rng.random_range(0..width as u32) as usize;
            let gy = ground_y + rng.random_range(0..(gdepth / 3).max(1) as u32) as usize;
            if gy < height {
                grid[gy][gx] = Cell::new(
                    grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                    lighten(palette[2], 8),
                );
            }
        }

        let mist_color = lighten(palette[0], 6);
        // draw stops back-to-front, colors desaturated for the misty look
        for s in &stops {
            let color = hsl_to_rgb(s.hue, (s.sat * 0.5) as f64, (s.light * 0.9) as f64);
            match s.el {
                SceneEl::Tree {
                    kind,
                    energy,
                    spread,
                    tree_h,
                    bole,
                    taper,
                } => {
                    let canopy_y = s.root_y.saturating_sub(tree_h).max(1);
                    let plot_w = (spread * 2 + 6).min(width);
                    let plot = Rect {
                        x: s.x.saturating_sub(plot_w / 2),
                        y: canopy_y,
                        w: plot_w,
                        h: s.root_y.saturating_sub(canopy_y) + 2,
                    };
                    let tp = TreeParams {
                        plot,
                        energy,
                        trunk_color: color,
                        bark_color: darken(color, 22),
                        branch_color: color,
                        tip_color: lighten(color, 24),
                        fruit_color: shift_hue(color, 40.0),
                        fruit_factor: 0.15,
                        branch_factor: 0.85,
                        direction: GrowDir::Up,
                        bole,
                        taper,
                    };
                    grow_tree_by_index(kind, &mut grid, &tp, &mut rng);
                }
                SceneEl::Bush {
                    style,
                    bush_w,
                    fade,
                } => {
                    let fadedir = match fade % 3 {
                        0 => FadeDir::Down,
                        1 => FadeDir::CenterOut,
                        _ => FadeDir::Up,
                    };
                    BushSprite {
                        style,
                        x: s.x as i32,
                        y: s.root_y as i32,
                        width: bush_w,
                        color,
                        ground: color,
                        fade: fadedir,
                        energy: 0.6,
                    }
                    .draw(&mut grid, &mut rng);
                }
                SceneEl::Flowers => {
                    for _ in 0..rng.random_range(2..6u32) {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..7u32) as f32;
                        let fx = (s.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = s.root_y as i32 + rng.random_range(0..3u32) as i32;
                        if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                        {
                            grow_flower_spiral(&mut grid, fx as usize, fy as usize, color, &mut rng);
                        }
                    }
                }
                SceneEl::FruitVine => {
                    for _ in 0..rng.random_range(2..4u32) {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..6u32) as f32;
                        let fx = (s.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = s.root_y as i32 + rng.random_range(0..2u32) as i32;
                        if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                        {
                            grow_fruit_vine(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                shift_hue(color, rng.random_range(20..80u32) as f64),
                                &mut rng,
                            );
                        }
                    }
                }
                SceneEl::Grass => {
                    for _ in 0..rng.random_range(2..5u32) {
                        let gx = s.x as i32 + rng.random_range(-3..4i32);
                        let gy = s.root_y as i32 + rng.random_range(0..2i32);
                        if gx >= 0 && gy >= 0 && (gx as usize) < width && (gy as usize) < height {
                            grid[gy as usize][gx as usize] = Cell::new(
                                grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                                lighten(color, 12),
                            );
                        }
                    }
                }
                SceneEl::Gap => {}
            }
        }

        // dim braille leaf clusters
        let leaf_color = hsl_to_rgb(rng.random_range(80..160u32) as f64, 0.3, 0.26);
        sprout_leaves(&mut grid, leaf_color, 12, &mut rng);

        // mist veil across the canopy region
        for _ in 0..(width as u32 / 3) {
            let fx = rng.random_range(0..width as u32) as usize;
            let fy = rng.random_range(2..ground_y as u32) as usize;
            if rng.random_range(0..2u32) == 0 {
                grid[fy][fx] = Cell::new('░', mist_color);
            }
        }

        // drifting fog streaks
        for _ in 0..fog {
            let fy = rng.random_range(2..ground_y as u32) as usize;
            let fx0 = rng.random_range(0..width as u32) as usize;
            let len = rng.random_range(10..30u32) as usize;
            for dx in 0..len {
                let fx = (fx0 + dx) % width;
                if rng.random_range(0..2u32) == 0 {
                    grid[fy][fx] = Cell::new('░', mist_color);
                }
            }
        }
    } else if mode == "boles6" {
        // boles6 [layers=0] -- close-packed bole forest, every trunk rooted in a bole
        let layers_arg: u8 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let layer_count = if layers_arg == 0 {
            (3 + seed % 2) as u8
        } else {
            layers_arg.clamp(2, 5)
        };
        let strong: &'static [usize] = &[0, 1, 3, 8, 6, 17, 22, 23];
        let opts = PackOpts {
            layer_count,
            overlap: 0.18,
            bole_rate: 1.0,
            ground_frac: 0.4,
            kind_filter: Some(strong),
            ..Default::default()
        };
        let (ground_y, slots) = pack_forest(width, height, &mut rng, &opts);

        // faint sky
        let sky_color = darken(palette[0], 85);
        for y in 0..ground_y {
            for x in 0..width {
                if rng.random_range(0..22u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // dim ground
        for x in 0..width {
            for y in ground_y..height {
                let depth = y - ground_y;
                grid[y][x] =
                    Cell::new('·', hsl_to_rgb(30.0, 0.25, (0.18 - depth as f64 * 0.004).max(0.08)));
            }
        }

        let lf_denom = (layer_count - 1).max(1) as f64;
        for s in &slots {
            let lfrac = s.layer as f64 / lf_denom;
            let color = hsl_to_rgb(s.hue, 0.40 + lfrac * 0.20, 0.16 + lfrac * 0.16);
            let tp = TreeParams {
                plot: s.plot,
                energy: s.energy,
                trunk_color: lighten(color, 8),
                bark_color: darken(color, 12),
                branch_color: color,
                tip_color: lighten(color, 26),
                fruit_color: shift_hue(color, 50.0),
                fruit_factor: 0.1,
                branch_factor: 0.7,
                direction: GrowDir::Up,
                bole: s.bole,
                taper: s.taper,
            };
            grow_tree_by_index(s.kind, &mut grid, &tp, &mut rng);
        }
    } else if mode == "trees10" {
        // trees10 [count=0] -- specimen row, every archetype side by side
        let count_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let count = if count_arg == 0 {
            (width / 7).max(8)
        } else {
            count_arg.clamp(4, 48)
        };
        let ground_y = (height as f32 * 0.82) as usize;
        for x in 0..width {
            for y in ground_y..height {
                grid[y][x] = Cell::new('·', darken(palette[1], 60));
            }
        }
        let slot = (width / count).max(6);
        let tapers = [
            TaperKind::Diagonal,
            TaperKind::Shelf,
            TaperKind::Bracket,
            TaperKind::Step,
            TaperKind::Melt,
        ];
        for i in 0..count {
            let kind = i % TREE_KIND_COUNT;
            let cx = i * slot + slot / 2;
            let plot_w = slot;
            let canopy_top = 2usize;
            let plot = Rect {
                x: cx.saturating_sub(plot_w / 2),
                y: canopy_top,
                w: plot_w,
                h: ground_y.saturating_sub(canopy_top) + 1,
            };
            let hue = (i as f64 * (360.0 / count as f64)) % 360.0;
            let color = hsl_to_rgb(hue, 0.55, 0.32);
            let tp = TreeParams {
                plot,
                energy: 0.92,
                trunk_color: color,
                bark_color: darken(color, 16),
                branch_color: color,
                tip_color: lighten(color, 30),
                fruit_color: shift_hue(color, 60.0),
                fruit_factor: 0.3,
                branch_factor: 0.85,
                direction: GrowDir::Up,
                bole: if i % 2 == 0 {
                    Some(Bole { style: i % 10 })
                } else {
                    None
                },
                taper: tapers[i % tapers.len()],
            };
            grow_tree_by_index(kind, &mut grid, &tp, &mut rng);
        }
    } else if mode == "mondrian2" {
        let line_w = 2;

        let fill_colors = if theme_name.is_empty() {
            let (fills, _) = mondrian_colors();
            fills
        } else {
            [
                lighten(palette[0], 40),
                palette[1],
                palette[2],
                palette[3],
                lighten(palette[0], 40),
            ]
        };
        let line_color = if theme_name.is_empty() {
            rgb(20, 20, 20)
        } else {
            darken(palette[0], 60)
        };

        // Layout mondrian grid with no content blocks -- all leaves are empty
        let rects = layout_mondrian(
            &mut grid,
            &[],
            0,
            line_w,
            12,
            5,
            line_color,
            line_color,
            &fill_colors,
            line_color,
            &mut rng,
        );

        // Fill each leaf with something unexpected
        for rect in &rects {
            let inset = Rect {
                x: rect.x + 1,
                y: rect.y + 1,
                w: rect.w.saturating_sub(2),
                h: rect.h.saturating_sub(2),
            };
            if inset.w < 3 || inset.h < 3 {
                continue;
            }

            match rng.random_range(0..7u32) {
                0..=1 => {
                    // Tree centered in the rect
                    let tx = inset.x + inset.w / 2;
                    let canopy = inset.y + 1;
                    let root = inset.y + inset.h - 1;
                    let spread = (inset.w / 3).max(2);
                    let color = palette[rng.random_range(1..4)];
                    // Clear to blank first
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y < height && x < width {
                                grid[y][x] = Cell::blank();
                            }
                        }
                    }
                    draw_tree(
                        &mut grid,
                        tx,
                        root,
                        canopy,
                        spread,
                        rng.random_range(0..12),
                        color,
                        &mut rng,
                    );
                }
                2 => {
                    // Flower garden -- clear + scatter flowers
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y < height && x < width {
                                grid[y][x] = Cell::blank();
                            }
                        }
                    }
                    let cx = inset.x + inset.w / 2;
                    let cy = inset.y + inset.h / 2;
                    draw_flower(&mut grid, cx, cy, rng.random_range(0..5), palette[3]);
                    let count = rng.random_range(2..6u32);
                    for _ in 0..count {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let r =
                            rng.random_range(2..((inset.w.min(inset.h) / 2).max(3)) as u32) as f32;
                        let fx = (cx as f32 + angle.cos() * r * 1.5) as usize;
                        let fy = (cy as f32 + angle.sin() * r * 0.7) as usize;
                        if fx > inset.x
                            && fx < inset.x + inset.w - 1
                            && fy > inset.y
                            && fy < inset.y + inset.h - 1
                        {
                            draw_flower(
                                &mut grid,
                                fx,
                                fy,
                                rng.random_range(0..5),
                                palette[rng.random_range(2..4)],
                            );
                        }
                    }
                }
                3 => {
                    // Rain in this cell only
                    let rain_color = darken(palette[2], 40);
                    let rain_chars = ['│', '┊', '╎', '┆'];
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y >= height || x >= width {
                                continue;
                            }
                            if grid[y][x].ch != ' ' {
                                continue;
                            }
                            if rng.random::<f32>() > 0.12 {
                                continue;
                            }
                            let streak = ((x * 7 + 13) % 11) < 3;
                            if !streak && rng.random::<f32>() > 0.3 {
                                continue;
                            }
                            let ch = rain_chars[rng.random_range(0..rain_chars.len())];
                            grid[y][x] = Cell::new(ch, darken(rain_color, rng.random_range(0..20)));
                        }
                    }
                }
                4 => {
                    // Fruit still life
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y < height && x < width {
                                grid[y][x] = Cell::blank();
                            }
                        }
                    }
                    let count = rng.random_range(2..5u32);
                    for _ in 0..count {
                        let fx = inset.x
                            + rng.random_range(2..inset.w.saturating_sub(2).max(3) as u32) as usize;
                        let fy = inset.y
                            + rng.random_range(1..inset.h.saturating_sub(2).max(2) as u32) as usize;
                        draw_fruit(
                            &mut grid,
                            fx,
                            fy,
                            rng.random_range(0..5),
                            palette[rng.random_range(1..4)],
                        );
                    }
                }
                5 => {
                    // Stars / night sky in this cell
                    let star_color = lighten(palette[4], 20);
                    let star_chars = ['·', '∙', '°', '*', '⋅', '✦'];
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y >= height || x >= width {
                                continue;
                            }
                            if grid[y][x].ch != ' ' {
                                continue;
                            }
                            if rng.random::<f32>() > 0.06 {
                                continue;
                            }
                            let ch = star_chars[rng.random_range(0..star_chars.len())];
                            grid[y][x] = Cell::new(ch, darken(star_color, rng.random_range(0..40)));
                        }
                    }
                }
                _ => {
                    // Leave as flat color fill (original mondrian behavior)
                }
            }
        }
    } else if mode == "kintsugi" {
        // kintsugi [cracks] -- shattered tile shards repaired with gold seams
        let crack_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
        let crack_count = crack_count.clamp(1, 12);

        // Each crack is a top-to-bottom polyline: one x per row, drifting with momentum.
        let mut cracks: Vec<Vec<i32>> = Vec::new();
        for i in 0..crack_count {
            let band = (width / (crack_count + 1)).max(2) as i32;
            let mut x = (i as i32 + 1) * band + rng.random_range(-band / 3..=band / 3);
            let mut drift: i32 = 0;
            let mut path = Vec::with_capacity(height);
            for _ in 0..height {
                path.push(x);
                if rng.random::<f32>() < 0.4 {
                    drift = rng.random_range(-1..=1);
                }
                x = (x + drift).clamp(1, width as i32 - 2);
            }
            cracks.push(path);
        }

        // Region id per cell = number of cracks left of it. Each shard gets its
        // own tile pattern and shade so the pieces read as separate pottery.
        let mut shard_tiles: Vec<TilePattern> = Vec::new();
        let mut shard_shade: Vec<u8> = Vec::new();
        for _ in 0..=crack_count {
            let v = tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT));
            shard_tiles.push(make_tile(v));
            shard_shade.push(rng.random_range(40..90));
        }
        for y in 0..height {
            for x in 0..width {
                let region = cracks.iter().filter(|c| c[y] < x as i32).count();
                let (ch, ci) = shard_tiles[region].at(x, y);
                if ch == ' ' {
                    continue;
                }
                let base = if ci == 0 { palette[1] } else { palette[2] };
                grid[y][x] = Cell::new(ch, darken(base, shard_shade[region]));
            }
        }

        // Gold seams over the top: slope-matched glyphs plus hairline branches.
        let gold = lighten(palette[3], 25);
        for path in &cracks {
            for y in 0..height {
                let x = path[y];
                let next = if y + 1 < height { path[y + 1] } else { x };
                let ch = if next > x {
                    '╲'
                } else if next < x {
                    '╱'
                } else {
                    '│'
                };
                if x >= 0 && (x as usize) < width {
                    grid[y][x as usize] = Cell::new(ch, gold);
                }
                if rng.random::<f32>() < 0.08 {
                    let dir: i32 = if rng.random::<f32>() < 0.5 { -1 } else { 1 };
                    let len = rng.random_range(2..5i32);
                    for k in 1..=len {
                        let bx = x + dir * k;
                        let by = y + k as usize;
                        if bx >= 0 && (bx as usize) < width && by < height {
                            let bch = if dir > 0 { '╲' } else { '╱' };
                            grid[by][bx as usize] = Cell::new(bch, darken(gold, 20));
                        }
                    }
                }
            }
        }
    } else if mode == "constellation" {
        // constellation [count] -- night sky with named, line-connected clusters
        let count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
        let count = count.clamp(1, 8);

        let field = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &field,
            NoiseVariant::Dot,
            darken(palette[2], 90),
            darken(palette[2], 70),
            &mut rng,
        );

        let syllables = [
            "vel", "ara", "cyg", "lyr", "tau", "rho", "nix", "ori", "eka", "sol",
        ];

        for _ in 0..count {
            let pad_x = (width / 8).max(1);
            let pad_y = (height / 6).max(1);
            let cx = rng.random_range(pad_x..(width - pad_x).max(pad_x + 1)) as i32;
            let cy = rng.random_range(pad_y..(height - pad_y).max(pad_y + 1)) as i32;
            let star_n = rng.random_range(4..8);
            let rx = rng.random_range(6..14i32);
            let ry = rng.random_range(2..5i32);

            let mut stars: Vec<(i32, i32)> = Vec::new();
            for _ in 0..star_n {
                let sx = (cx + rng.random_range(-rx..=rx)).clamp(0, width as i32 - 1);
                let sy = (cy + rng.random_range(-ry..=ry)).clamp(0, height as i32 - 2);
                if !stars.contains(&(sx, sy)) {
                    stars.push((sx, sy));
                }
            }
            // chain left-to-right so the figure doesn't crisscross itself
            stars.sort();

            let line_color = darken(palette[1], 40);
            for w in stars.windows(2) {
                let (x0, y0) = w[0];
                let (x1, y1) = w[1];
                let dx = x1 - x0;
                let dy = y1 - y0;
                let steps = dx.abs().max(dy.abs());
                for s in 1..steps {
                    let t = s as f32 / steps as f32;
                    let lx = (x0 as f32 + dx as f32 * t).round() as i32;
                    let ly = (y0 as f32 + dy as f32 * t).round() as i32;
                    if lx < 0 || ly < 0 || lx >= width as i32 || ly >= height as i32 {
                        continue;
                    }
                    let ch = if dy == 0 {
                        '─'
                    } else if dx == 0 {
                        '│'
                    } else if (dx > 0) == (dy > 0) {
                        '╲'
                    } else {
                        '╱'
                    };
                    grid[ly as usize][lx as usize] = Cell::new(ch, line_color);
                }
            }

            let star_chars = ['✦', '✧', '*', '◆'];
            for (si, &(sx, sy)) in stars.iter().enumerate() {
                let ch = star_chars[si % star_chars.len()];
                let c = if si == 0 {
                    lighten(palette[4], 20)
                } else {
                    palette[3]
                };
                grid[sy as usize][sx as usize] = Cell::new(ch, c);
            }

            let name = format!(
                "{}{}",
                syllables[rng.random_range(0..syllables.len())],
                syllables[rng.random_range(0..syllables.len())]
            );
            let ly = ((cy + ry + 1) as usize).min(height - 1);
            let lx = (cx as usize).saturating_sub(name.len() / 2);
            for (j, ch) in name.chars().enumerate() {
                if lx + j < width {
                    grid[ly][lx + j] = Cell::new(ch, darken(palette[4], 30));
                }
            }
        }
    } else if mode == "strata" {
        // strata [layers] -- geological cross-section with fossils
        let layer_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(6);
        let layer_count = layer_count.clamp(2, 10);

        // stacked contour boundaries, forced monotonic per column
        let band_h = (height / (layer_count + 1)).max(2);
        let mut bounds: Vec<Vec<usize>> = Vec::new();
        for i in 0..layer_count {
            let base = band_h * (i + 1);
            let amp = (band_h / 2).max(1);
            let mut c = gen_contour(width, base, amp, 0.55, &mut rng);
            if let Some(prev) = bounds.last() {
                for x in 0..width {
                    if c[x] <= prev[x] {
                        c[x] = prev[x] + 1;
                    }
                }
            }
            for x in 0..width {
                c[x] = c[x].min(height - 1);
            }
            bounds.push(c);
        }

        // fill each band with its own sediment texture, darker with depth
        let glyph_pools: [&[char]; 6] = [
            &['·', '∙', ' ', ' ', ' '],
            &['─', '─', '·', ' ', ' '],
            &['╱', '╲', ' ', ' '],
            &['░', '░', '·', ' ', ' '],
            &['~', '─', ' ', ' '],
            &['▪', '·', ' ', ' ', ' ', ' '],
        ];
        for li in 0..layer_count {
            let pool = glyph_pools[rng.random_range(0..glyph_pools.len())];
            let shade = (li * 90 / layer_count) as u8;
            let c1 = darken(palette[1 + li % 3], shade);
            for x in 0..width {
                let top = bounds[li][x] + 1;
                let bot = if li + 1 < layer_count {
                    bounds[li + 1][x]
                } else {
                    height
                };
                for y in top..bot.min(height) {
                    let ch = pool[rng.random_range(0..pool.len())];
                    if ch == ' ' {
                        continue;
                    }
                    grid[y][x] = Cell::new(ch, c1);
                }
            }
        }

        // boundary ridges on top of the fills
        let full = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        for (li, c) in bounds.iter().enumerate() {
            let ridge = darken(lighten(palette[2], 20), (li * 60 / layer_count) as u8);
            draw_contour_ridge(&mut grid, &full, c, ridge);
        }

        // fossils embedded in the deeper bands
        let fossil_count = rng.random_range(2..5);
        for _ in 0..fossil_count {
            let fx = rng.random_range(4..width.saturating_sub(4).max(5));
            let floor = bounds[(layer_count / 2).min(layer_count - 1)][fx];
            if floor + 3 >= height {
                continue;
            }
            let fy = rng.random_range(floor + 2..height.saturating_sub(1).max(floor + 3));
            if rng.random::<f32>() < 0.5 {
                draw_fruit(
                    &mut grid,
                    fx,
                    fy,
                    rng.random_range(0..5),
                    lighten(palette[3], 10),
                );
            } else {
                draw_mask(
                    &mut grid,
                    fx,
                    fy,
                    2,
                    rng.random_range(0..MASK_STYLE_COUNT),
                    lighten(palette[3], 10),
                );
            }
        }
    } else if mode == "circuit" {
        // circuit [traces] -- PCB traces with pads, Manhattan routing
        let trace_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(14);
        let trace_count = trace_count.clamp(1, 60);

        // board: faint dot grid
        for y in 0..height {
            for x in 0..width {
                if x % 4 == 0 && y % 2 == 0 {
                    grid[y][x] = Cell::new('·', darken(palette[1], 90));
                }
            }
        }

        let free = |g: &Grid, x: i32, y: i32| -> bool {
            let c = g[y as usize][x as usize].ch;
            c == ' ' || c == '·'
        };

        let trace_colors = [palette[1], palette[2], palette[3]];
        let mut placed = 0;
        let mut attempts = 0;
        while placed < trace_count && attempts < trace_count * 8 {
            attempts += 1;
            let mut x = rng.random_range(2..width as i32 - 2);
            let mut y = rng.random_range(1..height as i32 - 1);
            if !free(&grid, x, y) {
                continue;
            }

            let mut pts: Vec<(i32, i32)> = vec![(x, y)];
            let dirs = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)];
            let mut dir = dirs[rng.random_range(0..4)];
            let segs = rng.random_range(2..5);
            'seg: for _ in 0..segs {
                let len = rng.random_range(4..13);
                for _ in 0..len {
                    let nx = x + dir.0;
                    let ny = y + dir.1;
                    if nx < 1 || ny < 1 || nx >= width as i32 - 1 || ny >= height as i32 - 1 {
                        break 'seg;
                    }
                    if !free(&grid, nx, ny) || pts.contains(&(nx, ny)) {
                        break 'seg;
                    }
                    x = nx;
                    y = ny;
                    pts.push((x, y));
                }
                dir = if dir.0 != 0 {
                    if rng.random::<f32>() < 0.5 {
                        (0, 1)
                    } else {
                        (0, -1)
                    }
                } else {
                    if rng.random::<f32>() < 0.5 {
                        (1, 0)
                    } else {
                        (-1, 0)
                    }
                };
            }
            if pts.len() < 4 {
                continue;
            }

            let color = trace_colors[placed % trace_colors.len()];
            let pad_color = lighten(color, 30);
            for i in 0..pts.len() {
                let (px, py) = pts[i];
                let ch = if i == 0 || i == pts.len() - 1 {
                    '◉'
                } else {
                    let din = (pts[i].0 - pts[i - 1].0, pts[i].1 - pts[i - 1].1);
                    let dout = (pts[i + 1].0 - pts[i].0, pts[i + 1].1 - pts[i].1);
                    match (din, dout) {
                        ((1, 0), (1, 0)) | ((-1, 0), (-1, 0)) => '─',
                        ((0, 1), (0, 1)) | ((0, -1), (0, -1)) => '│',
                        ((1, 0), (0, 1)) | ((0, -1), (-1, 0)) => '╮',
                        ((1, 0), (0, -1)) | ((0, 1), (-1, 0)) => '╯',
                        ((-1, 0), (0, 1)) | ((0, -1), (1, 0)) => '╭',
                        ((-1, 0), (0, -1)) | ((0, 1), (1, 0)) => '╰',
                        _ => '·',
                    }
                };
                let c = if i == 0 || i == pts.len() - 1 {
                    pad_color
                } else {
                    color
                };
                grid[py as usize][px as usize] = Cell::new(ch, c);
            }
            placed += 1;
        }
    } else if mode == "quilt" {
        // quilt [min_patch] [max_patch] -- stitched patchwork of tile patterns
        let min_p: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(10);
        let max_p: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(22);
        let min_p = min_p.clamp(4, 30);
        let max_p = max_p.clamp(min_p + 1, 40);

        let col_strips = allocate_strips(width, min_p, max_p, &mut rng);
        let row_strips = allocate_strips(height, (min_p / 2).max(3), (max_p / 2).max(4), &mut rng);

        let mut patch_rects: Vec<Rect> = Vec::new();
        for &(ry, rh) in &row_strips {
            for &(cx, cw) in &col_strips {
                let r = Rect {
                    x: cx,
                    y: ry,
                    w: cw,
                    h: rh,
                };
                let variant = tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT));
                let c1 = darken(palette[1 + rng.random_range(0..3)], rng.random_range(0..50));
                let c2 = darken(
                    palette[1 + rng.random_range(0..3)],
                    rng.random_range(20..70),
                );
                fill_tile_pure(&mut grid, &r, variant, c1, c2);
                patch_rects.push(r);
            }
        }

        // stitched seams between patches
        let thread = darken(palette[4], 40);
        for &(cx, _) in col_strips.iter().skip(1) {
            for y in 0..height {
                grid[y][cx] = Cell::new('┆', thread);
            }
        }
        for &(ry, _) in row_strips.iter().skip(1) {
            for x in 0..width {
                grid[ry][x] = Cell::new('┄', thread);
            }
        }
        for &(ry, _) in row_strips.iter().skip(1) {
            for &(cx, _) in col_strips.iter().skip(1) {
                grid[ry][cx] = Cell::new('+', thread);
            }
        }

        // applique: stamp flowers on a few of the larger patches
        let mut candidates: Vec<&Rect> = patch_rects
            .iter()
            .filter(|r| r.w >= 9 && r.h >= 7)
            .collect();
        for _ in 0..3 {
            if candidates.is_empty() {
                break;
            }
            let idx = rng.random_range(0..candidates.len());
            let r = candidates.remove(idx);
            let cx = r.x + r.w / 2;
            let cy = r.y + r.h / 2;
            draw_flower(
                &mut grid,
                cx,
                cy,
                rng.random_range(0..5),
                lighten(palette[3], 20),
            );
        }
    } else if mode == "patchwalk" {
        // patchwalk [stops] [line_w] -- quilt x scene-walk x mondrian2:
        // skewed BSP with big flat fields against small quilted clusters,
        // a heavy thread route stitched between clearings.
        let stop_count: usize = args
            .get(4)
            .and_then(|s| s.parse().ok())
            .unwrap_or(4)
            .clamp(2, 8);
        let line_w: usize = args
            .get(5)
            .and_then(|s| s.parse().ok())
            .unwrap_or(2)
            .clamp(1, 3);

        // 1. Binding everywhere; leaves get carved out of it
        let line_color = darken(palette[0], 60);
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::with_bg(' ', line_color, line_color);
            }
        }

        // 2. Skewed BSP: splits land at 0.22-0.38, one branch can stop
        // early so big fields sit next to small clusters
        let mut leaves: Vec<Rect> = Vec::new();
        let mut stack: Vec<(Rect, usize)> = vec![(
            Rect {
                x: line_w,
                y: 1,
                w: width - line_w * 2,
                h: height - 2,
            },
            0,
        )];
        while let Some((r, d)) = stack.pop() {
            let can_v = r.w >= 15 + line_w;
            let can_h = r.h >= 8;
            let stop_p = match d {
                0 => 0.0,
                1 => 0.08,
                2 => 0.3,
                _ => 0.55,
            };
            if (!can_v && !can_h) || d >= 5 || rng.random::<f32>() < stop_p {
                leaves.push(r);
                continue;
            }
            let vert = if !can_h {
                true
            } else if !can_v {
                false
            } else if r.w > r.h * 3 {
                true
            } else if r.h * 2 > r.w {
                false
            } else {
                rng.random_range(0..2u32) == 0
            };
            let mut t = 0.22 + rng.random::<f32>() * 0.16;
            if rng.random_range(0..2u32) == 0 {
                t = 1.0 - t;
            }
            if vert {
                let sw = ((r.w as f32 * t) as usize).clamp(6, r.w - 6 - line_w);
                stack.push((
                    Rect {
                        x: r.x,
                        y: r.y,
                        w: sw,
                        h: r.h,
                    },
                    d + 1,
                ));
                stack.push((
                    Rect {
                        x: r.x + sw + line_w,
                        y: r.y,
                        w: r.w - sw - line_w,
                        h: r.h,
                    },
                    d + 1,
                ));
            } else {
                let sh = ((r.h as f32 * t) as usize).clamp(3, r.h - 4);
                stack.push((
                    Rect {
                        x: r.x,
                        y: r.y,
                        w: r.w,
                        h: sh,
                    },
                    d + 1,
                ));
                stack.push((
                    Rect {
                        x: r.x,
                        y: r.y + sh + 1,
                        w: r.w,
                        h: r.h - sh - 1,
                    },
                    d + 1,
                ));
            }
        }

        // 3. Treatments: big leaves lean flat color (mondrian fields),
        // small leaves lean quilted; white-weighted like the source
        let canvas = lighten(palette[0], 45);
        let field_colors = [canvas, canvas, canvas, palette[1], palette[2], palette[3]];
        let thread = darken(palette[4], 30);
        for r in &leaves {
            let area = r.w * r.h;
            let flat_p = if area > 400 {
                0.92
            } else if area > 300 {
                0.75
            } else if area > 120 {
                0.5
            } else {
                0.3
            };
            if rng.random::<f32>() < flat_p {
                let bg = field_colors[rng.random_range(0..field_colors.len())];
                for y in r.y..(r.y + r.h).min(height) {
                    for x in r.x..(r.x + r.w).min(width) {
                        grid[y][x] = Cell::with_bg(' ', bg, bg);
                    }
                }
            } else {
                let variant = tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT));
                let c1 = darken(palette[1 + rng.random_range(0..3)], rng.random_range(0..50));
                let c2 = darken(
                    palette[1 + rng.random_range(0..3)],
                    rng.random_range(20..70),
                );
                fill_tile_pure(&mut grid, r, variant, c1, c2);
                // running stitch just inside the patch edge
                let (x0, y0) = (r.x, r.y);
                let (x1, y1) = (r.x + r.w - 1, r.y + r.h - 1);
                for x in x0..=x1 {
                    if x % 2 == 0 && y0 < height && x < width {
                        grid[y0][x] = Cell::new('┈', thread);
                    }
                    if x % 2 == 0 && y1 < height && x < width {
                        grid[y1][x] = Cell::new('┈', thread);
                    }
                }
                for y in y0..=y1 {
                    if y % 2 == 0 && y < height && x0 < width {
                        grid[y][x0] = Cell::new('┊', thread);
                    }
                    if y % 2 == 0 && y < height && x1 < width {
                        grid[y][x1] = Cell::new('┊', thread);
                    }
                }
            }
        }

        // 4. Stops: centers of randomly chosen roomy leaves, walked
        // left to right
        let mut cands: Vec<(usize, usize)> = leaves
            .iter()
            .filter(|r| r.w >= 10 && r.h >= 5)
            .map(|r| (r.x + r.w / 2, r.y + r.h / 2))
            .collect();
        let mut stops: Vec<(usize, usize)> = Vec::new();
        while stops.len() < stop_count && !cands.is_empty() {
            let i = rng.random_range(0..cands.len() as u32) as usize;
            stops.push(cands.remove(i));
        }
        stops.sort_by_key(|s| s.0);

        // 5. Thread route: heavy box-drawing polyline, orthogonal runs
        // with elbows alternating horizontal-first / vertical-first
        let mut pts: Vec<(i32, i32)> = Vec::new();
        for (i, &(sx, sy)) in stops.iter().enumerate() {
            let p = (sx as i32, sy as i32);
            if i > 0 {
                let last = *pts.last().unwrap();
                let elbow = if i % 2 == 1 {
                    (p.0, last.1)
                } else {
                    (last.0, p.1)
                };
                if elbow != last && elbow != p {
                    pts.push(elbow);
                }
            }
            pts.push(p);
        }
        let path_color = lighten(palette[4], 25);
        let dir_of = |a: (i32, i32), b: (i32, i32)| -> (i32, i32) {
            ((b.0 - a.0).signum(), (b.1 - a.1).signum())
        };
        for seg in pts.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let d = dir_of(a, b);
            if d == (0, 0) {
                continue;
            }
            let ch = if d.0 != 0 { '━' } else { '┃' };
            let mut p = a;
            loop {
                p = (p.0 + d.0, p.1 + d.1);
                if p == b {
                    break;
                }
                if p.0 >= 0 && p.1 >= 0 && (p.0 as usize) < width && (p.1 as usize) < height {
                    grid[p.1 as usize][p.0 as usize] = Cell::new(ch, path_color);
                }
            }
        }
        for i in 1..pts.len().saturating_sub(1) {
            let din = dir_of(pts[i - 1], pts[i]);
            let dout = dir_of(pts[i], pts[i + 1]);
            if din == (0, 0) || dout == (0, 0) {
                continue;
            }
            let ch = match (din, dout) {
                ((1, 0), (0, 1)) | ((0, -1), (-1, 0)) => '┓',
                ((1, 0), (0, -1)) | ((0, 1), (-1, 0)) => '┛',
                ((-1, 0), (0, 1)) | ((0, -1), (1, 0)) => '┏',
                ((-1, 0), (0, -1)) | ((0, 1), (1, 0)) => '┗',
                _ if din == dout => {
                    if din.0 != 0 {
                        '━'
                    } else {
                        '┃'
                    }
                }
                _ => '╋',
            };
            let (vx, vy) = pts[i];
            if vx >= 0 && vy >= 0 && (vx as usize) < width && (vy as usize) < height {
                grid[vy as usize][vx as usize] = Cell::new(ch, path_color);
            }
        }

        // 6. Clearings at each stop: punch through, applique inside
        for (si, &(sx, sy)) in stops.iter().enumerate() {
            let rx = rng.random_range(3..7u32) as i32;
            let ry = rng.random_range(2..4u32) as i32;
            for y in (sy as i32 - ry)..=(sy as i32 + ry) {
                for x in (sx as i32 - rx)..=(sx as i32 + rx) {
                    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                        continue;
                    }
                    let nx = (x - sx as i32) as f32 / rx as f32;
                    let ny = (y - sy as i32) as f32 / ry as f32;
                    if nx * nx + ny * ny <= 1.0 {
                        grid[y as usize][x as usize] = Cell::blank();
                    }
                }
            }
            match rng.random_range(0..4u32) {
                0 => {
                    draw_flower(&mut grid, sx, sy, rng.random_range(0..5), palette[3]);
                }
                1 => {
                    draw_fruit(&mut grid, sx, sy, rng.random_range(0..5), palette[2]);
                }
                2 => {
                    let canopy = sy.saturating_sub(ry as usize);
                    draw_tree(
                        &mut grid,
                        sx,
                        sy + ry as usize - 1,
                        canopy,
                        (rx as usize / 2).max(2),
                        rng.random_range(0..12),
                        palette[1],
                        &mut rng,
                    );
                }
                _ => {
                    draw_flower(
                        &mut grid,
                        sx,
                        sy,
                        rng.random_range(0..5),
                        palette[rng.random_range(1..4)],
                    );
                }
            }
            let label = format!("{}", si + 1);
            let ly = sy + ry as usize + 1;
            if ly < height && sx < width {
                grid[ly][sx] = Cell::new(label.chars().next().unwrap(), thread);
            }
        }
    } else if mode == "aurora" {
        // aurora [bands] -- layered night-sky ribbons over a snowy horizon
        let band_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
        let band_count = band_count.clamp(1, 10);

        let sky = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &sky,
            NoiseVariant::Dot,
            darken(palette[2], 95),
            darken(palette[3], 80),
            &mut rng,
        );

        let horizon = (height * 3 / 4).max(1).min(height.saturating_sub(2));
        let star_chars = ['·', '∙', '°', '*'];
        let star_count = (width * horizon / 28).max(6);
        for _ in 0..star_count {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..horizon.max(1));
            let ch = star_chars[rng.random_range(0..star_chars.len())];
            grid[y][x] = Cell::new(ch, darken(lighten(palette[4], 10), rng.random_range(0..45)));
        }

        for b in 0..band_count {
            let color = shift_hue(lighten(palette[3], 20), b as f64 * 28.0);
            let glow = shift_hue(palette[1], b as f64 * 33.0);
            let base =
                height / 7 + (b + 1) * horizon.saturating_sub(height / 6).max(1) / (band_count + 2);
            let amp = rng.random_range(2..(height / 6).max(4) as u32) as f32;
            let thick = rng.random_range(2..5i32);
            let freq1 = rng.random_range(8..18u32) as f32;
            let freq2 = rng.random_range(18..35u32) as f32;
            let phase = rng.random::<f32>() * std::f32::consts::TAU;

            for x in 0..width {
                let xf = x as f32;
                let y_mid = base as f32
                    + (xf / freq1 + phase).sin() * amp
                    + (xf / freq2 + phase * 0.7).sin() * amp * 0.55;
                for dy in -thick..=thick {
                    let y = y_mid.round() as i32 + dy;
                    if y < 0 || y as usize >= horizon {
                        continue;
                    }
                    let falloff = 1.0 - (dy.abs() as f32 / (thick + 1) as f32);
                    if rng.random::<f32>() > 0.45 + falloff * 0.5 {
                        continue;
                    }
                    let ch = match dy.abs() {
                        0 => '═',
                        1 => '─',
                        2 => '~',
                        _ => '·',
                    };
                    let c = if dy == 0 {
                        color
                    } else {
                        darken(glow, (dy.abs() * 12) as u8)
                    };
                    grid[y as usize][x] = Cell::new(ch, c);
                }
            }
        }

        let snow_phase = rng.random::<f32>() * std::f32::consts::TAU;
        for x in 0..width {
            let crest = horizon as i32 + ((x as f32 / 9.0 + snow_phase).sin() * 2.0).round() as i32;
            for y in crest.max(0) as usize..height {
                let depth = y.saturating_sub(horizon);
                let ch = match (x + y * 3) % 9 {
                    0 | 1 => '·',
                    2 => '∿',
                    3 => '╱',
                    4 => '╲',
                    _ => ' ',
                };
                if ch != ' ' {
                    grid[y][x] = Cell::new(
                        ch,
                        darken(lighten(palette[1], 45), (depth * 5).min(90) as u8),
                    );
                }
            }
            if crest >= 0 && (crest as usize) < height {
                let ch = if x % 2 == 0 { '╱' } else { '╲' };
                grid[crest as usize][x] = Cell::new(ch, lighten(palette[4], 5));
            }
        }
    } else if mode == "aura2" {
        // aura2 [rain] -- sparse rain behind aurora ribbons and snowfields
        let rain: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(34);
        let rain = rain.clamp(0, 120);

        let sky = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &sky,
            NoiseVariant::Dot,
            darken(palette[2], 97),
            darken(palette[1], 92),
            &mut rng,
        );

        let horizon = (height * 3 / 4).max(1).min(height.saturating_sub(2));
        let wind = if seed % 2 == 0 { 1i32 } else { -1i32 };
        let drops = width * horizon * rain / 420;
        for _ in 0..drops {
            let len = rng.random_range(1..4i32);
            let x0 = rng.random_range(0..width) as i32;
            let y0 = rng.random_range(0..horizon.max(1)) as i32;
            for step in 0..len {
                let x = x0 + wind * step / 2;
                let y = y0 + step;
                if x < 0 || y < 0 || x as usize >= width || y as usize >= horizon {
                    continue;
                }
                let ch = if wind > 0 { '╲' } else { '╱' };
                grid[y as usize][x as usize] = Cell::new(ch, darken(palette[4], 78));
            }
        }

        let star_chars = ['·', '∙', '°'];
        for _ in 0..(width * horizon / 42).max(4) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..horizon.max(1));
            let ch = star_chars[rng.random_range(0..star_chars.len())];
            if grid[y][x].ch == ' ' || rng.random_range(0..3u32) == 0 {
                grid[y][x] =
                    Cell::new(ch, darken(lighten(palette[4], 8), rng.random_range(35..75)));
            }
        }

        let band_count = rng.random_range(4..7usize);
        for b in 0..band_count {
            let color = shift_hue(lighten(palette[3], 26), b as f64 * 31.0);
            let glow = shift_hue(palette[1], b as f64 * 41.0);
            let base =
                height / 8 + (b + 1) * horizon.saturating_sub(height / 7).max(1) / (band_count + 2);
            let amp = rng.random_range(2..(height / 5).max(5) as u32) as f32;
            let thick = if b % 2 == 0 { 3i32 } else { 2i32 };
            let freq1 = rng.random_range(9..22u32) as f32;
            let freq2 = rng.random_range(20..44u32) as f32;
            let phase = rng.random::<f32>() * std::f32::consts::TAU;

            for x in 0..width {
                let xf = x as f32;
                let y_mid = base as f32
                    + (xf / freq1 + phase).sin() * amp
                    + (xf / freq2 + phase * 0.5).sin() * amp * 0.45;
                for dy in -thick..=thick {
                    let y = y_mid.round() as i32 + dy;
                    if y < 0 || y as usize >= horizon {
                        continue;
                    }
                    let falloff = 1.0 - (dy.abs() as f32 / (thick + 1) as f32);
                    if rng.random::<f32>() > 0.38 + falloff * 0.57 {
                        continue;
                    }
                    let ch = match dy.abs() {
                        0 => '═',
                        1 => '─',
                        2 => '~',
                        _ => '·',
                    };
                    let c = if dy == 0 {
                        color
                    } else {
                        darken(glow, (dy.abs() * 16) as u8)
                    };
                    grid[y as usize][x] = Cell::new(ch, c);
                }
            }
        }

        let ridge = gen_contour(width, horizon, (height / 12).max(2), 0.55, &mut rng);
        for x in 0..width {
            let crest = ridge[x].min(height - 1);
            for y in crest..height {
                let depth = y.saturating_sub(crest);
                let ch = match (x * 2 + y * 3) % 11 {
                    0 | 1 => '·',
                    2 => '∿',
                    3 => '╱',
                    4 => '╲',
                    _ => ' ',
                };
                if ch != ' ' {
                    grid[y][x] = Cell::new(
                        ch,
                        darken(lighten(palette[1], 48), (depth * 4).min(80) as u8),
                    );
                }
            }
        }
        let full = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        draw_contour_ridge(&mut grid, &full, &ridge, lighten(palette[4], 5));
    } else if mode == "harbor" {
        // harbor [boats] -- ridiculous neon harbor carnival with cranes and fireworks
        let boat_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(9);
        let boat_count = boat_count.clamp(1, 24);
        let horizon = (height * 9 / 20).max(6).min(height.saturating_sub(6));

        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..20u32) == 0 {
                    grid[y][x] = Cell::new('·', darken(palette[4], 72));
                }
            }
        }

        let moon_x = width * 7 / 10;
        let moon_y = (horizon / 3).max(2);
        let moon_rx = (width / 9).max(5) as i32;
        let moon_ry = (height / 7).max(3) as i32;
        for y in
            moon_y.saturating_sub(moon_ry as usize)..=(moon_y + moon_ry as usize).min(height - 1)
        {
            for x in
                moon_x.saturating_sub(moon_rx as usize)..=(moon_x + moon_rx as usize).min(width - 1)
            {
                let dx = (x as i32 - moon_x as i32) as f32 / moon_rx as f32;
                let dy = (y as i32 - moon_y as i32) as f32 / moon_ry as f32;
                if dx * dx + dy * dy <= 1.0 {
                    let face = if (x + y) % 11 == 0 { '◉' } else { '●' };
                    grid[y][x] = Cell::new(face, lighten(palette[3], 35));
                }
            }
        }

        for _ in 0..5 {
            let fx = rng.random_range(6..width.saturating_sub(6).max(7));
            let fy = rng.random_range(2..horizon.saturating_sub(2).max(3));
            let burst = ['✦', '*', '+', '·'];
            for r in 0..4i32 {
                let c = shift_hue(lighten(palette[3], 20), rng.random_range(0..180u32) as f64);
                for &(dx, dy, ch) in &[
                    (r, 0, '─'),
                    (-r, 0, '─'),
                    (0, r, '│'),
                    (0, -r, '│'),
                    (r, r / 2, '╲'),
                    (-r, r / 2, '╱'),
                    (r, -r / 2, '╱'),
                    (-r, -r / 2, '╲'),
                ] {
                    let x = fx as i32 + dx;
                    let y = fy as i32 + dy;
                    if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < horizon {
                        grid[y as usize][x as usize] = Cell::new(
                            if r == 0 {
                                burst[rng.random_range(0..burst.len())]
                            } else {
                                ch
                            },
                            c,
                        );
                    }
                }
            }
        }

        let wheel_cx = width / 4;
        let wheel_cy = horizon.saturating_sub((height / 9).max(2));
        let wheel_r = (height / 4).max(4).min(width / 7);
        for i in 0..64 {
            let a = i as f32 / 64.0 * std::f32::consts::TAU;
            let x = wheel_cx as i32 + (a.cos() * wheel_r as f32 * 2.0).round() as i32;
            let y = wheel_cy as i32 + (a.sin() * wheel_r as f32).round() as i32;
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < horizon {
                grid[y as usize][x as usize] = Cell::new('○', lighten(palette[3], 10));
            }
        }
        for spoke in 0..8 {
            let a = spoke as f32 / 8.0 * std::f32::consts::TAU;
            for r in 0..=wheel_r {
                let x = wheel_cx as i32 + (a.cos() * r as f32 * 2.0).round() as i32;
                let y = wheel_cy as i32 + (a.sin() * r as f32).round() as i32;
                if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < horizon {
                    grid[y as usize][x as usize] = Cell::new(
                        if spoke % 2 == 0 { '─' } else { '╱' },
                        darken(palette[4], 15),
                    );
                }
            }
        }
        if wheel_cx < width && wheel_cy < height {
            grid[wheel_cy][wheel_cx] = Cell::new('◉', lighten(palette[4], 20));
        }

        let mut x = 0usize;
        while x < width {
            let w = rng.random_range(3..8usize);
            let h = rng.random_range(3..horizon.max(4));
            let top = horizon.saturating_sub(h);
            let color = shift_hue(palette[2], rng.random_range(0..120u32) as f64);
            for bx in x..(x + w).min(width) {
                for by in top..horizon {
                    let lit = (bx + by + seed as usize) % 5 == 0;
                    let ch = if by == top {
                        '▄'
                    } else if lit {
                        '▪'
                    } else {
                        '█'
                    };
                    let c = if lit {
                        lighten(palette[3], 20)
                    } else {
                        darken(color, 50)
                    };
                    grid[by][bx] = Cell::new(ch, c);
                }
            }
            x += w + rng.random_range(0..3usize);
        }

        for &crane_x in &[width / 8, width * 6 / 8] {
            let top = horizon.saturating_sub((height / 4).max(4));
            for y in top..horizon {
                if crane_x < width {
                    grid[y][crane_x] = Cell::new('┃', lighten(palette[3], 10));
                }
            }
            for x in crane_x..(crane_x + width / 6).min(width) {
                grid[top][x] = Cell::new('━', lighten(palette[3], 10));
            }
            let hook_x = (crane_x + width / 7).min(width - 1);
            for y in top..(top + 5).min(horizon) {
                grid[y][hook_x] = Cell::new('│', lighten(palette[4], 5));
            }
            if top + 5 < horizon {
                grid[top + 5][hook_x] = Cell::new('◆', palette[3]);
            }
        }

        let wave_chars = ['~', '≈', '∿', '─', ' '];
        for y in horizon..height {
            for x in 0..width {
                let drift = ((x as f32 / 5.0) + (y as f32 / 2.0)).sin();
                let idx =
                    ((x + y * 2 + seed as usize) % wave_chars.len()).min(wave_chars.len() - 1);
                let ch = if drift > 0.15 { '≈' } else { wave_chars[idx] };
                if ch != ' ' {
                    let depth = y - horizon;
                    let c = if (x + y) % 9 == 0 {
                        shift_hue(lighten(palette[3], 15), (x * 5 % 180) as f64)
                    } else {
                        darken(palette[1], (depth * 3).min(80) as u8)
                    };
                    grid[y][x] = Cell::new(ch, c);
                }
            }
        }

        let pier_y = (height * 2 / 3).min(height.saturating_sub(2));
        for x in 0..width {
            if x % 2 == 0 {
                grid[pier_y][x] = Cell::new('━', darken(palette[4], 25));
            }
        }
        for px in (width / 12..width * 11 / 12).step_by(7) {
            for y in pier_y..height {
                grid[y][px] = Cell::new('┃', darken(palette[4], 45));
            }
        }

        for _ in 0..boat_count {
            if width < 12 || height < horizon + 5 {
                break;
            }
            let len = rng.random_range(6..16usize).min(width.saturating_sub(3));
            let bx = rng.random_range(1..width.saturating_sub(len + 1).max(2));
            let by = rng.random_range(horizon + 2..height.saturating_sub(2).max(horizon + 3));
            let hull = shift_hue(lighten(palette[2], 25), rng.random_range(0..240u32) as f64);
            grid[by][bx] = Cell::new('╲', hull);
            for i in 1..len - 1 {
                if bx + i < width {
                    grid[by][bx + i] = Cell::new('━', hull);
                }
            }
            if bx + len - 1 < width {
                grid[by][bx + len - 1] = Cell::new('╱', hull);
            }

            let mast_x = bx + len / 2;
            let mast_h = rng.random_range(3..8usize).min(by);
            for k in 1..=mast_h {
                grid[by - k][mast_x] = Cell::new('│', lighten(palette[4], 5));
            }
            for k in 1..mast_h {
                let sx = mast_x.saturating_sub(k);
                if by >= k && sx < width {
                    grid[by - k][sx] = Cell::new('╱', lighten(palette[3], 15));
                }
                let sx2 = mast_x + k;
                if by >= k && sx2 < width && k < mast_h - 1 {
                    grid[by - k][sx2] = Cell::new('╲', lighten(palette[4], 5));
                }
            }
        }
    } else if mode == "labyrinth" {
        // labyrinth [markers] -- nested stone walls, deliberate gates, and one glowing route
        let marker_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(18);
        let marker_count = marker_count.clamp(0, 96);

        let bg_color = darken(palette[0], 8);
        let dust_color = darken(palette[2], 50);
        let floor_color = darken(palette[4], 44);
        let wall_color = lighten(palette[1], 18);
        let wall_shadow = darken(palette[2], 18);
        let path_color = lighten(palette[3], 34);
        let relic_color = lighten(palette[3], 48);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 11 + y * 7 + seed as usize * 3) % 41;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 => '░',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg_color)
                } else {
                    Cell::new(ch, dust_color)
                };
            }
        }

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let margin_x = (width / 10).clamp(4, 18);
        let margin_y = (height / 8).clamp(2, 7);
        let left = margin_x as i32;
        let right = width.saturating_sub(margin_x + 1) as i32;
        let top = margin_y as i32;
        let bottom = height.saturating_sub(margin_y + 1) as i32;

        if right - left >= 18 && bottom - top >= 8 {
            for y in top.saturating_sub(1)..=(bottom + 1).min(height as i32 - 1) {
                for x in left.saturating_sub(2)..=(right + 2).min(width as i32 - 1) {
                    if x < 0 || y < 0 {
                        continue;
                    }
                    let floor_noise =
                        ((x as usize * 5 + y as usize * 13 + seed as usize) % 29) == 0;
                    grid[y as usize][x as usize] = if floor_noise {
                        Cell::new('·', floor_color)
                    } else {
                        Cell::blank()
                    };
                }
            }

            let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
                if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                    grid[y as usize][x as usize] = Cell::new(ch, fg);
                }
            };
            let put_path = |grid: &mut Grid, x: i32, y: i32, step: usize| {
                if x < 0 || y < 0 || (x as usize) >= width || (y as usize) >= height {
                    return;
                }
                if matches!(
                    grid[y as usize][x as usize].ch,
                    '═' | '║' | '╔' | '╗' | '╚' | '╝' | '█' | '▓' | '╫' | '╬'
                ) {
                    return;
                }
                let ch = match step % 5 {
                    0 => '•',
                    1 | 2 => '·',
                    _ => '∙',
                };
                grid[y as usize][x as usize] = Cell::new(ch, path_color);
            };
            let draw_line = |grid: &mut Grid, a: (i32, i32), b: (i32, i32), start_step: usize| {
                let mut step = start_step;
                let (mut x, mut y) = a;
                while x != b.0 {
                    put_path(grid, x, y, step);
                    x += if b.0 > x { 1 } else { -1 };
                    step += 1;
                }
                while y != b.1 {
                    put_path(grid, x, y, step);
                    y += if b.1 > y { 1 } else { -1 };
                    step += 1;
                }
                put_path(grid, x, y, step);
            };
            let side_at = |side: usize, l: i32, t: i32, r: i32, b: i32, gx: i32, gy: i32| match side
            {
                0 => (gx.clamp(l + 2, r - 2), t),
                1 => (r, gy.clamp(t + 1, b - 1)),
                2 => (gx.clamp(l + 2, r - 2), b),
                _ => (l, gy.clamp(t + 1, b - 1)),
            };
            let inside_gate =
                |side: usize, l: i32, t: i32, r: i32, b: i32, gx: i32, gy: i32| match side {
                    0 => (gx.clamp(l + 2, r - 2), t + 1),
                    1 => (r - 1, gy.clamp(t + 1, b - 1)),
                    2 => (gx.clamp(l + 2, r - 2), b - 1),
                    _ => (l + 1, gy.clamp(t + 1, b - 1)),
                };
            let outside_gate =
                |side: usize, l: i32, t: i32, r: i32, b: i32, gx: i32, gy: i32| match side {
                    0 => (gx.clamp(l + 2, r - 2), t - 1),
                    1 => (r + 1, gy.clamp(t + 1, b - 1)),
                    2 => (gx.clamp(l + 2, r - 2), b + 1),
                    _ => (l - 1, gy.clamp(t + 1, b - 1)),
                };
            let perimeter_dist = |side: usize, x: i32, y: i32, l: i32, t: i32, r: i32, b: i32| {
                let w = r - l;
                let h = b - t;
                match side {
                    0 => x - l,
                    1 => w + y - t,
                    2 => w + h + r - x,
                    _ => w + h + w + b - y,
                }
            };
            let perimeter_point = |dist: i32, l: i32, t: i32, r: i32, b: i32| {
                let w = r - l;
                let h = b - t;
                let p = 2 * (w + h);
                let d = dist.rem_euclid(p);
                if d <= w {
                    (l + d, t)
                } else if d <= w + h {
                    (r, t + d - w)
                } else if d <= w + h + w {
                    (r - (d - w - h), b)
                } else {
                    (l, b - (d - w - h - w))
                }
            };

            let max_layers =
                ((height.saturating_sub(10) / 4).min(width.saturating_sub(20) / 12)).clamp(3, 8);
            let inset_x = ((right - left) as usize / (max_layers * 2 + 3)).clamp(4, 14) as i32;
            let inset_y = ((bottom - top) as usize / (max_layers * 2 + 3)).clamp(2, 5) as i32;

            let mut rings: Vec<(i32, i32, i32, i32, usize, i32, i32)> = Vec::new();
            for layer in 0..max_layers {
                let l = left + layer as i32 * inset_x;
                let r = right - layer as i32 * inset_x;
                let t = top + layer as i32 * inset_y;
                let b = bottom - layer as i32 * inset_y;
                if r - l < 13 || b - t < 5 {
                    break;
                }
                let side = match layer % 4 {
                    0 => 2,
                    1 => 1,
                    2 => 0,
                    _ => 3,
                };
                let x_span = (r - l - 8).max(1);
                let y_span = (b - t - 4).max(1);
                let x_wobble =
                    ((seed as i32 + layer as i32 * 17).rem_euclid(11) - 5) * ((x_span / 18).max(1));
                let y_wobble = ((seed as i32 / 3 + layer as i32 * 13).rem_euclid(9) - 4)
                    * ((y_span / 10).max(1));
                let gx = (cx + x_wobble).clamp(l + 4, r - 4);
                let gy = (cy + y_wobble).clamp(t + 2, b - 2);
                rings.push((l, t, r, b, side, gx, gy));
            }

            for (layer, &(l, t, r, b, side, gx, gy)) in rings.iter().enumerate() {
                let tone = if layer % 2 == 0 {
                    wall_color
                } else {
                    darken(wall_color, 12)
                };
                let half_gate_x = 2 + (layer as i32 % 2);
                let half_gate_y = 1;

                for x in l..=r {
                    let top_gate = side == 0 && (gx - half_gate_x..=gx + half_gate_x).contains(&x);
                    let bottom_gate =
                        side == 2 && (gx - half_gate_x..=gx + half_gate_x).contains(&x);
                    if !top_gate {
                        put(&mut grid, x, t, '═', tone);
                    }
                    if !bottom_gate {
                        put(&mut grid, x, b, '═', tone);
                    }
                    if layer % 2 == 0 && x > l && x < r && x % 9 == (seed as i32 % 9) {
                        if !top_gate {
                            put(&mut grid, x, t, '╫', darken(tone, 6));
                        }
                        if !bottom_gate {
                            put(&mut grid, x, b, '╫', darken(tone, 6));
                        }
                    }
                }
                for y in t..=b {
                    let left_gate = side == 3 && (gy - half_gate_y..=gy + half_gate_y).contains(&y);
                    let right_gate =
                        side == 1 && (gy - half_gate_y..=gy + half_gate_y).contains(&y);
                    if !left_gate {
                        put(&mut grid, l, y, '║', tone);
                    }
                    if !right_gate {
                        put(&mut grid, r, y, '║', tone);
                    }
                    if layer % 2 == 1 && y > t && y < b && y % 5 == (seed as i32 % 5) {
                        if !left_gate {
                            put(&mut grid, l, y, '╫', darken(tone, 6));
                        }
                        if !right_gate {
                            put(&mut grid, r, y, '╫', darken(tone, 6));
                        }
                    }
                }

                put(&mut grid, l, t, '╔', tone);
                put(&mut grid, r, t, '╗', tone);
                put(&mut grid, l, b, '╚', tone);
                put(&mut grid, r, b, '╝', tone);

                let (gate_x, gate_y) = side_at(side, l, t, r, b, gx, gy);
                match side {
                    0 | 2 => {
                        put(
                            &mut grid,
                            gate_x - half_gate_x - 1,
                            gate_y,
                            '█',
                            wall_shadow,
                        );
                        put(
                            &mut grid,
                            gate_x + half_gate_x + 1,
                            gate_y,
                            '█',
                            wall_shadow,
                        );
                        put(&mut grid, gate_x, gate_y, '╬', relic_color);
                    }
                    _ => {
                        put(
                            &mut grid,
                            gate_x,
                            gate_y - half_gate_y - 1,
                            '█',
                            wall_shadow,
                        );
                        put(
                            &mut grid,
                            gate_x,
                            gate_y + half_gate_y + 1,
                            '█',
                            wall_shadow,
                        );
                        put(&mut grid, gate_x, gate_y, '╬', relic_color);
                    }
                }
            }

            if let Some(&(l0, t0, r0, b0, side0, gx0, gy0)) = rings.first() {
                let entry = outside_gate(side0, l0, t0, r0, b0, gx0, gy0);
                let mut current = inside_gate(side0, l0, t0, r0, b0, gx0, gy0);
                let label_y = (entry.1 + 1).min(height as i32 - 1);
                for y in entry.1.max(0)..=label_y {
                    for dx in -2..=2 {
                        put(&mut grid, entry.0 + dx, y, ' ', floor_color);
                    }
                }
                draw_line(&mut grid, entry, current, 0);
                put(&mut grid, entry.0, label_y, 'S', lighten(palette[1], 26));

                for i in 0..rings.len().saturating_sub(1) {
                    let (l, t, r, b, side, gx, gy) = rings[i];
                    let (nl, nt, nr, nb, next_side, next_gx, next_gy) = rings[i + 1];
                    let pl = (l + nl) / 2;
                    let pr = (r + nr) / 2;
                    let pt = (t + nt) / 2;
                    let pb = (b + nb) / 2;
                    if pr <= pl || pb <= pt {
                        continue;
                    }
                    let start = side_at(side, pl, pt, pr, pb, gx, gy);
                    let end = side_at(next_side, pl, pt, pr, pb, next_gx, next_gy);
                    draw_line(&mut grid, current, start, i * 97);

                    let p = 2 * ((pr - pl) + (pb - pt));
                    let d1 = perimeter_dist(side, start.0, start.1, pl, pt, pr, pb);
                    let d2 = perimeter_dist(next_side, end.0, end.1, pl, pt, pr, pb);
                    let cw = (d2 - d1).rem_euclid(p);
                    let ccw = (d1 - d2).rem_euclid(p);
                    let go_clockwise = if i % 3 == 1 { cw > ccw } else { cw <= ccw };
                    let steps = if go_clockwise { cw } else { ccw };
                    for step in 0..=steps {
                        let d = if go_clockwise { d1 + step } else { d1 - step };
                        let (px, py) = perimeter_point(d, pl, pt, pr, pb);
                        put_path(&mut grid, px, py, i * 131 + step as usize);
                    }

                    let next_outside = outside_gate(next_side, nl, nt, nr, nb, next_gx, next_gy);
                    let next_inside = inside_gate(next_side, nl, nt, nr, nb, next_gx, next_gy);
                    draw_line(&mut grid, end, next_outside, i * 173);
                    draw_line(&mut grid, next_outside, next_inside, i * 211);
                    current = next_inside;
                }

                let inner = rings
                    .last()
                    .copied()
                    .unwrap_or((l0, t0, r0, b0, side0, gx0, gy0));
                let chamber_w = ((inner.2 - inner.0) / 2).clamp(8, 22);
                let chamber_h = ((inner.3 - inner.1) / 2).clamp(3, 7);
                let cl = (cx - chamber_w / 2).clamp(inner.0 + 2, inner.2 - chamber_w - 1);
                let cr = cl + chamber_w;
                let ct = (cy - chamber_h / 2).clamp(inner.1 + 1, inner.3 - chamber_h - 1);
                let cb = ct + chamber_h;
                let door_side = inner.4;
                let door_x = cx.clamp(cl + 2, cr - 2);
                let door_y = cy.clamp(ct + 1, cb - 1);

                for x in cl..=cr {
                    if !(door_side == 0 && (door_x - 1..=door_x + 1).contains(&x)) {
                        put(&mut grid, x, ct, '═', relic_color);
                    }
                    if !(door_side == 2 && (door_x - 1..=door_x + 1).contains(&x)) {
                        put(&mut grid, x, cb, '═', relic_color);
                    }
                }
                for y in ct..=cb {
                    if !(door_side == 3 && (door_y - 1..=door_y + 1).contains(&y)) {
                        put(&mut grid, cl, y, '║', relic_color);
                    }
                    if !(door_side == 1 && (door_y - 1..=door_y + 1).contains(&y)) {
                        put(&mut grid, cr, y, '║', relic_color);
                    }
                }
                put(&mut grid, cl, ct, '╔', relic_color);
                put(&mut grid, cr, ct, '╗', relic_color);
                put(&mut grid, cl, cb, '╚', relic_color);
                put(&mut grid, cr, cb, '╝', relic_color);

                let chamber_entry = match door_side {
                    0 => (door_x, ct - 1),
                    1 => (cr + 1, door_y),
                    2 => (door_x, cb + 1),
                    _ => (cl - 1, door_y),
                };
                let chamber_inside = match door_side {
                    0 => (door_x, ct + 1),
                    1 => (cr - 1, door_y),
                    2 => (door_x, cb - 1),
                    _ => (cl + 1, door_y),
                };
                draw_line(&mut grid, current, chamber_entry, 901);
                draw_line(&mut grid, chamber_entry, chamber_inside, 941);
                draw_line(&mut grid, chamber_inside, (cx, cy), 991);
                put(&mut grid, cx, cy, '◉', relic_color);
            }

            let glyphs = ['◆', '◇', '✦', '✧', '+'];
            for _ in 0..marker_count {
                if rings.len() < 2 {
                    break;
                }
                let ring = rng.random_range(0..rings.len() - 1);
                let (ol, ot, or, ob, _, _, _) = rings[ring];
                let (il, it, ir, ib, _, _, _) = rings[ring + 1];
                let side = rng.random_range(0..4);
                let (x, y) = match side {
                    0 => (rng.random_range(il + 1..ir), (ot + it) / 2),
                    1 => ((or + ir) / 2, rng.random_range(it + 1..ib)),
                    2 => (rng.random_range(il + 1..ir), (ob + ib) / 2),
                    _ => ((ol + il) / 2, rng.random_range(it + 1..ib)),
                };
                if x <= 0 || y <= 0 || x as usize >= width - 1 || y as usize >= height - 1 {
                    continue;
                }
                if grid[y as usize][x as usize].ch == ' ' {
                    grid[y as usize][x as usize] =
                        Cell::new(glyphs[rng.random_range(0..glyphs.len())], relic_color);
                }
            }
        }
    } else if mode == "rainfall" {
        // rainfall [intensity] -- wind-sheared rain, gutters, puddles, and bright strikes
        let intensity: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(65);
        let intensity = intensity.clamp(5, 150);
        let wind = match seed % 3 {
            0 => -1i32,
            1 => 0,
            _ => 1,
        };

        let field = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &field,
            NoiseVariant::Dot,
            darken(palette[2], 90),
            darken(palette[1], 85),
            &mut rng,
        );

        let drops = (width * height * intensity / 180).max(width / 2);
        for _ in 0..drops {
            let len = rng.random_range(2..8i32);
            let x0 = rng.random_range(0..width) as i32;
            let y0 = rng.random_range(0..height) as i32;
            for step in 0..len {
                let x = x0 + wind * step / 2;
                let y = y0 + step;
                if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                    continue;
                }
                let ch = match wind {
                    -1 => '╱',
                    1 => '╲',
                    _ => '│',
                };
                grid[y as usize][x as usize] =
                    Cell::new(ch, darken(lighten(palette[4], 5), rng.random_range(0..45)));
            }
        }

        let gutter_y = height.saturating_sub(5);
        for x in 0..width {
            if x % 2 == 0 {
                grid[gutter_y][x] = Cell::new('═', darken(palette[4], 45));
            }
            if x % 17 == 0 {
                for y in gutter_y..height {
                    grid[y][x] = Cell::new('║', darken(palette[4], 55));
                }
            }
        }

        for _ in 0..(width / 5).max(4) {
            let px = rng.random_range(0..width);
            let py = rng.random_range(gutter_y..height);
            let r = rng.random_range(2..7usize);
            for dx in 0..r {
                let x = px + dx;
                if x >= width {
                    break;
                }
                let ch = ['~', '≈', '∿', '_'][rng.random_range(0..4usize)];
                grid[py][x] = Cell::new(ch, lighten(palette[1], 15));
            }
        }

        let strikes = if intensity > 95 { 2 } else { 1 };
        for _ in 0..strikes {
            let mut x = rng.random_range(width / 5..(width * 4 / 5).max(width / 5 + 1)) as i32;
            let end_y = rng.random_range((height / 3).max(1)..(height * 2 / 3).max(2));
            for y in 0..end_y {
                if x >= 0 && (x as usize) < width {
                    let ch = if rng.random_range(0..2u32) == 0 {
                        '╲'
                    } else {
                        '╱'
                    };
                    grid[y][x as usize] = Cell::new(ch, lighten(palette[3], 35));
                }
                x += rng.random_range(-1..=1);
            }
        }
    } else if mode == "meadow" {
        // meadow [density] -- windy wildflower field with stems, seed heads, and grass
        let density: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(70);
        let density = density.clamp(10, 180);
        let horizon = (height / 3).max(2);
        let ground = gen_contour(width, horizon, (height / 10).max(2), 0.55, &mut rng);

        for y in 0..height {
            for x in 0..width {
                if y < ground[x].min(height - 1) {
                    if rng.random_range(0..24u32) == 0 {
                        grid[y][x] = Cell::new('·', darken(palette[4], 70));
                    }
                    continue;
                }
                let depth = y.saturating_sub(ground[x]);
                let ch = ['╱', '╲', '│', '∿', '·', ' '][rng.random_range(0..6usize)];
                if ch != ' ' {
                    grid[y][x] = Cell::new(ch, darken(palette[1], (depth * 4).min(85) as u8));
                }
            }
        }

        let full = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        draw_contour_ridge(&mut grid, &full, &ground, darken(palette[3], 20));

        let stem_count = (width * density / 28).clamp(8, width.saturating_mul(3).max(8));
        for _ in 0..stem_count {
            let bx = rng.random_range(0..width);
            let base_y = rng.random_range(ground[bx].min(height - 1)..height);
            let len = rng.random_range(3..(height / 3).max(5) as u32) as i32;
            let lean = rng.random_range(-3..=3i32);
            let color = darken(palette[2], rng.random_range(0..45));
            let mut top = (bx as i32, base_y as i32);
            for i in 0..len {
                let t = i as f32 / len.max(1) as f32;
                let sway = (t * std::f32::consts::PI).sin() * lean as f32;
                let x = bx as i32 + sway.round() as i32;
                let y = base_y as i32 - i;
                if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                    continue;
                }
                let ch = if lean < -1 {
                    '╱'
                } else if lean > 1 {
                    '╲'
                } else {
                    '│'
                };
                grid[y as usize][x as usize] = Cell::new(ch, color);
                top = (x, y);
            }
            if top.0 <= 1
                || top.1 <= 1
                || top.0 as usize >= width - 1
                || top.1 as usize >= height - 1
            {
                continue;
            }
            let tx = top.0 as usize;
            let ty = top.1 as usize;
            match rng.random_range(0..5u32) {
                0 => draw_flower(
                    &mut grid,
                    tx,
                    ty,
                    rng.random_range(0..5),
                    lighten(palette[3], 15),
                ),
                1 => grow_flower_spiral(&mut grid, tx, ty, palette[3], &mut rng),
                2 => draw_fruit(
                    &mut grid,
                    tx,
                    ty,
                    rng.random_range(0..5),
                    lighten(palette[2], 20),
                ),
                _ => {
                    let seed_chars = ['✦', '✧', '*', '·'];
                    grid[ty][tx] = Cell::new(
                        seed_chars[rng.random_range(0..seed_chars.len())],
                        lighten(palette[4], 5),
                    );
                }
            }
        }
    } else if mode == "solar-system" {
        grid = draw_solar_system(grid, width, height, seed, palette, rng, t_anim, &args);
    } else if mode == "world2" {
        // world2 [shards] -- cracked/leaking biome partitions with aurora and scene islands
        let shard_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(6);
        let shard_count = shard_count.clamp(3, 10);
        let crack_count = shard_count.saturating_sub(1);

        let bg = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &bg,
            NoiseVariant::Dot,
            darken(palette[2], 96),
            darken(palette[3], 92),
            &mut rng,
        );

        let mut cracks: Vec<Vec<i32>> = Vec::new();
        for i in 0..crack_count {
            let band = (width / shard_count).max(2) as i32;
            let mut x = ((i + 1) as i32 * band + rng.random_range(-band / 4..=band / 4))
                .clamp(1, width as i32 - 2);
            let mut drift = 0i32;
            let mut path = Vec::with_capacity(height);
            for y in 0..height {
                path.push(x);
                if rng.random::<f32>() < 0.35 || y % 7 == 0 {
                    drift = rng.random_range(-1..=1);
                }
                x = (x + drift).clamp(1, width as i32 - 2);
            }
            cracks.push(path);
        }

        let mut bounds = vec![0usize];
        for path in &cracks {
            let avg = (path.iter().sum::<i32>() / path.len().max(1) as i32)
                .clamp(1, width as i32 - 1) as usize;
            bounds.push(avg);
        }
        bounds.push(width);
        bounds.sort();
        bounds.dedup();

        for i in 0..bounds.len().saturating_sub(1) {
            let left = bounds[i];
            let right = bounds[i + 1].max(left + 1).min(width);
            if right <= left {
                continue;
            }
            let rect = Rect {
                x: left,
                y: 0,
                w: right - left,
                h: height,
            };
            let biome = biome_from_index((i + seed as usize) % 5);
            render_biome(biome, &mut grid, &rect, &palette, &mut rng);
        }

        let base = grid.clone();
        for (ci, path) in cracks.iter().enumerate() {
            for y in 0..height {
                let seam_x = path[y];
                for spread in 1..=5i32 {
                    for side in [-1i32, 1i32] {
                        let tx = seam_x + side * spread;
                        let src = seam_x - side * rng.random_range(1..=7i32);
                        if tx < 0 || src < 0 || tx as usize >= width || src as usize >= width {
                            continue;
                        }
                        let leak_strength = 1.0 - spread as f32 / 6.0;
                        let upper_boost = if y < height / 3 { 0.18 } else { 0.0 };
                        if rng.random::<f32>() > leak_strength * 0.45 + upper_boost {
                            continue;
                        }
                        let mut cell = base[y][src as usize];
                        if cell.ch == ' ' && rng.random::<f32>() < 0.65 {
                            continue;
                        }
                        cell.fg = if (ci + y + spread as usize) % 3 == 0 {
                            shift_hue(darken(cell.fg, (spread * 8) as u8), 30.0)
                        } else {
                            darken(cell.fg, (spread * 10) as u8)
                        };
                        grid[y][tx as usize] = cell;
                    }
                }
            }
        }

        let aurora_bands = 4usize;
        for b in 0..aurora_bands {
            let color = shift_hue(lighten(palette[3], 25), b as f64 * 39.0);
            let base_y = height / 10 + b * (height / 4).max(1) / aurora_bands;
            let amp = rng.random_range(1..(height / 8).max(3) as u32) as f32;
            let phase = rng.random::<f32>() * std::f32::consts::TAU;
            for x in 0..width {
                let y = base_y as i32 + ((x as f32 / 13.0 + phase).sin() * amp).round() as i32;
                if y <= 0 || y as usize >= height / 2 {
                    continue;
                }
                if rng.random::<f32>() < 0.82 {
                    grid[y as usize][x] = Cell::new(if b % 2 == 0 { '═' } else { '~' }, color);
                }
                if y + 1 < height as i32 && rng.random::<f32>() < 0.45 {
                    grid[(y + 1) as usize][x] = Cell::new('·', darken(color, 30));
                }
            }
        }

        let seam_colors = [lighten(palette[3], 30), lighten(palette[4], 10), palette[1]];
        for (ci, path) in cracks.iter().enumerate() {
            let seam = seam_colors[ci % seam_colors.len()];
            for y in 0..height {
                let x = path[y];
                let next = if y + 1 < height { path[y + 1] } else { x };
                let ch = if next > x {
                    '╲'
                } else if next < x {
                    '╱'
                } else {
                    '│'
                };
                if x >= 0 && (x as usize) < width {
                    grid[y][x as usize] = Cell::new(ch, seam);
                }
                if rng.random::<f32>() < 0.09 {
                    let dir: i32 = if rng.random::<f32>() < 0.5 { -1 } else { 1 };
                    let len = rng.random_range(2..6i32);
                    for k in 1..=len {
                        let bx = x + dir * k;
                        let by = y + k as usize;
                        if bx >= 0 && (bx as usize) < width && by < height {
                            grid[by][bx as usize] = Cell::new(
                                if dir > 0 { '╲' } else { '╱' },
                                darken(seam, (k * 9) as u8),
                            );
                        }
                    }
                }
            }
        }

        let island_count = rng.random_range(5..9usize);
        let mut layers = Vec::new();
        let mut stops = Vec::new();
        for i in 0..island_count {
            let cx = rng.random_range(width / 8..(width * 7 / 8).max(width / 8 + 1));
            let cy = rng.random_range(height / 5..(height * 4 / 5).max(height / 5 + 1));
            let rx = rng.random_range(5..13usize);
            let ry = rng.random_range(3..7usize);
            let fill = match rng.random_range(0..9u32) {
                0 => FillGen::Tree(rng.random_range(0..12)),
                1 => FillGen::Flower(rng.random_range(0..5)),
                2 => FillGen::Fruit(rng.random_range(0..5)),
                3 => FillGen::Mask(
                    rng.random_range(2..5),
                    rng.random_range(0..MASK_STYLE_COUNT),
                ),
                4 => FillGen::AztecDiamond(rng.random_range(2..6)),
                5 => FillGen::Labyrinth,
                6 => FillGen::Noise(NoiseVariant::Grass),
                7 => FillGen::Tile(TileParams::randomized(&mut rng)),
                _ => FillGen::Concentric,
            };
            let mut p = palette;
            p[1] = shift_hue(palette[1], (i * 37) as f64);
            p[2] = shift_hue(palette[2], (i * 53) as f64);
            p[3] = shift_hue(lighten(palette[3], 10), (i * 71) as f64);
            layers.push(Layer {
                fill,
                mask: Some(Box::new(mask_ellipse(
                    cx as f32,
                    cy as f32,
                    rx as f32 * 2.0,
                    ry as f32,
                    0.75,
                ))),
                palette: p,
            });
            stops.push((cx, cy));
        }
        let scene = Scene { layers };
        let full = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        render_scene(&mut grid, &full, &scene, &mut rng);
        stops.sort_by_key(|p| p.0);
        draw_path_trail(&mut grid, &stops, lighten(palette[4], 10), &mut rng);

        for &(sx, sy) in &stops {
            if sx < width && sy < height {
                grid[sy][sx] = Cell::new('◆', lighten(palette[3], 25));
            }
        }

        for (ci, path) in cracks.iter().enumerate() {
            let seam = seam_colors[ci % seam_colors.len()];
            for y in 0..height {
                let x = path[y];
                let next = if y + 1 < height { path[y + 1] } else { x };
                let ch = if next > x {
                    '╲'
                } else if next < x {
                    '╱'
                } else {
                    '┃'
                };
                if x >= 0 && (x as usize) < width {
                    grid[y][x as usize] = Cell::new(ch, lighten(seam, 12));
                }
                for dx in [-1i32, 1i32] {
                    let gx = x + dx;
                    if gx >= 0
                        && (gx as usize) < width
                        && rng.random::<f32>() < 0.35
                        && grid[y][gx as usize].ch != '◆'
                    {
                        grid[y][gx as usize] = Cell::new('·', darken(seam, 25));
                    }
                }
            }
        }
    } else if mode == "eyes" {
        // eyes [density] [mutation] -- maximalist field of varied staring forms
        let density: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(42);
        let density = density.clamp(8, 120);
        let mutation: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(78);
        let mutation = mutation.clamp(0, 140);

        let bg = darken(palette[0], 10);
        let vein = darken(palette[2], 42);
        let lid_base = lighten(palette[1], 18);
        let sclera = lighten(palette[4], 2);
        let iris_base = lighten(palette[3], 28);
        let pupil = darken(palette[0], 4);
        let glare = lighten(palette[4], 25);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 13 + y * 19 + seed as usize * 7) % 67;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 => '°',
                    3 if mutation > 65 => '╎',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, vein)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let line_char = |dx: i32, dy: i32| {
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line =
            |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, ch: char, fg: Color| {
                let dx = (x1 - x0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let dy = -(y1 - y0).abs();
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;
                loop {
                    put(grid, x0, y0, ch, fg);
                    if x0 == x1 && y0 == y1 {
                        break;
                    }
                    let e2 = 2 * err;
                    if e2 >= dy {
                        err += dy;
                        x0 += sx;
                    }
                    if e2 <= dx {
                        err += dx;
                        y0 += sy;
                    }
                }
            };
        let draw_eye = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: i32,
                        ry: i32,
                        style: usize,
                        gaze_x: i32,
                        gaze_y: i32,
                        lid_color: Color,
                        iris_color: Color,
                        rng: &mut StdRng| {
            let rx = rx.max(2);
            let ry = ry.max(1);
            let iris_rx = (rx / 3).max(1);
            let iris_ry = (ry / 2).max(1);
            let pupil_rx = (iris_rx / 2).max(1);
            let pupil_ry = if style % 4 == 0 {
                iris_ry.max(2)
            } else {
                1.max(iris_ry / 2)
            };
            let blink_cut = if style % 9 == 3 { 0.28 } else { 1.0 };

            for dy in -ry - 1..=ry + 1 {
                for dx in -rx - 1..=rx + 1 {
                    let nx = dx as f32 / rx as f32;
                    let ny = dy as f32 / ry as f32;
                    let metric = nx * nx + ny * ny;
                    if metric > 1.28 || ny.abs() > blink_cut {
                        continue;
                    }
                    let x = cx + dx;
                    let y = cy + dy;
                    let edge = (metric - 1.0).abs();
                    if edge < 0.26 || dy.abs() == ry {
                        let ch = if dy < -ry / 3 {
                            if dx < -rx / 2 {
                                '╭'
                            } else if dx > rx / 2 {
                                '╮'
                            } else {
                                '─'
                            }
                        } else if dy > ry / 3 {
                            if dx < -rx / 2 {
                                '╰'
                            } else if dx > rx / 2 {
                                '╯'
                            } else {
                                '─'
                            }
                        } else if dx < 0 {
                            '╱'
                        } else if dx > 0 {
                            '╲'
                        } else {
                            '│'
                        };
                        put(grid, x, y, ch, lid_color);
                        continue;
                    }

                    let idy = dy - gaze_y;
                    let idx = dx - gaze_x;
                    let im = (idx as f32 / iris_rx as f32).powi(2)
                        + (idy as f32 / iris_ry as f32).powi(2);
                    if im <= 1.0 {
                        let pm = (idx as f32 / pupil_rx as f32).powi(2)
                            + (idy as f32 / pupil_ry as f32).powi(2);
                        if pm <= 1.0 {
                            let ch = match style % 8 {
                                0 => '┃',
                                1 => '●',
                                2 => '█',
                                3 => '◆',
                                4 => '◉',
                                5 => '◐',
                                6 => '◍',
                                _ => '◎',
                            };
                            put(grid, x, y, ch, pupil);
                        } else {
                            let ch = match (style
                                + dx.unsigned_abs() as usize
                                + dy.unsigned_abs() as usize)
                                % 7
                            {
                                0 => '◌',
                                1 => '○',
                                2 => '◍',
                                3 => '◐',
                                4 => '◑',
                                5 => '·',
                                _ => '•',
                            };
                            put(grid, x, y, ch, iris_color);
                        }
                    } else {
                        let ch = match (style
                            + dx.unsigned_abs() as usize * 2
                            + dy.unsigned_abs() as usize)
                            % 9
                        {
                            0 => '·',
                            1 => '∙',
                            2 if style % 5 == 0 => '╎',
                            3 if style % 7 == 0 => '◇',
                            _ => ' ',
                        };
                        put(
                            grid,
                            x,
                            y,
                            ch,
                            if ch == ' ' {
                                sclera
                            } else {
                                darken(sclera, 35)
                            },
                        );
                    }
                }
            }

            put(
                grid,
                cx + gaze_x - iris_rx / 2,
                cy + gaze_y - iris_ry,
                '˙',
                glare,
            );
            if style % 3 == 0 {
                put(
                    grid,
                    cx + gaze_x + iris_rx / 2,
                    cy + gaze_y + iris_ry,
                    '·',
                    glare,
                );
            }
            if style % 5 == 0 {
                put(grid, cx - rx, cy, '<', lid_color);
                put(grid, cx + rx, cy, '>', lid_color);
            }

            if style % 2 == 0 {
                let lash_count = (3 + mutation / 28).min(8);
                for i in 0..lash_count {
                    let t = if lash_count <= 1 {
                        0.5
                    } else {
                        i as f32 / (lash_count - 1) as f32
                    };
                    let lx = cx - rx + (t * rx as f32 * 2.0).round() as i32;
                    let ly = cy - ry + ((t - 0.5).abs() * 2.0).round() as i32;
                    let lean = rng.random_range(-2..=2);
                    let len = rng.random_range(1..=3 + (mutation / 50) as i32);
                    let tx = lx + lean;
                    let ty = ly - len;
                    draw_line(
                        grid,
                        lx,
                        ly,
                        tx,
                        ty,
                        line_char(tx - lx, ty - ly),
                        darken(lid_color, 8),
                    );
                }
            }
            if style % 6 == 1 {
                for side in [-1, 1] {
                    for k in 1..=3 {
                        put(
                            grid,
                            cx + side * (rx + k),
                            cy + (k % 2) - 1,
                            '·',
                            darken(iris_color, 20),
                        );
                    }
                }
            }
        };

        let mut row_y = 2i32;
        while row_y < height as i32 {
            let mut x = rng.random_range(-6..=3);
            while x < width as i32 + 6 {
                let rx = rng.random_range(3..=(7 + density / 22) as i32).min(12);
                let ry = rng.random_range(1..=3 + (mutation / 70) as i32).min(5);
                let style = rng.random_range(0..18usize);
                let gaze_x = rng.random_range(-(rx / 4).max(1)..=(rx / 4).max(1));
                let gaze_y = rng.random_range(-(ry / 3).max(0)..=(ry / 3).max(0));
                let lid = shift_hue(lid_base, rng.random_range(-45..=55) as f64);
                let iris = shift_hue(iris_base, rng.random_range(-120..=120) as f64);
                draw_eye(
                    &mut grid, x, row_y, rx, ry, style, gaze_x, gaze_y, lid, iris, &mut rng,
                );
                x += rng
                    .random_range(7i32..=13i32)
                    .saturating_sub(density as i32 / 24);
            }
            row_y += rng.random_range(3..=5);
        }

        let large_count = (3 + mutation / 25).min(8);
        for i in 0..large_count {
            let rx = rng.random_range(7..=(width / 4).max(9) as i32).min(22);
            let ry = rng.random_range(3..=(height / 4).max(4) as i32).min(8);
            let cx = rng.random_range(-(rx / 2)..=(width as i32 + rx / 2));
            let cy = rng.random_range(1..height as i32);
            let style = i + rng.random_range(0..24usize);
            let gaze_x = rng.random_range(-(rx / 3)..=(rx / 3));
            let gaze_y = rng.random_range(-(ry / 3)..=(ry / 3));
            let lid = shift_hue(lighten(lid_base, 14), rng.random_range(-80..=80) as f64);
            let iris = shift_hue(lighten(iris_base, 18), rng.random_range(-160..=160) as f64);
            draw_eye(
                &mut grid, cx, cy, rx, ry, style, gaze_x, gaze_y, lid, iris, &mut rng,
            );
        }

        let sigils = ['◉', '◎', '◌', '◍', '◐', '◑', '●', '•', '˙'];
        for _ in 0..density {
            let x = rng.random_range(0..width) as i32;
            let y = rng.random_range(0..height) as i32;
            if rng.random::<f32>() < mutation as f32 / 170.0 {
                put(
                    &mut grid,
                    x,
                    y,
                    sigils[rng.random_range(0..sigils.len())],
                    shift_hue(iris_base, rng.random_range(-180..=180) as f64),
                );
            }
        }
    } else if mode == "eyes2" {
        // eyes2 [count] [pupil-visible] -- anatomical eyes all staring at a focal lure
        let eye_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(6);
        let eye_count = eye_count.clamp(3, 20);
        let pupil_visible: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(80);
        let pupil_visible = pupil_visible.clamp(50, 100);

        let bg = darken(palette[0], 12);
        let lid_base = lighten(palette[1], 12);
        let sclera = lighten(palette[4], 4);
        let iris_base = lighten(palette[3], 18);
        let pupil = darken(palette[0], 2);
        let shadow = darken(palette[2], 48);
        let highlight = lighten(palette[4], 26);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 17 + y * 29 + seed as usize * 3) % 91;
                let ch = match n {
                    0 => '·',
                    1 if (x + y) % 3 == 0 => '∙',
                    2 if (x + seed as usize) % 11 == 0 => '°',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, shadow)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let draw_line =
            |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, ch: char, fg: Color| {
                let dx = (x1 - x0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let dy = -(y1 - y0).abs();
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;
                loop {
                    put(grid, x0, y0, ch, fg);
                    if x0 == x1 && y0 == y1 {
                        break;
                    }
                    let e2 = 2 * err;
                    if e2 >= dy {
                        err += dy;
                        x0 += sx;
                    }
                    if e2 <= dx {
                        err += dx;
                        y0 += sy;
                    }
                }
            };
        let draw_eye = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: i32,
                        ry: i32,
                        open_pct: usize,
                        gaze_x: i32,
                        gaze_y: i32,
                        slant: f32,
                        style: usize,
                        lid_color: Color,
                        iris_color: Color| {
            let rx = rx.max(5);
            let ry = ry.max(2);
            let open = (open_pct as f32 / 100.0).clamp(0.50, 1.0);
            let iris_rx = ((rx as f32 * 0.27).round() as i32).max(2);
            let iris_ry = ((ry as f32 * 0.72).round() as i32).max(2);
            let pupil_rx = ((iris_rx as f32 * 0.42).round() as i32).max(1);
            let pupil_ry = if style % 4 == 0 {
                iris_ry.max(2)
            } else {
                ((iris_ry as f32 * 0.62).round() as i32).max(1)
            };

            for dx in -rx - 2..=rx + 2 {
                let nx = dx as f32 / rx as f32;
                if nx.abs() > 1.06 {
                    continue;
                }
                let curve = (1.0 - nx.abs().powf(1.72)).max(0.0).powf(0.56);
                let top = (-ry as f32 * open * curve - nx * slant).round() as i32;
                let bottom = (ry as f32 * open * 0.84 * curve + nx * slant * 0.38).round() as i32;
                if bottom < top {
                    continue;
                }
                for dy in top..=bottom {
                    let x = cx + dx;
                    let y = cy + dy;
                    let on_top = dy == top;
                    let on_bottom = dy == bottom;
                    if on_top || on_bottom {
                        let edge = dx.abs() as f32 / rx as f32;
                        let ch = if dx <= -rx {
                            if dx < 0 { '<' } else { '>' }
                        } else if dx >= rx {
                            '>'
                        } else if edge < 0.82 {
                            '─'
                        } else if on_top {
                            if dx < 0 { '╱' } else { '╲' }
                        } else if dx < 0 {
                            '╲'
                        } else {
                            '╱'
                        };
                        put(grid, x, y, ch, lid_color);
                        continue;
                    }

                    let idx = dx - gaze_x;
                    let idy = dy - gaze_y;
                    let im = (idx as f32 / iris_rx as f32).powi(2)
                        + (idy as f32 / iris_ry as f32).powi(2);
                    if im <= 1.08 {
                        let pm = (idx as f32 / pupil_rx as f32).powi(2)
                            + (idy as f32 / pupil_ry as f32).powi(2);
                        if pm <= 0.56 {
                            let ch = match style % 6 {
                                0 => '│',
                                1 => '●',
                                2 => '◐',
                                3 => '◑',
                                4 => '◉',
                                _ => '┃',
                            };
                            put(grid, x, y, ch, pupil);
                        } else if im > 0.72 {
                            let ch = if (idx + idy + style as i32) % 3 == 0 {
                                '◌'
                            } else {
                                '○'
                            };
                            put(grid, x, y, ch, darken(iris_color, 8));
                        } else {
                            let ch = match (idx.abs() + idy.abs() + style as i32) % 7 {
                                0 => '╎',
                                1 | 2 | 3 => '·',
                                4 => '∙',
                                _ => '˙',
                            };
                            put(grid, x, y, ch, iris_color);
                        }
                    } else if (dx * 5 + dy * 7 + style as i32) % 37 == 0 {
                        put(grid, x, y, '·', darken(sclera, 45));
                    } else if (dx * 3 + dy * 11 + style as i32) % 11 == 0 {
                        put(grid, x, y, '·', darken(sclera, 24));
                    } else {
                        put(grid, x, y, ' ', sclera);
                    }
                }
            }

            put(
                grid,
                cx + gaze_x - iris_rx / 2,
                cy + gaze_y - iris_ry / 2,
                '˙',
                highlight,
            );
            if style % 5 == 0 {
                let lid_y = cy + gaze_y - iris_ry / 2;
                draw_line(
                    grid,
                    cx - iris_rx,
                    lid_y,
                    cx + iris_rx,
                    lid_y,
                    '─',
                    darken(lid_color, 2),
                );
            }
        };

        let focus_x = (width as i32 / 2
            + rng.random_range(-(width as i32 / 12)..=(width as i32 / 12)))
        .clamp(6, width as i32 - 7);
        let focus_y = ((height as f32 * 0.72).round() as i32
            + rng.random_range(-(height as i32 / 18)..=(height as i32 / 18)))
        .clamp(8, height as i32 - 5);
        let lure_color = shift_hue(lighten(iris_base, 30), 55.0);
        for dy in -3i32..=3i32 {
            for dx in -6i32..=6i32 {
                let metric = (dx as f32 / 6.0).powi(2) + (dy as f32 / 3.0).powi(2);
                if metric <= 1.0 && (dx.abs() + dy.abs()) % 2 == 0 {
                    put(
                        &mut grid,
                        focus_x + dx,
                        focus_y + dy,
                        '·',
                        darken(lure_color, 22),
                    );
                }
            }
        }
        draw_line(
            &mut grid,
            focus_x,
            focus_y - 3,
            focus_x,
            focus_y + 2,
            '│',
            darken(lure_color, 8),
        );
        draw_line(
            &mut grid,
            focus_x - 3,
            focus_y,
            focus_x + 3,
            focus_y,
            '─',
            darken(lure_color, 8),
        );
        put(&mut grid, focus_x, focus_y, '◆', lighten(lure_color, 12));
        put(&mut grid, focus_x, focus_y - 2, '◇', lighten(highlight, 4));
        put(
            &mut grid,
            focus_x - 2,
            focus_y + 2,
            '╲',
            darken(lure_color, 2),
        );
        put(
            &mut grid,
            focus_x + 2,
            focus_y + 2,
            '╱',
            darken(lure_color, 2),
        );

        let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| {
            let dx = (focus_x - ex) as f32;
            let dy = (focus_y - ey) as f32;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let gx = ((dx / dist) * (rx as f32 * 0.24)).round() as i32;
            let gy = ((dy / dist) * (ry as f32 * 0.42)).round() as i32;
            let gx = gx.clamp(-(rx / 4).max(1), (rx / 4).max(1));
            let gy = gy.clamp(-(ry / 2).max(1), (ry / 2).max(1));
            let slant = (dx / dist * 1.35).clamp(-1.2, 1.2);
            (gx, gy, slant)
        };
        let draw_dotted_line =
            |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
                let dx = (x1 - x0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let dy = -(y1 - y0).abs();
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;
                let mut step = 0usize;
                loop {
                    if step % 4 == 0
                        && x0 >= 0
                        && y0 >= 0
                        && (x0 as usize) < width
                        && (y0 as usize) < height
                        && grid[y0 as usize][x0 as usize].ch == ' '
                    {
                        put(grid, x0, y0, '·', fg);
                    }
                    if x0 == x1 && y0 == y1 {
                        break;
                    }
                    let e2 = 2 * err;
                    if e2 >= dy {
                        err += dy;
                        x0 += sx;
                    }
                    if e2 <= dx {
                        err += dx;
                        y0 += sy;
                    }
                    step += 1;
                }
            };

        let mut eye_specs: Vec<(i32, i32, i32, i32, usize, usize)> = Vec::new();
        for i in 0..eye_count {
            let t = (i as f32 + 0.5) / eye_count as f32;
            let angle =
                std::f32::consts::PI + t * std::f32::consts::PI + rng.random_range(-0.20..0.20);
            let arc_rx = width as f32 * rng.random_range(0.34..0.52);
            let arc_ry = height as f32 * rng.random_range(0.28..0.52);
            let mut ex = (focus_x as f32 + angle.cos() * arc_rx).round() as i32;
            let mut ey = (focus_y as f32 + angle.sin() * arc_ry).round() as i32;
            let mut rx = rng.random_range(7..=13);
            let mut ry = rng.random_range(3..=6);
            if i == eye_count / 2 {
                rx = ((width as f32 * 0.18).round() as i32).clamp(12, 20);
                ry = ((height as f32 * 0.20).round() as i32).clamp(4, 7);
                ex = (width as i32 / 2 + rng.random_range(-4..=4))
                    .clamp(rx + 2, width as i32 - rx - 3);
                ey = (focus_y - (height as i32 / 3).max(6) + rng.random_range(-2..=2))
                    .clamp(ry + 3, height as i32 - ry - 4);
            }
            ex = ex.clamp(-rx / 2, width as i32 + rx / 2);
            ey = ey.clamp(ry + 2, height as i32 - ry - 3);
            let visible =
                (pupil_visible as i32 + rng.random_range(-8..=14)).clamp(50, 100) as usize;
            eye_specs.push((ex, ey, rx, ry, visible, i));
        }

        for &(ex, ey, rx, ry, _, i) in &eye_specs {
            let (gx, gy, _) = gaze_for(ex, ey, rx, ry);
            let iris_x = ex + gx;
            let iris_y = ey + gy;
            draw_dotted_line(
                &mut grid,
                iris_x,
                iris_y,
                focus_x,
                focus_y,
                darken(shift_hue(iris_base, i as f64 * 23.0), 35),
            );
        }
        for &(ex, ey, rx, ry, visible, i) in &eye_specs {
            let (gaze_x, gaze_y, slant) = gaze_for(ex, ey, rx, ry);
            let lid = shift_hue(lid_base, rng.random_range(-34..=42) as f64);
            let iris = shift_hue(iris_base, rng.random_range(-120..=120) as f64);
            draw_eye(
                &mut grid, ex, ey, rx, ry, visible, gaze_x, gaze_y, slant, i, lid, iris,
            );
        }
    } else if mode == "eyes3" {
        grid = draw_eyes3(grid, width, height, seed, palette, rng, t_anim, &args);
    } else if mode == "fullmetal-eyes" {
        grid = draw_fullmetal_eyes(grid, width, height, seed, palette, rng, t_anim, &args);
    } else if mode == "fullmetal-eyes2" {
        grid = draw_fullmetal_eyes2(grid, width, height, seed, palette, rng, t_anim, &args);
    } else if mode == "fullmetal-alchemist" {
        // fullmetal-alchemist [rings] [glyphs] -- original generative alchemical sealwork
        let ring_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
        let ring_count = ring_count.clamp(2, 9);
        let glyph_count: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(64);
        let glyph_count = glyph_count.clamp(0, 180);
        let chord_count: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(28);
        let chord_count = chord_count.clamp(0, 96);

        let bg = darken(palette[0], 8);
        let chalk = lighten(palette[4], 8);
        let gold = lighten(palette[1], 32);
        let ember = lighten(palette[3], 24);
        let shadow = darken(palette[2], 36);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 17 + y * 31 + seed as usize) % 89;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 => '°',
                    3 => '\'',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, shadow)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let draw_ellipse = |grid: &mut Grid,
                            cx: i32,
                            cy: i32,
                            rx: f32,
                            ry: f32,
                            fg: Color,
                            phase: f32,
                            dotted: bool| {
            let samples = ((rx + ry) * 16.0).max(96.0) as usize;
            let mut prev: Option<(i32, i32)> = None;
            for i in 0..=samples {
                if dotted && i % 5 == 3 {
                    prev = None;
                    continue;
                }
                let a = phase + i as f32 / samples as f32 * std::f32::consts::TAU;
                let p = point_on(cx, cy, rx, ry, a);
                if let Some(q) = prev {
                    let ch = stroke_char(q.0, q.1, p.0, p.1);
                    draw_line(grid, q.0, q.1, p.0, p.1, fg);
                    put(grid, p.0, p.1, ch, fg);
                } else {
                    put(grid, p.0, p.1, '·', fg);
                }
                prev = Some(p);
            }
        };
        let draw_poly = |grid: &mut Grid,
                         cx: i32,
                         cy: i32,
                         rx: f32,
                         ry: f32,
                         sides: usize,
                         phase: f32,
                         skip: usize,
                         fg: Color| {
            let mut pts = Vec::new();
            for i in 0..sides {
                let a = phase + i as f32 / sides as f32 * std::f32::consts::TAU;
                pts.push(point_on(cx, cy, rx, ry, a));
            }
            let skip = skip.max(1).min(sides - 1);
            for i in 0..sides {
                let j = (i + skip) % sides;
                draw_line(grid, pts[i].0, pts[i].1, pts[j].0, pts[j].1, fg);
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 4.0).max(8.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(4.0);

        for i in 0..ring_count {
            let t = i as f32 / ring_count.max(1) as f32;
            let rx = max_rx * (1.0 - t * 0.68);
            let ry = max_ry * (1.0 - t * 0.68);
            let fg = if i % 2 == 0 {
                shift_hue(gold, i as f64 * 21.0)
            } else {
                shift_hue(chalk, i as f64 * -18.0)
            };
            draw_ellipse(&mut grid, cx, cy, rx, ry, fg, i as f32 * 0.11, i % 3 == 2);
        }

        let phase = rng.random::<f32>() * std::f32::consts::TAU;
        draw_poly(
            &mut grid,
            cx,
            cy,
            max_rx * 0.84,
            max_ry * 0.84,
            3,
            phase,
            1,
            ember,
        );
        draw_poly(
            &mut grid,
            cx,
            cy,
            max_rx * 0.72,
            max_ry * 0.72,
            6,
            phase + std::f32::consts::PI / 6.0,
            2,
            lighten(chalk, 8),
        );
        draw_poly(
            &mut grid,
            cx,
            cy,
            max_rx * 0.54,
            max_ry * 0.54,
            5 + (seed as usize % 3),
            phase * 0.5,
            2,
            shift_hue(gold, 70.0),
        );

        for _ in 0..chord_count {
            let a = rng.random::<f32>() * std::f32::consts::TAU;
            let b = a + rng.random_range(2.0..5.2) + rng.random_range(-0.35..0.35);
            let r = rng.random_range(0.34..0.96);
            let p1 = point_on(cx, cy, max_rx * r, max_ry * r, a);
            let p2 = point_on(cx, cy, max_rx * r, max_ry * r, b);
            let color = if rng.random::<f32>() < 0.55 {
                darken(chalk, rng.random_range(0..38))
            } else {
                darken(ember, rng.random_range(0..32))
            };
            draw_line(&mut grid, p1.0, p1.1, p2.0, p2.1, color);
        }

        let runes = [
            '△', '▽', '□', '◇', '○', '☉', '☽', '☿', '♄', '♃', '♁', '✶', '✦', '✧', '+', '×', '≡',
            '∴', '∵',
        ];
        for i in 0..glyph_count {
            let lane = match i % 4 {
                0 => 0.96,
                1 => 0.81,
                2 => 0.59,
                _ => rng.random_range(0.30..0.92),
            };
            let a = i as f32 / glyph_count.max(1) as f32 * std::f32::consts::TAU
                + rng.random_range(-0.045..0.045)
                + phase * 0.13;
            let (x, y) = point_on(cx, cy, max_rx * lane, max_ry * lane, a);
            let glyph = runes[rng.random_range(0..runes.len())];
            put(
                &mut grid,
                x,
                y,
                glyph,
                shift_hue(lighten(gold, 8), rng.random_range(-80..=90) as f64),
            );
        }

        let walker_count = (ring_count + glyph_count / 30).clamp(4, 14);
        for w in 0..walker_count {
            let mut angle = phase + w as f32 * 1.37;
            let mut radius = rng.random_range(0.16..0.42);
            let mut prev = point_on(cx, cy, max_rx * radius, max_ry * radius, angle);
            let steps = rng.random_range(8..22);
            for s in 0..steps {
                angle += rng.random_range(-0.55..0.72);
                radius = (radius + rng.random_range(-0.04..0.09)).clamp(0.10, 0.74);
                let next = point_on(cx, cy, max_rx * radius, max_ry * radius, angle);
                let color = if s % 3 == 0 { ember } else { darken(chalk, 18) };
                draw_line(&mut grid, prev.0, prev.1, next.0, next.1, color);
                if rng.random::<f32>() < 0.42 {
                    put(
                        &mut grid,
                        next.0,
                        next.1,
                        runes[rng.random_range(0..runes.len())],
                        gold,
                    );
                }
                prev = next;
            }
        }

        let anchors = [
            (-std::f32::consts::FRAC_PI_2, '△'),
            (0.0, '☉'),
            (std::f32::consts::FRAC_PI_2, '▽'),
            (std::f32::consts::PI, '□'),
        ];
        for &(a, ch) in &anchors {
            let outer = point_on(cx, cy, max_rx + 1.0, max_ry + 1.0, a);
            let inner = point_on(cx, cy, max_rx * 0.88, max_ry * 0.88, a);
            draw_line(&mut grid, inner.0, inner.1, outer.0, outer.1, gold);
            put(&mut grid, outer.0, outer.1, ch, lighten(chalk, 15));
        }

        for dy in -2i32..=2i32 {
            for dx in -4i32..=4i32 {
                let metric = (dx as f32 / 4.0).powi(2) + (dy as f32 / 2.0).powi(2);
                if metric <= 1.0 {
                    let ch = if dx == 0 && dy == 0 {
                        '☉'
                    } else if dx.abs() == dy.abs() {
                        '╳'
                    } else if dy == 0 {
                        '═'
                    } else if dx == 0 {
                        '║'
                    } else {
                        '·'
                    };
                    put(&mut grid, cx + dx, cy + dy, ch, lighten(ember, 18));
                }
            }
        }
    } else if mode == "fullmetal-alchemist2" {
        // fullmetal-alchemist2 [nodes=0] [runes] [fractures] -- node-first ritual geometry
        let node_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let node_count = if node_arg == 0 {
            5 + ((seed as usize * 37 + 11) % 7)
        } else {
            node_arg.clamp(5, 14)
        };
        let rune_count: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(72);
        let rune_count = rune_count.clamp(16, 240);
        let fracture_count: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(7);
        let fracture_count = fracture_count.clamp(0, 80);

        let bg = darken(palette[0], 10);
        let chalk = lighten(palette[4], 10);
        let gold = lighten(palette[1], 34);
        let ether = shift_hue(lighten(palette[3], 32), 42.0);
        let blood = shift_hue(lighten(palette[2], 42), -30.0);
        let hush = darken(palette[2], 55);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 23 + y * 41 + seed as usize * 5) % 113;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 => '°',
                    3 => '\'',
                    4 => '`',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, hush)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let blank = |grid: &mut Grid, x: i32, y: i32| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::blank();
            }
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let ch = stroke_char(x0, y0, x1, y1);
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        puncture: usize| {
            let span = (end - start).abs().max(0.05);
            let samples = ((rx + ry) * span * 3.4).max(12.0) as usize;
            let mut prev: Option<(i32, i32)> = None;
            for i in 0..=samples {
                if puncture > 0 && i % puncture == puncture - 1 {
                    prev = None;
                    continue;
                }
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if let Some(q) = prev {
                    draw_line(grid, q.0, q.1, p.0, p.1, fg);
                } else {
                    put(grid, p.0, p.1, '·', fg);
                }
                prev = Some(p);
            }
        };
        let draw_poly = |grid: &mut Grid, pts: &[(i32, i32)], skip: usize, fg: Color| {
            if pts.len() < 2 {
                return;
            }
            let skip = skip.max(1).min(pts.len() - 1);
            for i in 0..pts.len() {
                let j = (i + skip) % pts.len();
                draw_line(grid, pts[i].0, pts[i].1, pts[j].0, pts[j].1, fg);
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 3.0).max(10.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let phase = -std::f32::consts::FRAC_PI_2 + rng.random_range(-0.08..0.08);

        for y in 0..height {
            for x in 0..width {
                let dx = (x as i32 - cx) as f32 / (max_rx * 0.95);
                let dy = (y as i32 - cy) as f32 / (max_ry * 0.95);
                let metric = dx * dx + dy * dy;
                if metric < 0.91 {
                    blank(&mut grid, x as i32, y as i32);
                } else if metric < 1.08 && (x + y + seed as usize) % 5 == 0 {
                    grid[y][x] = Cell::new('·', darken(chalk, 55));
                }
            }
        }

        for band in 0..3 {
            let rx = max_rx - band as f32 * 3.2;
            let ry = max_ry - band as f32 * 1.25;
            let color = match band {
                0 => ether,
                1 => chalk,
                _ => gold,
            };
            let gap = 0.16 + band as f32 * 0.035;
            for node in 0..node_count {
                let a0 = phase + node as f32 * std::f32::consts::TAU / node_count as f32 + gap;
                let a1 =
                    phase + (node + 1) as f32 * std::f32::consts::TAU / node_count as f32 - gap;
                draw_arc(
                    &mut grid,
                    cx,
                    cy,
                    rx,
                    ry,
                    a0,
                    a1,
                    darken(color, (band * 10) as u8),
                    if band == 1 { 7 } else { 0 },
                );
            }
        }

        let mut nodes = Vec::new();
        let mut inner_nodes = Vec::new();
        let node_glyphs = ['△', '▽', '□', '◇', '☉', '☽', '☿', '♄', '♃', '♁', '✦', '∴'];
        for i in 0..node_count {
            let base = phase + i as f32 * std::f32::consts::TAU / node_count as f32;
            let a = base + rng.random_range(-0.17..0.17);
            let outer_scale = rng.random_range(0.82..0.97);
            let inner_scale = rng.random_range(0.53..0.68);
            let outer = point_on(cx, cy, max_rx * outer_scale, max_ry * outer_scale, a);
            let inner = point_on(cx, cy, max_rx * inner_scale, max_ry * inner_scale, a);
            nodes.push(outer);
            inner_nodes.push(inner);
            draw_line(
                &mut grid,
                inner.0,
                inner.1,
                outer.0,
                outer.1,
                darken(gold, 6),
            );
            draw_line(
                &mut grid,
                outer.0 - 2,
                outer.1,
                outer.0 + 2,
                outer.1,
                lighten(chalk, 8),
            );
            draw_line(
                &mut grid,
                outer.0,
                outer.1 - 1,
                outer.0,
                outer.1 + 1,
                lighten(chalk, 8),
            );
            put(
                &mut grid,
                outer.0,
                outer.1,
                node_glyphs[i % node_glyphs.len()],
                ether,
            );
            put(&mut grid, outer.0 - 3, outer.1, '╴', darken(ether, 20));
            put(&mut grid, outer.0 + 3, outer.1, '╶', darken(ether, 20));
        }

        let mut core = Vec::new();
        let core_count = 5 + (seed as usize % 2);
        for i in 0..core_count {
            let a = phase
                + std::f32::consts::PI / core_count as f32
                + i as f32 * std::f32::consts::TAU / core_count as f32;
            core.push(point_on(cx, cy, max_rx * 0.24, max_ry * 0.24, a));
        }
        draw_poly(&mut grid, &core, 1, darken(chalk, 4));

        let runes = [
            '△', '▽', '□', '◇', '○', '☉', '☽', '☿', '♄', '♃', '♁', '✶', '✦', '✧', '+', '×', '≡',
            '∴', '∵', '⌬', '⊕', '⊗',
        ];
        for i in 0..rune_count {
            let lane = match i % 5 {
                0 => 0.93,
                1 => 0.84,
                2 => 0.76,
                3 => 0.68,
                _ => rng.random_range(0.66..0.92),
            };
            let jitter = rng.random_range(-0.035..0.035);
            let a = phase
                + i as f32 / rune_count as f32 * std::f32::consts::TAU
                + jitter
                + (i % node_count) as f32 * 0.006;
            let p = point_on(cx, cy, max_rx * lane, max_ry * lane, a);
            let color = match i % 4 {
                0 => gold,
                1 => ether,
                2 => chalk,
                _ => blood,
            };
            put(
                &mut grid,
                p.0,
                p.1,
                runes[(i + rng.random_range(0..runes.len())) % runes.len()],
                darken(color, rng.random_range(0..24)),
            );
        }

        for _ in 0..fracture_count {
            let node = rng.random_range(0..node_count);
            let from = nodes[node];
            let target_angle = phase
                + (node as f32 + rng.random_range(1.5..4.5)) * std::f32::consts::TAU
                    / node_count as f32;
            let target_radius = rng.random_range(0.68..0.94);
            let to = point_on(
                cx,
                cy,
                max_rx * target_radius,
                max_ry * target_radius,
                target_angle,
            );
            let mid = (
                ((from.0 + to.0) / 2) + rng.random_range(-5..=5),
                ((from.1 + to.1) / 2) + rng.random_range(-2..=2),
            );
            let color = if rng.random::<f32>() < 0.45 {
                darken(blood, rng.random_range(0..34))
            } else {
                darken(ether, rng.random_range(0..28))
            };
            draw_line(&mut grid, from.0, from.1, mid.0, mid.1, color);
            draw_line(&mut grid, mid.0, mid.1, to.0, to.1, color);
            if rng.random::<f32>() < 0.38 {
                put(&mut grid, mid.0, mid.1, '✦', lighten(color, 18));
            }
        }

        let core_clear_rx = (max_rx * 0.18).round() as i32;
        let core_clear_ry = (max_ry * 0.18).round() as i32;
        for dy in -core_clear_ry..=core_clear_ry {
            for dx in -core_clear_rx..=core_clear_rx {
                let metric = (dx as f32 / core_clear_rx.max(1) as f32).powi(2)
                    + (dy as f32 / core_clear_ry.max(1) as f32).powi(2);
                if metric <= 1.0 {
                    blank(&mut grid, cx + dx, cy + dy);
                }
            }
        }
        draw_arc(
            &mut grid,
            cx,
            cy,
            max_rx * 0.16,
            max_ry * 0.14,
            phase,
            phase + std::f32::consts::TAU,
            lighten(gold, 6),
            0,
        );
        draw_arc(
            &mut grid,
            cx,
            cy,
            max_rx * 0.10,
            max_ry * 0.09,
            phase,
            phase + std::f32::consts::TAU,
            darken(chalk, 6),
            5,
        );

        let vertical = (max_ry * 0.24) as i32;
        let horizontal = (max_rx * 0.10) as i32;
        draw_line(
            &mut grid,
            cx,
            cy - vertical,
            cx,
            cy + vertical,
            lighten(chalk, 10),
        );
        draw_line(
            &mut grid,
            cx - horizontal,
            cy,
            cx + horizontal,
            cy,
            lighten(chalk, 10),
        );
        for dy in -2i32..=2i32 {
            for dx in -5i32..=5i32 {
                let metric = (dx as f32 / 5.0).powi(2) + (dy as f32 / 2.0).powi(2);
                if metric <= 1.0 {
                    let ch = if dx == 0 && dy == 0 {
                        '⊕'
                    } else if dx.abs() == dy.abs() * 2 {
                        '╳'
                    } else if dy == 0 {
                        '═'
                    } else if dx == 0 {
                        '║'
                    } else {
                        '·'
                    };
                    put(&mut grid, cx + dx, cy + dy, ch, lighten(blood, 18));
                }
            }
        }
        for (i, &(nx, ny)) in nodes.iter().enumerate() {
            for dy in -1i32..=1i32 {
                for dx in -2i32..=2i32 {
                    blank(&mut grid, nx + dx, ny + dy);
                }
            }
            put(&mut grid, nx - 2, ny - 1, '╭', lighten(chalk, 8));
            put(&mut grid, nx + 2, ny - 1, '╮', lighten(chalk, 8));
            put(&mut grid, nx - 2, ny + 1, '╰', lighten(chalk, 8));
            put(&mut grid, nx + 2, ny + 1, '╯', lighten(chalk, 8));
            put(&mut grid, nx - 1, ny - 1, '─', lighten(chalk, 8));
            put(&mut grid, nx, ny - 1, '─', lighten(chalk, 8));
            put(&mut grid, nx + 1, ny - 1, '─', lighten(chalk, 8));
            put(&mut grid, nx - 1, ny + 1, '─', lighten(chalk, 8));
            put(&mut grid, nx, ny + 1, '─', lighten(chalk, 8));
            put(&mut grid, nx + 1, ny + 1, '─', lighten(chalk, 8));
            put(&mut grid, nx - 2, ny, '│', lighten(chalk, 8));
            put(&mut grid, nx + 2, ny, '│', lighten(chalk, 8));
            put(&mut grid, nx, ny, node_glyphs[i % node_glyphs.len()], ether);
        }
    } else if mode == "fa3" || mode == "fullmetal-alchemist3" {
        // fa3 [paths=0] [rings] [nodes=0] -- ornamented ray paths with inner circles and node stations
        let path_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let path_count = if path_arg == 0 {
            8 + ((seed as usize * 19 + 3) % 8)
        } else {
            path_arg.clamp(5, 22)
        };
        let inner_count: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(4);
        let inner_count = inner_count.clamp(2, 7);
        let node_arg: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let base_nodes = if node_arg == 0 {
            4 + ((seed as usize * 31 + 7) % 7)
        } else {
            node_arg.clamp(3, 14)
        };

        let bg = darken(palette[0], 12);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 32);
        let ether = shift_hue(lighten(palette[3], 36), 35.0);
        let rose = shift_hue(lighten(palette[2], 42), -38.0);
        let hush = darken(palette[2], 66);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 29 + y * 37 + seed as usize * 11) % 173;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 if (x + y) % 5 == 0 => '°',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, hush)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let blank = |grid: &mut Grid, x: i32, y: i32| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::blank();
            }
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        gap: usize| {
            let span = (end - start).abs().max(0.05);
            let samples = ((rx + ry) * span * 3.6).max(18.0) as usize;
            let mut prev: Option<(i32, i32)> = None;
            for i in 0..=samples {
                if gap > 0 && i % gap == gap - 1 {
                    prev = None;
                    continue;
                }
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if let Some(q) = prev {
                    draw_line(grid, q.0, q.1, p.0, p.1, fg);
                } else {
                    put(grid, p.0, p.1, '·', fg);
                }
                prev = Some(p);
            }
        };
        let draw_shape = |grid: &mut Grid, x: i32, y: i32, kind: usize, fg: Color| match kind % 7 {
            0 => {
                put(grid, x, y, '◇', fg);
                put(grid, x - 1, y, '╴', darken(fg, 18));
                put(grid, x + 1, y, '╶', darken(fg, 18));
            }
            1 => {
                for dx in -1..=1 {
                    put(grid, x + dx, y - 1, '─', fg);
                    put(grid, x + dx, y + 1, '─', fg);
                }
                put(grid, x - 2, y, '│', fg);
                put(grid, x + 2, y, '│', fg);
                put(grid, x, y, '□', lighten(fg, 10));
            }
            2 => {
                put(grid, x, y - 1, '△', lighten(fg, 8));
                put(grid, x - 1, y, '╱', fg);
                put(grid, x + 1, y, '╲', fg);
                put(grid, x, y + 1, '─', darken(fg, 12));
            }
            3 => {
                put(grid, x, y - 1, '○', fg);
                put(grid, x - 1, y, '◌', darken(fg, 10));
                put(grid, x, y, '☉', lighten(fg, 10));
                put(grid, x + 1, y, '◌', darken(fg, 10));
                put(grid, x, y + 1, '○', fg);
            }
            4 => {
                put(grid, x, y, '⊕', lighten(fg, 12));
                put(grid, x - 1, y, '─', fg);
                put(grid, x + 1, y, '─', fg);
                put(grid, x, y - 1, '│', fg);
                put(grid, x, y + 1, '│', fg);
            }
            5 => {
                put(grid, x, y, '⌬', lighten(fg, 8));
                put(grid, x - 1, y - 1, '╲', fg);
                put(grid, x + 1, y - 1, '╱', fg);
                put(grid, x - 1, y + 1, '╱', fg);
                put(grid, x + 1, y + 1, '╲', fg);
            }
            _ => {
                put(grid, x, y, '✦', lighten(fg, 14));
                put(grid, x - 1, y, '·', fg);
                put(grid, x + 1, y, '·', fg);
                put(grid, x, y - 1, '·', fg);
                put(grid, x, y + 1, '·', fg);
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 3.0).max(10.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let phase = -std::f32::consts::FRAC_PI_2 + rng.random_range(-0.10..0.10);

        for y in 0..height {
            for x in 0..width {
                let dx = (x as i32 - cx) as f32 / (max_rx * 0.96);
                let dy = (y as i32 - cy) as f32 / (max_ry * 0.96);
                let metric = dx * dx + dy * dy;
                if metric < 0.88 {
                    blank(&mut grid, x as i32, y as i32);
                } else if metric < 1.07 && (x + y + seed as usize) % 4 == 0 {
                    grid[y][x] = Cell::new('·', darken(chalk, 55));
                }
            }
        }

        for band in 0..4 {
            let rx = max_rx - band as f32 * 2.6;
            let ry = max_ry - band as f32 * 1.05;
            let fg = match band {
                0 => ether,
                1 => chalk,
                2 => gold,
                _ => darken(rose, 4),
            };
            let gap = 0.08 + band as f32 * 0.035;
            let segments = path_count.max(6);
            for seg in 0..segments {
                let a0 = phase + seg as f32 * std::f32::consts::TAU / segments as f32 + gap;
                let a1 = phase + (seg + 1) as f32 * std::f32::consts::TAU / segments as f32 - gap;
                draw_arc(
                    &mut grid,
                    cx,
                    cy,
                    rx,
                    ry,
                    a0,
                    a1,
                    darken(fg, (band * 7) as u8),
                    if band == 2 { 6 } else { 0 },
                );
            }
        }

        let mut ring_specs = Vec::new();
        for r in 0..inner_count {
            let t = (r + 1) as f32 / (inner_count + 1) as f32;
            let scale = 0.18 + t * 0.60 + rng.random_range(-0.025..0.025);
            let node_count = (base_nodes + r + (seed as usize % 3)).clamp(3, 16);
            let ring_phase = phase + r as f32 * 0.41 + rng.random_range(-0.12..0.12);
            let fg = match r % 4 {
                0 => gold,
                1 => ether,
                2 => chalk,
                _ => rose,
            };
            ring_specs.push((scale, node_count, ring_phase, fg));
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * scale,
                max_ry * scale,
                ring_phase,
                ring_phase + std::f32::consts::TAU,
                darken(fg, 8),
                if r % 2 == 0 { 8 } else { 0 },
            );
            for n in 0..node_count {
                let a = ring_phase + n as f32 * std::f32::consts::TAU / node_count as f32;
                let p = point_on(cx, cy, max_rx * scale, max_ry * scale, a);
                draw_shape(&mut grid, p.0, p.1, (n + r * 3) % 7, fg);
            }
        }

        for path in 0..path_count {
            let base_a = phase
                + path as f32 * std::f32::consts::TAU / path_count as f32
                + rng.random_range(-0.13..0.13);
            let mut prev = point_on(cx, cy, max_rx * 0.08, max_ry * 0.08, base_a);
            let path_color = match path % 4 {
                0 => ether,
                1 => gold,
                2 => chalk,
                _ => rose,
            };
            for (ri, &(scale, _, ring_phase, _)) in ring_specs.iter().enumerate() {
                let a = base_a + (ring_phase - phase) * 0.18 + (ri as f32 * 0.11).sin() * 0.16;
                let p = point_on(cx, cy, max_rx * scale, max_ry * scale, a);
                draw_line(&mut grid, prev.0, prev.1, p.0, p.1, darken(path_color, 10));
                draw_shape(
                    &mut grid,
                    p.0,
                    p.1,
                    path + ri + (seed as usize % 5),
                    shift_hue(lighten(path_color, 6), (ri * 22) as f64),
                );

                let bead_count = 1 + ((path + ri + seed as usize) % 3);
                for bead in 1..=bead_count {
                    let f = bead as f32 / (bead_count + 1) as f32;
                    let bx = (prev.0 as f32 + (p.0 - prev.0) as f32 * f).round() as i32;
                    let by = (prev.1 as f32 + (p.1 - prev.1) as f32 * f).round() as i32;
                    let bead_ch = ['○', '□', '◇', '△', '☉', '⊕'][(path + ri + bead) % 6];
                    put(&mut grid, bx, by, bead_ch, darken(path_color, 4));
                }
                prev = p;
            }
            let outer = point_on(
                cx,
                cy,
                max_rx * 0.96,
                max_ry * 0.96,
                base_a + rng.random_range(-0.05..0.05),
            );
            draw_line(
                &mut grid,
                prev.0,
                prev.1,
                outer.0,
                outer.1,
                darken(path_color, 8),
            );
            draw_shape(
                &mut grid,
                outer.0,
                outer.1,
                path + 3,
                lighten(path_color, 8),
            );
        }

        let core_rx = (max_rx * 0.17).round() as i32;
        let core_ry = (max_ry * 0.17).round() as i32;
        for dy in -core_ry..=core_ry {
            for dx in -core_rx..=core_rx {
                let metric = (dx as f32 / core_rx.max(1) as f32).powi(2)
                    + (dy as f32 / core_ry.max(1) as f32).powi(2);
                if metric <= 1.0 {
                    blank(&mut grid, cx + dx, cy + dy);
                }
            }
        }
        for r in 0..3 {
            let scale = 0.06 + r as f32 * 0.045;
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * scale,
                max_ry * scale,
                phase,
                phase + std::f32::consts::TAU,
                if r == 1 { gold } else { chalk },
                if r == 2 { 5 } else { 0 },
            );
        }
        let core_nodes = 6 + (seed as usize % 3);
        for n in 0..core_nodes {
            let a = phase + n as f32 * std::f32::consts::TAU / core_nodes as f32;
            let p = point_on(cx, cy, max_rx * 0.13, max_ry * 0.13, a);
            draw_shape(
                &mut grid,
                p.0,
                p.1,
                n + 4,
                if n % 2 == 0 { gold } else { ether },
            );
        }
        put(&mut grid, cx, cy, '⊙', lighten(rose, 16));
    } else if mode == "fa4" || mode == "fullmetal-alchemist4" {
        // fa4 [paths=0] [rings] [nodes=0] [ornaments] [stations=0] -- airy curved ritual lattice
        let path_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let path_count = if path_arg == 0 {
            0
        } else {
            path_arg.clamp(1, 6)
        };
        let ring_count: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(2);
        let ring_count = ring_count.clamp(2, 5);
        let node_arg: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let base_nodes = if node_arg == 0 {
            3 + ((seed as usize * 41 + 9) % 3)
        } else {
            node_arg.clamp(3, 8)
        };
        let ornament_step: usize = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(32);
        let ornament_step = ornament_step.clamp(18, 48);
        let station_arg: usize = args.get(8).and_then(|s| s.parse().ok()).unwrap_or(0);
        let station_base = if station_arg == 0 {
            6
        } else {
            station_arg.clamp(4, 16)
        };

        let bg = darken(palette[0], 13);
        let chalk = lighten(palette[4], 14);
        let gold = lighten(palette[1], 34);
        let ether = shift_hue(lighten(palette[3], 38), 32.0);
        let rose = shift_hue(lighten(palette[2], 44), -36.0);
        let verdigris = shift_hue(lighten(palette[1], 28), 92.0);
        let hush = darken(palette[2], 70);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 31 + y * 43 + seed as usize * 13) % 353;
                let ch = match n {
                    0 => '·',
                    1 if (x + y + seed as usize) % 9 == 0 => '°',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, hush)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let blank = |grid: &mut Grid, x: i32, y: i32| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::blank();
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let curve_char = |prev: (i32, i32), here: (i32, i32), next: (i32, i32)| {
            let dx1 = (here.0 - prev.0).signum();
            let dy1 = (here.1 - prev.1).signum();
            let dx2 = (next.0 - here.0).signum();
            let dy2 = (next.1 - here.1).signum();
            if (dx1, dy1) == (dx2, dy2) {
                if dy1 == 0 {
                    '─'
                } else if dx1 == 0 {
                    '│'
                } else if dx1 == dy1 {
                    '╲'
                } else {
                    '╱'
                }
            } else if dy1 == 0 && dx2 == 0 {
                match (dx1, dy2) {
                    (1, 1) => '╮',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╰',
                    _ => '╮',
                }
            } else if dx1 == 0 && dy2 == 0 {
                match (dy1, dx2) {
                    (1, 1) => '╰',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╮',
                    _ => '╰',
                }
            } else if dx1 != dx2 && dy1 != dy2 {
                match (dx1, dy1, dx2, dy2) {
                    (1, 1, 1, -1) | (-1, -1, -1, 1) => '╯',
                    (1, -1, 1, 1) | (-1, 1, -1, -1) => '╮',
                    (1, 1, -1, 1) | (-1, -1, 1, -1) => '╰',
                    (1, -1, -1, -1) | (-1, 1, 1, 1) => '╭',
                    _ => '○',
                }
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else if dx2 == dy2 {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let ch = if dx > (-dy) * 2 {
                '─'
            } else if -dy > dx * 2 {
                '│'
            } else if (x1 - x0).signum() == (y1 - y0).signum() {
                '╲'
            } else {
                '╱'
            };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let draw_micro_shape =
            |grid: &mut Grid, x: i32, y: i32, kind: usize, fg: Color| match kind % 10 {
                0 => {
                    put(grid, x, y, '○', lighten(fg, 10));
                    put(grid, x - 1, y, '╴', darken(fg, 12));
                    put(grid, x + 1, y, '╶', darken(fg, 12));
                }
                1 => {
                    put(grid, x, y, '◇', lighten(fg, 12));
                    put(grid, x, y - 1, '╭', fg);
                    put(grid, x, y + 1, '╯', fg);
                }
                2 => {
                    put(grid, x - 1, y - 1, '╭', fg);
                    put(grid, x + 1, y - 1, '╮', fg);
                    put(grid, x - 1, y + 1, '╰', fg);
                    put(grid, x + 1, y + 1, '╯', fg);
                    put(grid, x, y, '⊙', lighten(fg, 10));
                }
                3 => {
                    put(grid, x, y - 1, '△', lighten(fg, 8));
                    put(grid, x - 1, y, '╰', fg);
                    put(grid, x + 1, y, '╯', fg);
                    put(grid, x, y + 1, '╰', darken(fg, 8));
                }
                4 => {
                    put(grid, x, y, '☉', lighten(fg, 14));
                    put(grid, x - 1, y - 1, '╭', darken(fg, 4));
                    put(grid, x + 1, y - 1, '╮', darken(fg, 4));
                    put(grid, x - 1, y + 1, '╰', darken(fg, 4));
                    put(grid, x + 1, y + 1, '╯', darken(fg, 4));
                }
                5 => {
                    put(grid, x, y, '⌬', lighten(fg, 8));
                    put(grid, x - 1, y, '╭', fg);
                    put(grid, x + 1, y, '╮', fg);
                }
                6 => {
                    put(grid, x, y, '□', lighten(fg, 8));
                    put(grid, x - 1, y, '◜', fg);
                    put(grid, x + 1, y, '◝', fg);
                    put(grid, x, y + 1, '╯', darken(fg, 10));
                }
                7 => {
                    put(grid, x, y, '⊕', lighten(fg, 10));
                    put(grid, x - 1, y, '─', fg);
                    put(grid, x + 1, y, '─', fg);
                    put(grid, x, y - 1, '│', fg);
                    put(grid, x, y + 1, '│', fg);
                }
                8 => {
                    put(grid, x, y, '✦', lighten(fg, 16));
                    put(grid, x - 1, y, '◌', fg);
                    put(grid, x + 1, y, '◌', fg);
                }
                _ => {
                    put(grid, x, y, '∴', lighten(fg, 10));
                    put(grid, x - 1, y - 1, '·', fg);
                    put(grid, x + 1, y + 1, '·', fg);
                    put(grid, x + 1, y - 1, '·', darken(fg, 10));
                }
            };
        let draw_curve = |grid: &mut Grid,
                          p0: (i32, i32),
                          p1: (i32, i32),
                          p2: (i32, i32),
                          fg: Color,
                          accent: Color,
                          shape_offset: usize,
                          ornament_step: usize|
         -> Vec<(i32, i32)> {
            let d0 = ((p1.0 - p0.0).abs() + (p1.1 - p0.1).abs()) as f32;
            let d1 = ((p2.0 - p1.0).abs() + (p2.1 - p1.1).abs()) as f32;
            let samples = ((d0 + d1) * 1.7).clamp(18.0, 240.0) as usize;
            let mut pts = Vec::new();
            for i in 0..=samples {
                let t = i as f32 / samples as f32;
                let u = 1.0 - t;
                let x = (u * u * p0.0 as f32 + 2.0 * u * t * p1.0 as f32 + t * t * p2.0 as f32)
                    .round() as i32;
                let y = (u * u * p0.1 as f32 + 2.0 * u * t * p1.1 as f32 + t * t * p2.1 as f32)
                    .round() as i32;
                if pts.last().copied() != Some((x, y)) {
                    pts.push((x, y));
                }
            }
            if pts.len() < 3 {
                draw_line(grid, p0.0, p0.1, p2.0, p2.1, fg);
                return pts;
            }
            for i in 0..pts.len() {
                let (x, y) = pts[i];
                if i == 0 || i + 1 == pts.len() {
                    if shape_offset % 3 != 1 {
                        put(grid, x, y, '○', darken(accent, 8));
                    }
                } else {
                    let ch = curve_char(pts[i - 1], pts[i], pts[i + 1]);
                    put(grid, x, y, ch, fg);
                    if i % ornament_step == shape_offset % ornament_step {
                        draw_micro_shape(grid, x, y, i + shape_offset, accent);
                    }
                }
            }
            pts
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        accent: Color,
                        gap: usize,
                        ornament_step: usize| {
            let span = (end - start).abs().max(0.04);
            let samples = ((rx + ry) * span * 4.2).max(20.0) as usize;
            let mut pts = Vec::new();
            for i in 0..=samples {
                if gap > 0 && i % gap == gap - 1 {
                    if pts.len() > 2 {
                        for p in 1..pts.len() - 1 {
                            let ch = curve_char(pts[p - 1], pts[p], pts[p + 1]);
                            put(grid, pts[p].0, pts[p].1, ch, fg);
                            if p % (ornament_step * 2) == 0 {
                                draw_micro_shape(grid, pts[p].0, pts[p].1, p, accent);
                            }
                        }
                    }
                    pts.clear();
                    continue;
                }
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if pts.last().copied() != Some(p) {
                    pts.push(p);
                }
            }
            if pts.len() > 2 {
                for p in 1..pts.len() - 1 {
                    let ch = curve_char(pts[p - 1], pts[p], pts[p + 1]);
                    put(grid, pts[p].0, pts[p].1, ch, fg);
                    if p % (ornament_step * 2) == 0 {
                        draw_micro_shape(grid, pts[p].0, pts[p].1, p + 3, accent);
                    }
                }
            }
        };
        let draw_box_frame = |grid: &mut Grid, cx: i32, cy: i32, hw: i32, hh: i32, fg: Color| {
            for x in cx - hw + 1..=cx + hw - 1 {
                put(grid, x, cy - hh, '─', fg);
                put(grid, x, cy + hh, '─', fg);
            }
            for y in cy - hh + 1..=cy + hh - 1 {
                put(grid, cx - hw, y, '│', fg);
                put(grid, cx + hw, y, '│', fg);
            }
            put(grid, cx - hw, cy - hh, '╭', fg);
            put(grid, cx + hw, cy - hh, '╮', fg);
            put(grid, cx - hw, cy + hh, '╰', fg);
            put(grid, cx + hw, cy + hh, '╯', fg);
        };
        let draw_diamond_frame =
            |grid: &mut Grid, cx: i32, cy: i32, rx: i32, ry: i32, fg: Color| {
                let top = (cx, cy - ry);
                let right = (cx + rx, cy);
                let bottom = (cx, cy + ry);
                let left = (cx - rx, cy);
                draw_line(grid, top.0, top.1, right.0, right.1, fg);
                draw_line(grid, right.0, right.1, bottom.0, bottom.1, fg);
                draw_line(grid, bottom.0, bottom.1, left.0, left.1, fg);
                draw_line(grid, left.0, left.1, top.0, top.1, fg);
                put(grid, top.0, top.1, '△', lighten(fg, 8));
                put(grid, right.0, right.1, '◇', lighten(fg, 8));
                put(grid, bottom.0, bottom.1, '▽', lighten(fg, 8));
                put(grid, left.0, left.1, '◇', lighten(fg, 8));
            };
        let draw_ring_ticks = |grid: &mut Grid,
                               cx: i32,
                               cy: i32,
                               rx: f32,
                               ry: f32,
                               count: usize,
                               phase: f32,
                               fg: Color| {
            for i in 0..count {
                let a = phase + i as f32 * std::f32::consts::TAU / count as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if a.sin().abs() > a.cos().abs() {
                    put(grid, p.0 - 1, p.1, '─', fg);
                    put(grid, p.0, p.1, '┼', lighten(fg, 8));
                    put(grid, p.0 + 1, p.1, '─', fg);
                } else {
                    put(grid, p.0, p.1 - 1, '│', fg);
                    put(grid, p.0, p.1, '┼', lighten(fg, 8));
                    put(grid, p.0, p.1 + 1, '│', fg);
                }
            }
        };
        let draw_geo_station = |grid: &mut Grid, x: i32, y: i32, kind: usize, fg: Color| {
            for dy in -1i32..=1 {
                for dx in -3i32..=3 {
                    blank(grid, x + dx, y + dy);
                }
            }
            match kind % 8 {
                0 => {
                    put(grid, x, y - 1, '╱', fg);
                    put(grid, x + 1, y - 1, '╲', fg);
                    put(grid, x - 1, y, '◇', lighten(fg, 16));
                    put(grid, x, y, '◆', lighten(fg, 20));
                    put(grid, x + 1, y, '◇', lighten(fg, 16));
                    put(grid, x, y + 1, '╲', fg);
                    put(grid, x + 1, y + 1, '╱', fg);
                }
                1 => {
                    put(grid, x - 2, y - 1, '╭', fg);
                    put(grid, x - 1, y - 1, '─', fg);
                    put(grid, x, y - 1, '□', lighten(fg, 14));
                    put(grid, x + 1, y - 1, '─', fg);
                    put(grid, x + 2, y - 1, '╮', fg);
                    put(grid, x - 2, y, '│', fg);
                    put(grid, x, y, '⊙', lighten(fg, 20));
                    put(grid, x + 2, y, '│', fg);
                    put(grid, x - 2, y + 1, '╰', fg);
                    put(grid, x - 1, y + 1, '─', fg);
                    put(grid, x, y + 1, '□', lighten(fg, 14));
                    put(grid, x + 1, y + 1, '─', fg);
                    put(grid, x + 2, y + 1, '╯', fg);
                }
                2 => {
                    put(grid, x, y - 1, '△', lighten(fg, 18));
                    put(grid, x - 2, y, '╱', fg);
                    put(grid, x - 1, y, '─', darken(fg, 4));
                    put(grid, x, y, '☉', lighten(fg, 18));
                    put(grid, x + 1, y, '─', darken(fg, 4));
                    put(grid, x + 2, y, '╲', fg);
                    put(grid, x - 1, y + 1, '╰', fg);
                    put(grid, x, y + 1, '─', fg);
                    put(grid, x + 1, y + 1, '╯', fg);
                }
                3 => {
                    put(grid, x, y, '⊕', lighten(fg, 20));
                    put(grid, x - 2, y, '╴', fg);
                    put(grid, x - 1, y, '○', lighten(fg, 12));
                    put(grid, x + 1, y, '○', lighten(fg, 12));
                    put(grid, x + 2, y, '╶', fg);
                    put(grid, x, y - 1, '│', fg);
                    put(grid, x, y + 1, '│', fg);
                }
                4 => {
                    put(grid, x, y, '✦', lighten(fg, 22));
                    put(grid, x - 2, y, '◇', fg);
                    put(grid, x + 2, y, '◇', fg);
                    put(grid, x, y - 1, '△', fg);
                    put(grid, x, y + 1, '▽', fg);
                    put(grid, x - 1, y - 1, '╲', darken(fg, 6));
                    put(grid, x + 1, y - 1, '╱', darken(fg, 6));
                }
                5 => {
                    put(grid, x - 2, y, '╭', fg);
                    put(grid, x - 1, y, '─', fg);
                    put(grid, x, y, '⌬', lighten(fg, 18));
                    put(grid, x + 1, y, '─', fg);
                    put(grid, x + 2, y, '╮', fg);
                    put(grid, x - 1, y + 1, '╰', darken(fg, 8));
                    put(grid, x, y + 1, '─', darken(fg, 8));
                    put(grid, x + 1, y + 1, '╯', darken(fg, 8));
                }
                6 => {
                    put(grid, x - 2, y - 1, '╭', fg);
                    put(grid, x, y - 1, '┬', fg);
                    put(grid, x + 2, y - 1, '╮', fg);
                    put(grid, x - 1, y, '□', lighten(fg, 14));
                    put(grid, x, y, '┼', lighten(fg, 20));
                    put(grid, x + 1, y, '□', lighten(fg, 14));
                    put(grid, x - 2, y + 1, '╰', fg);
                    put(grid, x, y + 1, '┴', fg);
                    put(grid, x + 2, y + 1, '╯', fg);
                }
                _ => {
                    put(grid, x, y + 1, '▽', lighten(fg, 16));
                    put(grid, x - 2, y, '╲', fg);
                    put(grid, x - 1, y, '─', fg);
                    put(grid, x, y, '⊙', lighten(fg, 18));
                    put(grid, x + 1, y, '─', fg);
                    put(grid, x + 2, y, '╱', fg);
                    put(grid, x, y - 1, '╭', darken(fg, 8));
                }
            }
        };
        let draw_ring_stations = |grid: &mut Grid,
                                  cx: i32,
                                  cy: i32,
                                  rx: f32,
                                  ry: f32,
                                  count: usize,
                                  phase: f32,
                                  rung: usize,
                                  fg: Color| {
            let mut count = count.clamp(4, 16);
            if count % 2 == 1 {
                count += 1;
            }
            for i in 0..count {
                let sector = (i * 8 / count) % 8;
                let kind = match (rung + sector) % 8 {
                    0 => 2,
                    1 => 3,
                    2 => 1,
                    3 => 0,
                    4 => 7,
                    5 => 4,
                    6 => 6,
                    _ => 5,
                };
                let a = phase + i as f32 * std::f32::consts::TAU / count as f32;
                let wobble = if (i + rung) % 2 == 0 { 1.006 } else { 0.994 };
                let p = point_on(cx, cy, rx * wobble, ry * wobble, a);
                let color = if sector % 2 == 0 {
                    lighten(fg, 8)
                } else {
                    darken(fg, 4)
                };
                draw_geo_station(grid, p.0, p.1, kind + rung * 3 + i, color);
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 3.0).max(10.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let phase = -std::f32::consts::FRAC_PI_2 + rng.random_range(-0.16..0.16);

        for y in 0..height {
            for x in 0..width {
                let dx = (x as i32 - cx) as f32 / (max_rx * 0.98);
                let dy = (y as i32 - cy) as f32 / (max_ry * 0.98);
                let metric = dx * dx + dy * dy;
                if metric < 0.90 {
                    blank(&mut grid, x as i32, y as i32);
                } else if metric < 1.08 && (x + y + seed as usize) % 4 == 0 {
                    grid[y][x] = Cell::new('·', darken(chalk, 55));
                }
            }
        }

        for band in 0..1 {
            let rx = max_rx - band as f32 * 4.8;
            let ry = max_ry - band as f32 * 1.95;
            let fg = match band {
                0 => ether,
                _ => chalk,
            };
            let segments = (ring_count + band * 2 + 2).max(5);
            let gap = 0.16 + band as f32 * 0.045;
            for seg in 0..segments {
                let a0 = phase + seg as f32 * std::f32::consts::TAU / segments as f32 + gap;
                let a1 = phase + (seg + 1) as f32 * std::f32::consts::TAU / segments as f32 - gap;
                draw_arc(
                    &mut grid,
                    cx,
                    cy,
                    rx,
                    ry,
                    a0,
                    a1,
                    darken(fg, (band * 5) as u8),
                    lighten(fg, 12),
                    if band == 1 { 24 } else { 0 },
                    ornament_step + band * 3,
                );
            }
        }

        let mut ring_specs = Vec::new();
        for r in 0..ring_count {
            let t = (r + 1) as f32 / (ring_count + 1) as f32;
            let scale = 0.24 + t * 0.52 + rng.random_range(-0.012..0.016);
            let nodes = (base_nodes + r + (seed as usize % 2)).clamp(4, 10);
            let ring_phase = phase + r as f32 * 0.29 + rng.random_range(-0.13..0.13);
            let fg = match r % 5 {
                0 => gold,
                1 => ether,
                2 => chalk,
                3 => verdigris,
                _ => rose,
            };
            ring_specs.push((scale, nodes, ring_phase, fg));
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * scale,
                max_ry * scale,
                ring_phase,
                ring_phase + std::f32::consts::TAU,
                darken(fg, 7),
                lighten(fg, 12),
                if r % 2 == 0 { 21 } else { 0 },
                ornament_step + r * 3,
            );

            for n in 0..nodes {
                let a = ring_phase + n as f32 * std::f32::consts::TAU / nodes as f32;
                let p = point_on(cx, cy, max_rx * scale, max_ry * scale, a);
                draw_micro_shape(&mut grid, p.0, p.1, n + r * 5, fg);
                if n % 3 == 0 {
                    let inner = point_on(
                        cx,
                        cy,
                        max_rx * (scale - 0.035).max(0.05),
                        max_ry * (scale - 0.035).max(0.04),
                        a + 0.025,
                    );
                    let outer = point_on(
                        cx,
                        cy,
                        max_rx * (scale + 0.035).min(0.98),
                        max_ry * (scale + 0.035).min(0.98),
                        a - 0.025,
                    );
                    let control = point_on(cx, cy, max_rx * scale, max_ry * scale, a + 0.10);
                    draw_curve(
                        &mut grid,
                        inner,
                        control,
                        outer,
                        darken(fg, 8),
                        lighten(fg, 10),
                        n + r,
                        ornament_step + 8,
                    );
                }
            }
        }

        for path in 0..path_count {
            let base_a = phase
                + path as f32 * std::f32::consts::TAU / path_count as f32
                + rng.random_range(-0.10..0.10);
            let path_color = match path % 5 {
                0 => ether,
                1 => gold,
                2 => chalk,
                3 => verdigris,
                _ => rose,
            };
            let mut prev = point_on(cx, cy, max_rx * 0.42, max_ry * 0.36, base_a);
            for (ri, &(scale, _, ring_phase, ring_color)) in ring_specs.iter().enumerate() {
                let wobble =
                    (path as f32 * 0.73 + ri as f32 * 1.11 + seed as f32 * 0.013).sin() * 0.22;
                let a = base_a + (ring_phase - phase) * 0.22 + wobble;
                let target = point_on(cx, cy, max_rx * scale, max_ry * scale, a);
                let mid_scale = (scale + 0.06).min(0.98);
                let bend = if (path + ri) % 2 == 0 { 0.48 } else { -0.48 };
                let control = point_on(
                    cx,
                    cy,
                    max_rx * mid_scale,
                    max_ry * mid_scale,
                    a + bend + rng.random_range(-0.08..0.08),
                );
                draw_curve(
                    &mut grid,
                    prev,
                    control,
                    target,
                    darken(path_color, (ri * 4) as u8),
                    lighten(ring_color, 10),
                    path + ri * 3,
                    ornament_step + 3 + (path + ri) % 5,
                );
                if (path + ri) % 5 == 0 {
                    let hook = point_on(
                        cx,
                        cy,
                        max_rx * (scale + 0.045).min(0.98),
                        max_ry * (scale + 0.045).min(0.98),
                        a + 0.18,
                    );
                    let hook_control = point_on(cx, cy, max_rx * scale, max_ry * scale, a + 0.34);
                    draw_curve(
                        &mut grid,
                        target,
                        hook_control,
                        hook,
                        darken(ring_color, 10),
                        path_color,
                        path + ri + 5,
                        ornament_step + 10,
                    );
                }
                draw_micro_shape(
                    &mut grid,
                    target.0,
                    target.1,
                    path + ri + seed as usize,
                    shift_hue(lighten(path_color, 5), (ri * 18) as f64),
                );
                prev = target;
            }
            let outer_a = base_a + rng.random_range(-0.08..0.08);
            let outer = point_on(cx, cy, max_rx * 0.98, max_ry * 0.98, outer_a);
            let control = point_on(
                cx,
                cy,
                max_rx * 0.86,
                max_ry * 0.86,
                outer_a + rng.random_range(-0.55..0.55),
            );
            draw_curve(
                &mut grid,
                prev,
                control,
                outer,
                darken(path_color, 6),
                lighten(path_color, 12),
                path + 11,
                ornament_step,
            );
            draw_micro_shape(
                &mut grid,
                outer.0,
                outer.1,
                path + 7,
                lighten(path_color, 12),
            );
        }

        let bridge_count = (path_count / 2).min(2);
        for bridge in 0..bridge_count {
            let scale = rng.random_range(0.52..0.86);
            let a0 = phase
                + bridge as f32 * std::f32::consts::TAU / bridge_count as f32
                + rng.random_range(-0.10..0.10);
            let a2 = a0 + rng.random_range(0.30..0.92);
            let p0 = point_on(cx, cy, max_rx * scale, max_ry * scale, a0);
            let p2 = point_on(cx, cy, max_rx * scale, max_ry * scale, a2);
            let ctrl_scale = (scale + rng.random_range(-0.10..0.13)).clamp(0.16, 0.98);
            let p1 = point_on(
                cx,
                cy,
                max_rx * ctrl_scale,
                max_ry * ctrl_scale,
                (a0 + a2) * 0.5 + rng.random_range(-0.45..0.45),
            );
            let color = match bridge % 5 {
                0 => darken(ether, 12),
                1 => darken(gold, 10),
                2 => darken(rose, 8),
                3 => darken(verdigris, 8),
                _ => darken(chalk, 18),
            };
            draw_curve(
                &mut grid,
                p0,
                p1,
                p2,
                color,
                lighten(color, 16),
                bridge,
                ornament_step + 8 + bridge % 7,
            );
        }

        let outer_belts = [
            (0.935_f32, 0.925_f32, darken(ether, 14)),
            (0.960_f32, 0.955_f32, lighten(ether, 5)),
            (0.985_f32, 0.982_f32, darken(chalk, 2)),
        ];
        for (i, &(sx, sy, color)) in outer_belts.iter().enumerate() {
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * sx,
                max_ry * sy,
                phase + i as f32 * 0.035,
                phase + i as f32 * 0.035 + std::f32::consts::TAU,
                color,
                lighten(color, 10),
                0,
                96,
            );
            draw_ring_stations(
                &mut grid,
                cx,
                cy,
                max_rx * sx,
                max_ry * sy,
                station_base + i * 2,
                phase + i as f32 * 0.29,
                i,
                color,
            );
        }
        draw_ring_ticks(
            &mut grid,
            cx,
            cy,
            max_rx * 0.960,
            max_ry * 0.955,
            10,
            phase + std::f32::consts::PI / 10.0,
            darken(gold, 4),
        );

        let core_rx = (max_rx * 0.43).round() as i32;
        let core_ry = (max_ry * 0.37).round() as i32;
        for dy in -core_ry..=core_ry {
            for dx in -core_rx..=core_rx {
                let metric = (dx as f32 / core_rx.max(1) as f32).powi(2)
                    + (dy as f32 / core_ry.max(1) as f32).powi(2);
                if metric <= 1.0 {
                    blank(&mut grid, cx + dx, cy + dy);
                }
            }
        }
        let thick_rings = [(0.34, 0.29, ether)];
        for (i, &(rxs, rys, color)) in thick_rings.iter().enumerate() {
            for belt in [0.0_f32] {
                draw_arc(
                    &mut grid,
                    cx,
                    cy,
                    max_rx * (rxs + belt),
                    max_ry * (rys + belt * 0.8),
                    phase + i as f32 * 0.08,
                    phase + i as f32 * 0.08 + std::f32::consts::TAU,
                    darken(color, 10),
                    lighten(color, 8),
                    0,
                    64,
                );
            }
            draw_ring_stations(
                &mut grid,
                cx,
                cy,
                max_rx * rxs,
                max_ry * rys,
                station_base,
                phase + i as f32 * 0.41 + 0.17,
                i + 10,
                color,
            );
        }
        let seal_nodes = 6;
        let mut outer_nodes = Vec::new();
        let mut inner_nodes = Vec::new();
        for n in 0..seal_nodes {
            let a = phase + n as f32 * std::f32::consts::TAU / seal_nodes as f32;
            let outer = point_on(cx, cy, max_rx * 0.36, max_ry * 0.31, a);
            let inner = point_on(
                cx,
                cy,
                max_rx * 0.18,
                max_ry * 0.16,
                a + std::f32::consts::PI / 6.0,
            );
            outer_nodes.push(outer);
            inner_nodes.push(inner);
        }
        for n in 0..seal_nodes {
            draw_micro_shape(
                &mut grid,
                outer_nodes[n].0,
                outer_nodes[n].1,
                n + 2,
                if n % 2 == 0 { gold } else { ether },
            );
            put(
                &mut grid,
                inner_nodes[n].0,
                inner_nodes[n].1,
                ['△', '□', '◇', '○', '▽', '⊕'][n],
                lighten(chalk, 8),
            );
        }

        for dy in -3i32..=3i32 {
            for dx in -9i32..=9i32 {
                let metric = (dx as f32 / 9.0).powi(2) + (dy as f32 / 3.0).powi(2);
                if metric <= 1.0 {
                    blank(&mut grid, cx + dx, cy + dy);
                }
            }
        }
        draw_diamond_frame(&mut grid, cx, cy, 8, 3, darken(ether, 6));
        draw_box_frame(&mut grid, cx, cy, 5, 2, lighten(gold, 8));
        draw_line(&mut grid, cx - 7, cy, cx + 7, cy, lighten(chalk, 10));
        draw_line(&mut grid, cx, cy - 4, cx, cy + 4, lighten(chalk, 10));
        put(&mut grid, cx - 2, cy - 1, '╭', lighten(gold, 10));
        put(&mut grid, cx + 2, cy - 1, '╮', lighten(gold, 10));
        put(&mut grid, cx - 2, cy + 1, '╰', lighten(gold, 10));
        put(&mut grid, cx + 2, cy + 1, '╯', lighten(gold, 10));
        put(&mut grid, cx - 1, cy - 1, '─', lighten(gold, 10));
        put(&mut grid, cx, cy - 1, '⊛', lighten(rose, 18));
        put(&mut grid, cx + 1, cy - 1, '─', lighten(gold, 10));
        put(&mut grid, cx - 1, cy + 1, '─', lighten(gold, 10));
        put(&mut grid, cx, cy + 1, '☉', lighten(ether, 16));
        put(&mut grid, cx + 1, cy + 1, '─', lighten(gold, 10));
    } else if mode == "fa5" || mode == "fullmetal-alchemist5" {
        // fa5 [polys=0] [skew=0] [chords=0] [stations=0] -- inscribed polygon star array
        let poly_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let poly_count = if poly_arg == 0 {
            3 + ((seed as usize * 23 + 5) % 4)
        } else {
            poly_arg.clamp(2, 7)
        };
        let skew_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let skew_override = if skew_arg == 0 {
            None
        } else {
            Some(skew_arg.clamp(1, 4))
        };
        let chord_arg: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let chord_pairs = if chord_arg == 0 {
            2 + (seed as usize % 4)
        } else {
            chord_arg.clamp(0, 8)
        };
        let station_arg: usize = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);
        let station_base = if station_arg == 0 {
            6 + (seed as usize % 5)
        } else {
            station_arg.clamp(4, 16)
        };

        let bg = darken(palette[0], 12);
        let chalk = lighten(palette[4], 14);
        let gold = lighten(palette[1], 32);
        let ether = shift_hue(lighten(palette[3], 36), 35.0);
        let rose = shift_hue(lighten(palette[2], 42), -38.0);
        let verdigris = shift_hue(lighten(palette[1], 26), 92.0);
        let hush = darken(palette[2], 66);
        let ring_colors = [chalk, gold, ether, rose, verdigris, chalk, gold];

        for y in 0..height {
            for x in 0..width {
                let n = (x * 31 + y * 43 + seed as usize * 13) % 353;
                let ch = match n {
                    0 => '·',
                    1 if (x + y + seed as usize) % 9 == 0 => '°',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, hush)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let blank = |grid: &mut Grid, x: i32, y: i32| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::blank();
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let curve_char = |prev: (i32, i32), here: (i32, i32), next: (i32, i32)| {
            let dx1 = (here.0 - prev.0).signum();
            let dy1 = (here.1 - prev.1).signum();
            let dx2 = (next.0 - here.0).signum();
            let dy2 = (next.1 - here.1).signum();
            if (dx1, dy1) == (dx2, dy2) {
                if dy1 == 0 {
                    '─'
                } else if dx1 == 0 {
                    '│'
                } else if dx1 == dy1 {
                    '╲'
                } else {
                    '╱'
                }
            } else if dy1 == 0 && dx2 == 0 {
                match (dx1, dy2) {
                    (1, 1) => '╮',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╰',
                    _ => '╮',
                }
            } else if dx1 == 0 && dy2 == 0 {
                match (dy1, dx2) {
                    (1, 1) => '╰',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╮',
                    _ => '╰',
                }
            } else if dx1 != dx2 && dy1 != dy2 {
                match (dx1, dy1, dx2, dy2) {
                    (1, 1, 1, -1) | (-1, -1, -1, 1) => '╯',
                    (1, -1, 1, 1) | (-1, 1, -1, -1) => '╮',
                    (1, 1, -1, 1) | (-1, -1, 1, -1) => '╰',
                    (1, -1, -1, -1) | (-1, 1, 1, 1) => '╭',
                    _ => '○',
                }
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else if dx2 == dy2 {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        gap: usize| {
            let span = (end - start).abs().max(0.05);
            let samples = ((rx + ry) * span * 3.8).max(20.0) as usize;
            let mut pts: Vec<(i32, i32)> = Vec::new();
            let mut flush = |pts: &mut Vec<(i32, i32)>| {
                if pts.len() > 2 {
                    for p in 1..pts.len() - 1 {
                        let ch = curve_char(pts[p - 1], pts[p], pts[p + 1]);
                        put(grid, pts[p].0, pts[p].1, ch, fg);
                    }
                }
                pts.clear();
            };
            for i in 0..=samples {
                if gap > 0 && i % gap == gap - 1 {
                    flush(&mut pts);
                    continue;
                }
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if pts.last().copied() != Some(p) {
                    pts.push(p);
                }
            }
            flush(&mut pts);
        };
        let draw_micro_shape = |grid: &mut Grid, x: i32, y: i32, kind: usize, fg: Color| match kind % 10 {
            0 => {
                put(grid, x, y, '○', lighten(fg, 10));
                put(grid, x - 1, y, '╴', darken(fg, 12));
                put(grid, x + 1, y, '╶', darken(fg, 12));
            }
            1 => {
                put(grid, x, y, '◇', lighten(fg, 12));
                put(grid, x, y - 1, '╭', fg);
                put(grid, x, y + 1, '╯', fg);
            }
            2 => {
                put(grid, x - 1, y - 1, '╭', fg);
                put(grid, x + 1, y - 1, '╮', fg);
                put(grid, x - 1, y + 1, '╰', fg);
                put(grid, x + 1, y + 1, '╯', fg);
                put(grid, x, y, '⊙', lighten(fg, 10));
            }
            3 => {
                put(grid, x, y - 1, '△', lighten(fg, 8));
                put(grid, x - 1, y, '╰', fg);
                put(grid, x + 1, y, '╯', fg);
                put(grid, x, y + 1, '╰', darken(fg, 8));
            }
            4 => {
                put(grid, x, y, '☉', lighten(fg, 14));
                put(grid, x - 1, y - 1, '╭', darken(fg, 4));
                put(grid, x + 1, y - 1, '╮', darken(fg, 4));
                put(grid, x - 1, y + 1, '╰', darken(fg, 4));
                put(grid, x + 1, y + 1, '╯', darken(fg, 4));
            }
            5 => {
                put(grid, x, y, '⌬', lighten(fg, 8));
                put(grid, x - 1, y, '╭', fg);
                put(grid, x + 1, y, '╮', fg);
            }
            6 => {
                put(grid, x, y, '□', lighten(fg, 8));
                put(grid, x - 1, y, '◜', fg);
                put(grid, x + 1, y, '◝', fg);
                put(grid, x, y + 1, '╯', darken(fg, 10));
            }
            7 => {
                put(grid, x, y, '⊕', lighten(fg, 10));
                put(grid, x - 1, y, '─', fg);
                put(grid, x + 1, y, '─', fg);
                put(grid, x, y - 1, '│', fg);
                put(grid, x, y + 1, '│', fg);
            }
            8 => {
                put(grid, x, y, '✦', lighten(fg, 16));
                put(grid, x - 1, y, '◌', fg);
                put(grid, x + 1, y, '◌', fg);
            }
            _ => {
                put(grid, x, y, '∴', lighten(fg, 10));
                put(grid, x - 1, y - 1, '·', fg);
                put(grid, x + 1, y + 1, '·', fg);
                put(grid, x + 1, y - 1, '·', darken(fg, 10));
            }
        };
        let draw_ring_ticks = |grid: &mut Grid,
                               cx: i32,
                               cy: i32,
                               rx: f32,
                               ry: f32,
                               count: usize,
                               phase: f32,
                               fg: Color| {
            for i in 0..count {
                let a = phase + i as f32 * std::f32::consts::TAU / count as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if a.sin().abs() > a.cos().abs() {
                    put(grid, p.0 - 1, p.1, '─', fg);
                    put(grid, p.0, p.1, '┼', lighten(fg, 8));
                    put(grid, p.0 + 1, p.1, '─', fg);
                } else {
                    put(grid, p.0, p.1 - 1, '│', fg);
                    put(grid, p.0, p.1, '┼', lighten(fg, 8));
                    put(grid, p.0, p.1 + 1, '│', fg);
                }
            }
        };
        let draw_polygon = |grid: &mut Grid,
                            cx: i32,
                            cy: i32,
                            rx: f32,
                            ry: f32,
                            n: usize,
                            phase: f32,
                            fg: Color|
         -> Vec<(i32, i32)> {
            let n = n.max(3);
            let mut verts = Vec::with_capacity(n);
            for i in 0..n {
                let a = phase + i as f32 * std::f32::consts::TAU / n as f32;
                verts.push(point_on(cx, cy, rx, ry, a));
            }
            for i in 0..n {
                let (ax, ay) = verts[i];
                let (bx, by) = verts[(i + 1) % n];
                draw_line(grid, ax, ay, bx, by, fg);
            }
            verts
        };
        let draw_star = |grid: &mut Grid, verts: &[(i32, i32)], k: usize, fg: Color| {
            let n = verts.len();
            if k == 0 || n < 3 {
                return;
            }
            for i in 0..n {
                let j = (i + k) % n;
                if j != i {
                    let (ax, ay) = verts[i];
                    let (bx, by) = verts[j];
                    draw_line(grid, ax, ay, bx, by, fg);
                }
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 3.0).max(10.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let base_phase = -std::f32::consts::FRAC_PI_2 + rng.random_range(-0.12..0.12);

        for y in 0..height {
            for x in 0..width {
                let dx = (x as i32 - cx) as f32 / (max_rx * 0.96);
                let dy = (y as i32 - cy) as f32 / (max_ry * 0.96);
                let metric = dx * dx + dy * dy;
                if metric < 0.92 {
                    blank(&mut grid, x as i32, y as i32);
                } else if metric < 1.08 && (x + y + seed as usize) % 4 == 0 {
                    grid[y][x] = Cell::new('·', darken(chalk, 55));
                }
            }
        }

        for &(sx, sy, color) in [
            (0.965_f32, 0.955_f32, darken(ether, 12)),
            (0.985_f32, 0.978_f32, lighten(chalk, 4)),
        ]
        .iter()
        {
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * sx,
                max_ry * sy,
                base_phase,
                base_phase + std::f32::consts::TAU,
                color,
                0,
            );
        }
        draw_ring_ticks(
            &mut grid,
            cx,
            cy,
            max_rx * 0.975,
            max_ry * 0.965,
            station_base,
            base_phase,
            darken(gold, 6),
        );

        let scale_lo = 0.30_f32;
        let scale_hi = 0.86_f32;
        let mut ring_specs: Vec<(usize, f32, f32, Color)> = Vec::new();
        for r in 0..poly_count {
            let t = if poly_count > 1 {
                r as f32 / (poly_count - 1) as f32
            } else {
                0.5
            };
            let scale = scale_lo + t * (scale_hi - scale_lo) + rng.random_range(-0.014..0.014);
            let n = 3 + ((seed as usize * (r * 7 + 3) + 11) % 6);
            let phase = base_phase
                + r as f32 * 0.41
                + rng.random_range(-0.20..0.20);
            let color = ring_colors[r % ring_colors.len()];
            ring_specs.push((n, scale, phase, color));
        }

        let mut all_verts: Vec<Vec<(i32, i32)>> = Vec::new();
        for &(n, scale, phase, color) in ring_specs.iter() {
            let rx = max_rx * scale;
            let ry = max_ry * scale;
            let verts = draw_polygon(&mut grid, cx, cy, rx, ry, n, phase, darken(color, 6));
            if n >= 5 {
                let max_k = (n - 1) / 2;
                let k = skew_override
                    .unwrap_or_else(|| 2 + (seed as usize + n) % (max_k - 1).max(1))
                    .min(max_k)
                    .max(2);
                draw_star(&mut grid, &verts, k, lighten(color, 10));
            } else {
                let twin_phase = phase + std::f32::consts::TAU / (2 * n) as f32;
                draw_polygon(&mut grid, cx, cy, rx, ry, n, twin_phase, darken(color, 14));
            }
            for (i, &(vx, vy)) in verts.iter().enumerate() {
                draw_micro_shape(&mut grid, vx, vy, i + n + (seed as usize % 5), color);
            }
            all_verts.push(verts);
        }

        for r in 1..all_verts.len() {
            let outer = &all_verts[r];
            let inner = &all_verts[r - 1];
            if outer.is_empty() || inner.is_empty() {
                continue;
            }
            let chord_color = match r % 5 {
                0 => darken(verdigris, 8),
                1 => darken(rose, 6),
                2 => darken(ether, 10),
                3 => darken(gold, 6),
                _ => darken(chalk, 14),
            };
            let offset = (seed as usize + r * 3) % inner.len();
            let steps = chord_pairs.min(outer.len());
            for c in 0..steps {
                let oi = if steps > 0 {
                    (c * outer.len()) / steps
                } else {
                    c
                };
                let ii = (oi + offset) % inner.len();
                let (ox, oy) = outer[oi];
                let (ix, iy) = inner[ii];
                draw_line(&mut grid, ox, oy, ix, iy, chord_color);
            }
        }

        let core_rx = (max_rx * 0.22).round() as i32;
        let core_ry = (max_ry * 0.22).round() as i32;
        for dy in -core_ry..=core_ry {
            for dx in -core_rx..=core_rx {
                let metric = (dx as f32 / core_rx.max(1) as f32).powi(2)
                    + (dy as f32 / core_ry.max(1) as f32).powi(2);
                if metric <= 1.0 {
                    blank(&mut grid, cx + dx, cy + dy);
                }
            }
        }
        draw_arc(
            &mut grid,
            cx,
            cy,
            max_rx * 0.205,
            max_ry * 0.205,
            base_phase,
            base_phase + std::f32::consts::TAU,
            darken(ether, 8),
            0,
        );
        let core_n = 3 + (seed as usize % 4);
        let core_verts = draw_polygon(
            &mut grid,
            cx,
            cy,
            max_rx * 0.16,
            max_ry * 0.16,
            core_n,
            base_phase,
            lighten(gold, 8),
        );
        if core_n >= 5 {
            draw_star(&mut grid, &core_verts, 2, lighten(ether, 12));
        } else {
            let twin_phase = base_phase + std::f32::consts::TAU / (2 * core_n) as f32;
            draw_polygon(
                &mut grid,
                cx,
                cy,
                max_rx * 0.16,
                max_ry * 0.16,
                core_n,
                twin_phase,
                darken(ether, 10),
            );
        }
        let core_ring_n = 6;
        for i in 0..core_ring_n {
            let a = base_phase + i as f32 * std::f32::consts::TAU / core_ring_n as f32;
            let p = point_on(cx, cy, max_rx * 0.10, max_ry * 0.10, a);
            put(
                &mut grid,
                p.0,
                p.1,
                ['△', '◇', '○', '□', '▽', '⊕'][i],
                lighten(chalk, 8),
            );
        }
        put(&mut grid, cx, cy, '⊙', lighten(rose, 18));
    } else if mode == "spiro" {
        grid = draw_spiro(grid, width, height, seed, palette, rng, t_anim, &args);
    } else if mode == "spiro-tile" {
        grid = draw_spiro_tile(grid, width, height, seed, palette, rng, t_anim, &args);
    } else if mode == "weave" {
        grid = draw_weave(grid, width, height, seed, palette, rng, t_anim, &args);
    } else if mode == "gears" {
        grid = draw_gears(grid, width, height, seed, palette, rng, t_anim, &args);
    } else if mode == "kaleido" {
        // kaleido [folds=0] [strokes=0] [mirror=0] -- N-fold symmetric mandala
        let fold_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let folds = if fold_arg == 0 {
            [6, 8, 12, 5, 7][(seed as usize) % 5]
        } else {
            fold_arg.clamp(3, 16)
        };
        let stroke_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let stroke_count = if stroke_arg == 0 {
            8 + (seed as usize % 7)
        } else {
            stroke_arg.clamp(3, 24)
        };
        let mirror_arg: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let mirror = mirror_arg != 0 || (seed as usize % 3 == 0);

        let bg = darken(palette[0], 14);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 30);
        let cyan = shift_hue(lighten(palette[3], 34), 35.0);
        let magenta = shift_hue(lighten(palette[2], 40), -42.0);
        let lime = shift_hue(lighten(palette[1], 28), 90.0);
        let violet = shift_hue(lighten(palette[3], 30), 150.0);
        let stroke_colors = [chalk, gold, cyan, magenta, lime, violet, chalk, gold];

        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(' ', bg);
            }
        }
        for _ in 0..(width * height / 120) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);
            grid[y][x] = Cell::new('·', darken(chalk, 62));
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid,
                         mut x0: i32,
                         mut y0: i32,
                         x1: i32,
                         y1: i32,
                         fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let curve_char = |prev: (i32, i32), here: (i32, i32), next: (i32, i32)| {
            let dx1 = (here.0 - prev.0).signum();
            let dy1 = (here.1 - prev.1).signum();
            let dx2 = (next.0 - here.0).signum();
            let dy2 = (next.1 - here.1).signum();
            if (dx1, dy1) == (dx2, dy2) {
                if dy1 == 0 {
                    '─'
                } else if dx1 == 0 {
                    '│'
                } else if dx1 == dy1 {
                    '╲'
                } else {
                    '╱'
                }
            } else if dy1 == 0 && dx2 == 0 {
                match (dx1, dy2) {
                    (1, 1) => '╮',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╰',
                    _ => '╮',
                }
            } else if dx1 == 0 && dy2 == 0 {
                match (dy1, dx2) {
                    (1, 1) => '╰',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╮',
                    _ => '╰',
                }
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else if dx2 == dy2 {
                '╲'
            } else {
                '╱'
            }
        };

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let max_r = (width.min(height * 2) as f32 / 2.0 - 2.0).max(8.0);
        let wedge = std::f32::consts::TAU / folds as f32;

        // generate strokes within wedge [0, wedge] as (kind, p1, p2, r, a0, a1, color_idx, ch)
        enum SK {
            Seg((f32, f32), (f32, f32)),
            Arc((f32, f32), f32, f32, f32),
            Dot((f32, f32)),
        }
        let polar = |rad: f32, ang: f32| -> (f32, f32) {
            (rad * ang.cos(), rad * ang.sin() * 0.5)
        };
        let mut strokes: Vec<(SK, usize, char)> = Vec::new();
        for s in 0..stroke_count {
            let color_idx = s % stroke_colors.len();
            let kind = seed as usize + s;
            match kind % 4 {
                0 => {
                    let r1 = rng.random_range(0.15..0.95) * max_r;
                    let r2 = rng.random_range(0.15..0.95) * max_r;
                    let a1 = rng.random_range(0.0..wedge);
                    let a2 = rng.random_range(0.0..wedge);
                    strokes.push((SK::Seg(polar(r1, a1), polar(r2, a2)), color_idx, '─'));
                }
                1 => {
                    let rc = rng.random_range(0.2..0.85) * max_r;
                    let ac = rng.random_range(0.05..wedge - 0.05);
                    let ar = rng.random_range(0.06..0.22) * max_r;
                    let a0 = rng.random_range(0.0..std::f32::consts::TAU);
                    let a1 = a0 + rng.random_range(0.6..2.4);
                    let center = polar(rc, ac);
                    strokes.push((SK::Arc(center, ar, a0, a1), color_idx, '○'));
                }
                2 => {
                    let r1 = rng.random_range(0.2..0.9) * max_r;
                    let a1 = rng.random_range(0.0..wedge);
                    let glyphs = ['◇', '△', '▽', '○', '✦', '⊕', '⊙', '⌬', '□'];
                    strokes.push((
                        SK::Dot(polar(r1, a1)),
                        color_idx,
                        glyphs[(s + seed as usize) % glyphs.len()],
                    ));
                }
                _ => {
                    // short chord cluster: two segments sharing an endpoint
                    let r0 = rng.random_range(0.3..0.9) * max_r;
                    let a0 = rng.random_range(0.0..wedge);
                    let pivot = polar(r0, a0);
                    let r1 = rng.random_range(0.15..0.95) * max_r;
                    let a1 = rng.random_range(0.0..wedge);
                    let r2 = rng.random_range(0.15..0.95) * max_r;
                    let a2 = rng.random_range(0.0..wedge);
                    strokes.push((SK::Seg(pivot, polar(r1, a1)), color_idx, '─'));
                    strokes.push((SK::Seg(pivot, polar(r2, a2)), color_idx, '─'));
                }
            }
        }

        let rotate = |p: (f32, f32), ang: f32| -> (f32, f32) {
            (p.0 * ang.cos() - p.1 * ang.sin(), p.0 * ang.sin() + p.1 * ang.cos())
        };
        let _ = rotate;

        let to_screen = |lp: (f32, f32), ox: f32, oy: f32| -> (i32, i32) {
            ((ox + lp.0).round() as i32, (oy + lp.1).round() as i32)
        };

        let mut pass = |sign: f32| {
            for k in 0..folds {
                let ang = k as f32 * wedge;
                let cosr = ang.cos();
                let sinr = ang.sin();
                let rot = |p: (f32, f32)| -> (f32, f32) {
                    // p.y already aspect-compressed (×0.5); rotate in that space
                    (p.0 * cosr - p.1 * sinr, p.0 * sinr + p.1 * cosr)
                };
                let rot_m = |p: (f32, f32)| -> (f32, f32) {
                    let q = (p.0, sign * p.1);
                    (q.0 * cosr - q.1 * sinr, q.0 * sinr + q.1 * cosr)
                };
                for (sk, cidx, glyph) in &strokes {
                    let color = stroke_colors[*cidx % stroke_colors.len()];
                    match sk {
                        SK::Seg(a, b) => {
                            let (a2, b2) = if sign != 0.0 {
                                (rot_m(*a), rot_m(*b))
                            } else {
                                (rot(*a), rot(*b))
                            };
                            let pa = to_screen(a2, cx, cy);
                            let pb = to_screen(b2, cx, cy);
                            draw_line(&mut grid, pa.0, pa.1, pb.0, pb.1, color);
                        }
                        SK::Arc(center, r, a0, a1) => {
                            let c2 = if sign != 0.0 { rot_m(*center) } else { rot(*center) };
                            let cs = to_screen(c2, cx, cy);
                            let samples = ((*r + *r) * (*a1 - *a0).abs() * 3.8).max(12.0) as usize;
                            let mut pts: Vec<(i32, i32)> = Vec::new();
                            for i in 0..=samples {
                                let a = *a0 + (*a1 - *a0) * i as f32 / samples as f32;
                                let lp = (*r * a.cos(), *r * a.sin() * 0.5);
                                let p = to_screen(lp, cs.0 as f32, cs.1 as f32);
                                if pts.last().copied() != Some(p) {
                                    pts.push(p);
                                }
                            }
                            for i in 1..pts.len().saturating_sub(1) {
                                let ch = curve_char(pts[i - 1], pts[i], pts[i + 1]);
                                put(&mut grid, pts[i].0, pts[i].1, ch, color);
                            }
                        }
                        SK::Dot(p) => {
                            let p2 = if sign != 0.0 { rot_m(*p) } else { rot(*p) };
                            let ps = to_screen(p2, cx, cy);
                            put(&mut grid, ps.0, ps.1, *glyph, lighten(color, 12));
                        }
                    }
                }
            }
        };
        pass(0.0);
        if mirror {
            pass(1.0);
        }
        put(&mut grid, cx.round() as i32, cy.round() as i32, '⊙', lighten(chalk, 12));
    } else if mode == "contour" {
        // contour [levels=0] [scale=0] -- topographic iso-lines over procedural heightmap
        let level_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let level_count = if level_arg == 0 {
            6 + (seed as usize % 5)
        } else {
            level_arg.clamp(3, 14)
        };
        let scale_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let hscale = if scale_arg == 0 {
            0.16 + (seed as f32 * 0.013).fract() * 0.10
        } else {
            (scale_arg as f32 / 100.0).clamp(0.05, 0.5)
        };

        let bg = darken(palette[0], 8);
        let deep_c = darken(palette[1], 30);
        let mid_c = lighten(palette[3], 10);
        let high_c = lighten(palette[4], 16);
        let snow = lighten(palette[4], 30);
        let hush = darken(palette[2], 60);

        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(' ', bg);
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid,
                         mut x0: i32,
                         mut y0: i32,
                         x1: i32,
                         y1: i32,
                         fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };

        // heightmap with sines + random gaussian bumps
        let bump_n = 1 + (seed as usize % 3);
        let mut bumps: Vec<(f32, f32, f32, f32)> = Vec::new();
        for _ in 0..bump_n {
            let bx = rng.random_range(0.1..0.9) * width as f32;
            let by = rng.random_range(0.1..0.9) * height as f32;
            let br = rng.random_range(3.0..9.0);
            let ba = rng.random_range(0.5..1.4) * if rng.random_range(0..2) == 0 { -1.0 } else { 1.0 };
            bumps.push((bx, by, br, ba));
        }
        let hfield = |xf: f32, yf: f32| -> f32 {
            let mut v = (hscale * xf).sin() * (hscale * 1.1 * yf).cos();
            v += 0.6 * (hscale * 1.8 * xf + 0.4).sin() * (hscale * 1.3 * yf - 0.2).sin();
            v += 0.4 * (hscale * 0.7 * xf - 0.1).cos() * (hscale * 2.1 * yf + 0.5).cos();
            for &(bx, by, br, ba) in &bumps {
                let dx = xf - bx;
                let dy = yf - by;
                v += ba * (-(dx * dx + dy * dy) / (br * br)).exp();
            }
            v
        };

        let cols = width + 1;
        let rows = height + 1;
        let mut h = vec![vec![0.0f32; cols]; rows];
        let mut hmin = f32::INFINITY;
        let mut hmax = f32::NEG_INFINITY;
        for yy in 0..rows {
            for xx in 0..cols {
                let v = hfield(xx as f32, yy as f32);
                h[yy][xx] = v;
                if v < hmin {
                    hmin = v;
                }
                if v > hmax {
                    hmax = v;
                }
            }
        }

        let level_color = |frac: f32| -> Color {
            if frac < 0.33 {
                let t = frac / 0.33;
                lerp_color(deep_c, mid_c, t)
            } else if frac < 0.7 {
                let t = (frac - 0.33) / 0.37;
                lerp_color(mid_c, high_c, t)
            } else {
                let t = (frac - 0.7) / 0.3;
                lerp_color(high_c, snow, t)
            }
        };

        for li in 0..level_count {
            let frac = (li as f32 + 0.5) / level_count as f32;
            let level = hmin + (hmax - hmin) * frac;
            let color = level_color(frac);
            let major = li % 3 == 0;
            let line_color = if major { lighten(color, 8) } else { darken(color, 10) };

            for yy in 0..height {
                for xx in 0..width {
                    let c00 = h[yy][xx];
                    let c10 = h[yy][xx + 1];
                    let c11 = h[yy + 1][xx + 1];
                    let c01 = h[yy + 1][xx];
                    let mut code = 0u8;
                    if c00 > level {
                        code |= 1;
                    }
                    if c10 > level {
                        code |= 2;
                    }
                    if c11 > level {
                        code |= 4;
                    }
                    if c01 > level {
                        code |= 8;
                    }
                    if code == 0 || code == 15 {
                        continue;
                    }
                    let xf = xx as f32;
                    let yf = yy as f32;
                    let edge_pt = |e: u8| -> (f32, f32) {
                        match e {
                            1 => {
                                // bottom edge: (xf,yf)-(xf+1,yf)
                                let t = (level - c00) / (c10 - c00);
                                (xf + t, yf)
                            }
                            2 => {
                                // right edge: (xf+1,yf)-(xf+1,yf+1)
                                let t = (level - c10) / (c11 - c10);
                                (xf + 1.0, yf + t)
                            }
                            4 => {
                                // top edge: (xf,yf+1)-(xf+1,yf+1)
                                let t = (level - c01) / (c11 - c01);
                                (xf + t, yf + 1.0)
                            }
                            8 => {
                                // left edge: (xf,yf)-(xf,yf+1)
                                let t = (level - c00) / (c01 - c00);
                                (xf, yf + t)
                            }
                            _ => (xf, yf),
                        }
                    };
                    let pairs: &[(u8, u8)] = match code {
                        1 | 14 => &[(8, 1)],
                        2 | 13 => &[(1, 2)],
                        3 | 12 => &[(8, 2)],
                        4 | 11 => &[(2, 4)],
                        5 => &[(8, 4), (1, 2)],
                        6 | 9 => &[(1, 4)],
                        7 | 8 => &[(8, 4)],
                        10 => &[(8, 1), (2, 4)],
                        _ => &[],
                    };
                    for &(ea, eb) in pairs {
                        let pa = edge_pt(ea);
                        let pb = edge_pt(eb);
                        let ax = pa.0.round() as i32;
                        let ay = pa.1.round() as i32;
                        let bx = pb.0.round() as i32;
                        let by = pb.1.round() as i32;
                        if major {
                            draw_line(&mut grid, ax, ay, bx, by, line_color);
                        } else {
                            // minor contour: sparse char along the segment midpoint
                            let mx = ((pa.0 + pb.0) * 0.5).round() as i32;
                            let my = ((pa.1 + pb.1) * 0.5).round() as i32;
                            let glyph = if (xx + yy) % 3 == 0 { '·' } else { '∙' };
                            put(&mut grid, mx, my, glyph, line_color);
                        }
                    }
                }
            }
        }
        // sparse summit markers
        let _ = hush;
    } else if mode == "world" {
        render_world(&mut grid, width, height, &palette, &mut rng);
    } else if mode == "noise" {
        let names = ["truchet", "higaki", "higaki-s", "grass", "static", "dot"];
        let cols = NOISE_VARIANT_COUNT;
        let cell_w = width / cols;
        for i in 0..NOISE_VARIANT_COUNT {
            let x0 = i * cell_w;
            let r = Rect {
                x: x0,
                y: 1,
                w: cell_w,
                h: height - 1,
            };
            let variant = noise_variant_from_index(i);
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            fill_noise(&mut grid, &r, variant, c1, c2, &mut rng);
            for (j, ch) in names[i].chars().enumerate() {
                if x0 + j < width {
                    grid[0][x0 + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "eyes++" {
        draw_eyes_pp(&mut grid, width, height, seed, &palette, &mut rng);
    } else if mode == "fullmetal-eyes++" {
        draw_fme_pp(&mut grid, width, height, seed, &palette, &mut rng);
    } else if mode == "trees++" {
        draw_trees_pp(&mut grid, width, height, seed, &palette, &mut rng);
    } else if mode == "forest++" {
        draw_forest_pp(&mut grid, width, height, seed, &palette, &mut rng);
    } else if mode == "phyllotaxis" {
        draw_phyllotaxis(&mut grid, width, height, seed, &palette, &mut rng, t_anim);
    } else if mode == "moire" {
        draw_moire(&mut grid, width, height, seed, &palette, &mut rng, t_anim);
    } else if mode == "nebula" {
        draw_nebula(&mut grid, width, height, seed, &palette, &mut rng, t_anim);
    } else if mode == "delta" {
        draw_delta(&mut grid, width, height, seed, &palette, &mut rng, t_anim);
    } else if mode == "stained" {
        draw_stained(&mut grid, width, height, seed, &palette, &mut rng);
    } else {
        fill_truchet(&mut grid, width, height, darken(palette[1], 80), &mut rng);

        let cx = width / 2;
        let cy = height / 2;
        let content_w = 30;
        let content_h = 10;
        let x0 = cx - content_w / 2;
        let y0 = cy - content_h / 2;

        for y in y0..y0 + content_h {
            for x in x0..x0 + content_w {
                grid[y][x] = Cell::blank();
            }
        }

        let lines = [
            "「 技 」 S K I L L S",
            "",
            "  typespec ···· 12",
            "  ast-grep ···· 5",
            "  tree-sit ···· 3",
            "  alloy    ···· 2",
            "",
            "  ◁━━ 43 LOADED",
        ];

        for (i, line) in lines.iter().enumerate() {
            let y = y0 + 1 + i;
            if y < y0 + content_h {
                for (j, ch) in line.chars().enumerate() {
                    let x = x0 + 1 + j;
                    if x < x0 + content_w {
                        grid[y][x] = Cell::new(ch, palette[4]);
                    }
                }
            }
        }

        for y in 2..18 {
            for x in 2..22 {
                grid[y][x] = Cell::blank();
            }
        }
        grow_tree(&mut grid, 12, 17, 3, 8, palette[1], &mut rng);

        for y in 2..18 {
            for x in 58..78 {
                grid[y][x] = Cell::blank();
            }
        }
        grow_tree(&mut grid, 68, 17, 3, 8, palette[2], &mut rng);

        draw_flower(&mut grid, 30, 8, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 50, 8, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 15, 35, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 65, 35, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 40, 38, rng.random_range(0..5), palette[3]);
    }

    emit_grid(&grid);
}

// ============================================================================
// "++" modes and dealer's-choice modes. Self-contained renderers; each tuned
// to look good with demo defaults (no required args). Shared geometry helpers.
// ============================================================================

fn pp_put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

fn pp_point_on(cx: i32, cy: i32, rx: f32, ry: f32, a: f32) -> (i32, i32) {
    (
        cx + (a.cos() * rx).round() as i32,
        cy + (a.sin() * ry).round() as i32,
    )
}

fn pp_stroke(dx: i32, dy: i32) -> char {
    if dx.abs() > dy.abs() * 2 {
        '─'
    } else if dy.abs() > dx.abs() * 2 {
        '│'
    } else if dx.signum() == dy.signum() {
        '╲'
    } else {
        '╱'
    }
}

fn pp_line(grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color) {
    let ch = pp_stroke(x1 - x0, y1 - y0);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        pp_put(grid, x0, y0, ch, fg);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn pp_arc(grid: &mut Grid, cx: i32, cy: i32, rx: f32, ry: f32, start: f32, end: f32, fg: Color, gap: usize) {
    let samples = ((rx + ry) * 16.0).max(90.0) as usize;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=samples {
        if gap > 0 && i % gap == gap - 1 {
            prev = None;
            continue;
        }
        let a = start + (end - start) * i as f32 / samples as f32;
        let p = pp_point_on(cx, cy, rx, ry, a);
        if let Some(q) = prev {
            pp_line(grid, q.0, q.1, p.0, p.1, fg);
        } else {
            pp_put(grid, p.0, p.1, '·', fg);
        }
        prev = Some(p);
    }
}

// --- eyes++ : an argus field. Hero eye + two orbital rings of gazing eyes,
//     dense rays, halo arcs, every eye tracking a seeded lure. ---
fn draw_eyes_pp(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng) {
    use std::f32::consts::TAU;
    let bg = darken(palette[0], 14);
    let chalk = lighten(palette[4], 14);
    let gold = lighten(palette[1], 30);
    let iris_outer = shift_hue(lighten(palette[3], 26), 18.0);
    let iris_inner = lighten(palette[2], 20);
    let lid_color = lighten(palette[1], 18);
    let pupil = darken(palette[0], 2);
    let sclera = lighten(palette[4], 6);
    let ray_color = lighten(palette[4], 18);
    let hush = darken(palette[2], 60);

    for y in 0..height {
        for x in 0..width {
            let n = (x * 17 + y * 29 + seed as usize * 3) % 97;
            let (ch, col) = match n {
                0 => ('·', hush),
                1 if (x + y) % 3 == 0 => ('∙', hush),
                _ => (' ', bg),
            };
            grid[y][x] = Cell::new(ch, col);
        }
    }

    let cx = width as i32 / 2;
    let cy = height as i32 / 2;
    let max_rx = (width as f32 / 2.0 - 1.0).max(10.0);
    let max_ry = (height as f32 / 2.0 - 1.0).max(5.0);
    let phase = rng.random_range(0.0f32..TAU);

    let lure_x = (cx + rng.random_range(-(width as i32 / 6)..=(width as i32 / 6))).clamp(8, width as i32 - 8);
    let lure_y = (cy + rng.random_range(-(height as i32 / 5)..=(height as i32 / 5))).clamp(6, height as i32 - 4);
    let lure_col = shift_hue(lighten(iris_inner, 26), rng.random_range(-40..=40) as f64);

    let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| -> (i32, i32) {
        let dx = (lure_x - ex) as f32;
        let dy = (lure_y - ey) as f32;
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        let gx = ((dx / d) * (rx as f32 * 0.30)).round() as i32;
        let gy = ((dy / d) * (ry as f32 * 0.55)).round() as i32;
        (
            gx.clamp(-(rx / 3).max(1), (rx / 3).max(1)),
            gy.clamp(-(ry / 2).max(1), (ry / 2).max(1)),
        )
    };

    let ray_count = 28 + (seed as usize % 16);
    for i in 0..ray_count {
        let a = phase + i as f32 * TAU / ray_count as f32;
        let long = i % 2 == 0;
        let r0 = if long { 7.0 } else { 5.0 };
        let r1 = if long { 0.99 } else { 0.62 };
        let p0 = pp_point_on(cx, cy, r0, r0 * 0.5, a);
        let p1 = pp_point_on(cx, cy, max_rx * r1, max_ry * r1, a);
        pp_line(grid, p0.0, p0.1, p1.0, p1.1, darken(ray_color, if long { 8 } else { 30 }));
        if long {
            pp_put(grid, p1.0, p1.1, '◇', darken(ray_color, 18));
        }
    }

    for r in [0.42f32, 0.55, 0.70, 0.86] {
        pp_arc(grid, cx, cy, max_rx * r, max_ry * r, 0.0, TAU, darken(gold, 24 + (r * 20.0) as u8), 0);
    }

    let draw_eye = |grid: &mut Grid, ex: i32, ey: i32, erx: i32, ery: i32, gx: i32, gy: i32, io: Color, ii: Color, lid: Color, fibers: usize| {
        let iris_rx = (erx as f32 * 0.44).round().max(1.0) as i32;
        let iris_ry = (ery as f32 * 0.92).round().max(1.0) as i32;
        let pupil_rx = (iris_rx as f32 * 0.42).round().max(1.0) as i32;
        let pupil_ry = (iris_ry as f32 * 0.62).round().max(1.0) as i32;
        let icx = ex + gx;
        let icy = ey + gy;
        for dx in -erx - 1..=erx + 1 {
            let nx = dx as f32 / erx.max(1) as f32;
            if nx.abs() > 1.04 {
                continue;
            }
            let curve = (1.0 - nx.abs().powf(1.8)).max(0.0).powf(0.6);
            let top = (-ery as f32 * curve).round() as i32;
            let bot = (ery as f32 * curve).round() as i32;
            for dy in top..=bot {
                let idx = dx - gx;
                let idy = dy - gy;
                let im = (idx as f32 / iris_rx as f32).powi(2) + (idy as f32 / iris_ry as f32).powi(2);
                if im <= 1.0 {
                    let pm = (idx as f32 / pupil_rx as f32).powi(2) + (idy as f32 / pupil_ry as f32).powi(2);
                    if pm <= 1.0 {
                        pp_put(grid, ex + dx, ey + dy, '●', pupil);
                    } else {
                        pp_put(grid, ex + dx, ey + dy, '·', darken(ii, 14));
                    }
                } else {
                    pp_put(grid, ex + dx, ey + dy, ' ', sclera);
                }
            }
        }
        for i in 0..fibers {
            let a = i as f32 * TAU / fibers as f32;
            let p0 = pp_point_on(icx, icy, pupil_rx as f32 * 1.1, pupil_ry as f32 * 1.1, a);
            let p1 = pp_point_on(icx, icy, iris_rx as f32 * 0.96, iris_ry as f32 * 0.96, a);
            let col = if i % 2 == 0 { io } else { darken(ii, 6) };
            pp_line(grid, p0.0, p0.1, p1.0, p1.1, col);
        }
        pp_arc(grid, icx, icy, pupil_rx as f32 * 1.1, pupil_ry as f32 * 1.1, 0.0, TAU, pupil, 0);
        pp_put(grid, icx, icy, '◉', pupil);
        pp_put(grid, icx - iris_rx / 2, icy - iris_ry / 2, '˙', chalk);
        for dx in -erx - 1..=erx + 1 {
            let nx = dx as f32 / erx.max(1) as f32;
            if nx.abs() > 1.04 {
                continue;
            }
            let curve = (1.0 - nx.abs().powf(1.8)).max(0.0).powf(0.6);
            let top = (-ery as f32 * curve).round() as i32;
            let bot = (ery as f32 * curve).round() as i32;
            let cht = if dx < -erx / 2 { '╭' } else if dx > erx / 2 { '╮' } else { '─' };
            let chb = if dx < -erx / 2 { '╰' } else if dx > erx / 2 { '╯' } else { '─' };
            pp_put(grid, ex + dx, ey + top, cht, lighten(lid, 10));
            pp_put(grid, ex + dx, ey + bot, chb, darken(lid, 4));
        }
        pp_put(grid, ex - erx, ey, '<', lighten(lid, 6));
        pp_put(grid, ex + erx, ey, '>', lighten(lid, 6));
    };

    for ring in 0..2 {
        let count = if ring == 0 { 6 + seed as usize % 3 } else { 9 + seed as usize % 4 };
        let dist = if ring == 0 { 0.42 } else { 0.74 };
        for i in 0..count {
            let a = phase + i as f32 * TAU / count as f32 + ring as f32 * 0.3;
            let ex = (cx as f32 + a.cos() * max_rx * dist).round() as i32;
            let ey = (cy as f32 + a.sin() * max_ry * dist).round() as i32;
            if ex < 6 || ey < 3 || ex >= width as i32 - 6 || ey >= height as i32 - 3 {
                continue;
            }
            let (rx, ry) = if ring == 0 { (8, 4) } else { (5, 2) };
            let (gx, gy) = gaze_for(ex, ey, rx, ry);
            let io = shift_hue(iris_outer, (i as f64 * 47.0) % 160.0 - 80.0);
            let ii = shift_hue(iris_inner, (i as f64 * 33.0) % 120.0 - 60.0);
            let lid = shift_hue(lid_color, (i as f64 * 20.0) % 80.0 - 40.0);
            draw_eye(grid, ex, ey, rx, ry, gx, gy, io, ii, lid, 10 + i % 4);
        }
    }

    let hero_rx = ((width as f32 * 0.21).round() as i32).clamp(10, 24);
    let hero_ry = ((height as f32 * 0.17).round() as i32).clamp(3, 8);
    let hero_y = cy - (height as i32 / 14).max(1);
    let (hgx, hgy) = gaze_for(cx, hero_y, hero_rx, hero_ry);
    draw_eye(grid, cx, hero_y, hero_rx, hero_ry, hgx, hgy, iris_outer, iris_inner, lid_color, 20 + seed as usize % 8);
    pp_line(grid, cx - (hero_rx - 1), hero_y - hero_ry - 1, cx, hero_y - hero_ry - 3, darken(lid_color, 6));
    pp_line(grid, cx, hero_y - hero_ry - 3, cx + (hero_rx - 1), hero_y - hero_ry - 1, darken(lid_color, 6));

    pp_put(grid, lure_x, lure_y, '◆', lighten(lure_col, 12));
}

// --- fullmetal-eyes++ : the multi-tier seal cranked. 4 arc bands, 3-4 star
//     polygons, node eyes on EVERY tier vertex, twin rune bands, hero eye. ---
fn draw_fme_pp(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng) {
    use std::f32::consts::{FRAC_PI_2, TAU};
    let bg = darken(palette[0], 12);
    let chalk = lighten(palette[4], 12);
    let gold = lighten(palette[1], 30);
    let iris = lighten(palette[3], 30);
    let lid = lighten(palette[2], 18);
    let pupil = darken(palette[0], 2);
    let sclera = lighten(palette[4], 4);
    let shadow = darken(palette[2], 60);

    for y in 0..height {
        for x in 0..width {
            let n = (x * 19 + y * 43 + seed as usize * 5) % 151;
            let (ch, col) = match n {
                0 => ('·', shadow),
                1 => ('∙', shadow),
                _ => (' ', bg),
            };
            grid[y][x] = Cell::new(ch, col);
        }
    }

    let cx = width as i32 / 2;
    let cy = height as i32 / 2;
    let max_rx = (width as f32 / 2.0 - 4.0).max(12.0);
    let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
    let phase = seed as f32 * 0.031 - FRAC_PI_2;

    let lure_x = cx + (rng.random_range(-0.35f32..0.35) * max_rx * 0.9) as i32;
    let lure_y = cy + (rng.random_range(-0.30f32..0.30) * max_ry * 0.9) as i32;

    let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| -> (i32, i32) {
        let dx = (lure_x - ex) as f32;
        let dy = (lure_y - ey) as f32;
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        let gx = ((dx / d) * (rx as f32 * 0.34)).round() as i32;
        let gy = ((dy / d) * (ry as f32 * 0.6)).round() as i32;
        (
            gx.clamp(-(rx / 3).max(1), (rx / 3).max(1)),
            gy.clamp(-(ry / 2).max(1), (ry / 2).max(1)),
        )
    };

    let draw_hero_eye = |grid: &mut Grid, ex: i32, ey: i32, rx: i32, ry: i32, gx: i32, gy: i32, iris_c: Color, lid_c: Color| {
        for dx in -rx..=rx {
            let nx = dx as f32 / rx.max(1) as f32;
            let curve = (1.0 - nx.abs().powf(1.6)).max(0.0).powf(0.55);
            let top = (-(ry as f32) * curve).round() as i32;
            let bottom = ((ry as f32) * 0.78 * curve).round() as i32;
            for dy in top..=bottom {
                if dy == top || dy == bottom {
                    let ch = if dx < -rx / 2 {
                        if dy == top { '╱' } else { '╲' }
                    } else if dx > rx / 2 {
                        if dy == top { '╲' } else { '╱' }
                    } else {
                        '─'
                    };
                    pp_put(grid, ex + dx, ey + dy, ch, lid_c);
                } else {
                    let ix = (dx - gx) as f32 / (rx as f32 * 0.42);
                    let iy = (dy - gy) as f32 / ry.max(1) as f32;
                    let im = ix * ix + iy * iy;
                    if im <= 0.16 {
                        pp_put(grid, ex + dx, ey + dy, '◉', pupil);
                    } else if im <= 1.0 {
                        let ang = (iy.atan2(ix) + TAU) % TAU;
                        let fiber = ((ang / (TAU / 16.0)).round() as i32) % 2 == 0;
                        let ch = if im > 0.78 { '○' } else if fiber { '╎' } else { '·' };
                        pp_put(grid, ex + dx, ey + dy, ch, iris_c);
                    } else {
                        pp_put(grid, ex + dx, ey + dy, '·', sclera);
                    }
                }
            }
        }
        pp_put(grid, ex + gx - 1, ey + gy - 1, '˙', chalk);
        pp_put(grid, ex + gx, ey + gy - 1, '˙', chalk);
    };

    let draw_node_eye = |grid: &mut Grid, ncx: i32, ncy: i32, rx: i32, ry: i32, gx: i32, gy: i32, lid_c: Color, iris_c: Color| {
        for dx in -rx..=rx {
            let nx = dx as f32 / rx.max(1) as f32;
            let curve = (1.0 - nx.abs().powf(1.65)).max(0.0).powf(0.55);
            let top = (-(ry as f32) * curve).round() as i32;
            let bottom = ((ry as f32) * 0.75 * curve).round() as i32;
            for dy in top..=bottom {
                if dy == top || dy == bottom {
                    let ch = if dx < -rx / 2 {
                        if dy == top { '╱' } else { '╲' }
                    } else if dx > rx / 2 {
                        if dy == top { '╲' } else { '╱' }
                    } else {
                        '─'
                    };
                    pp_put(grid, ncx + dx, ncy + dy, ch, lid_c);
                } else {
                    let ix = (dx - gx) as f32 / (rx as f32 * 0.5);
                    let iy = (dy - gy) as f32 / ry.max(1) as f32;
                    let im = ix * ix + iy * iy;
                    if im <= 0.5 {
                        pp_put(grid, ncx + dx, ncy + dy, '◉', pupil);
                    } else if im <= 1.0 {
                        pp_put(grid, ncx + dx, ncy + dy, '·', iris_c);
                    }
                }
            }
        }
        pp_put(grid, ncx + gx - 1, ncy + gy - 1, '˙', chalk);
    };

    let runes = [
        '△', '▽', '□', '◇', '☉', '☽', '☿', '♄', '♃', '✦', '∴', '∵', '⊕', '⊗', '✶', '◈',
    ];

    let band_count = 4;
    for i in 0..band_count {
        let t = i as f32 / (band_count - 1) as f32;
        let r = 0.46 + t * 0.46;
        let col = match i % 3 {
            0 => chalk,
            1 => gold,
            _ => iris,
        };
        let gap = if i % 3 == 2 { 9 } else { 0 };
        pp_arc(grid, cx, cy, max_rx * r, max_ry * r, phase, phase + TAU, col, gap);
    }

    let tier_count = 3 + seed as usize % 2;
    let mut tier_verts: Vec<Vec<(i32, i32)>> = Vec::new();
    for ti in 0..tier_count {
        let rad = 0.40 + ti as f32 * 0.15;
        let n = 5 + (seed as usize + ti * 7) % 6;
        let rot = phase + ti as f32 * 0.4 + seed as f32 * 0.01 * ti as f32;
        let mut verts: Vec<(i32, i32)> = Vec::with_capacity(n);
        for i in 0..n {
            let a = rot + i as f32 * TAU / n as f32;
            verts.push(pp_point_on(cx, cy, max_rx * rad, max_ry * rad, a));
        }
        let max_k = ((n - 1) / 2).max(1);
        let k = if max_k >= 2 { 2 + (seed as usize + ti) % (max_k - 1) } else { 1 };
        let col = if ti % 2 == 0 { gold } else { chalk };
        for i in 0..n {
            let (ax, ay) = verts[i];
            let (bx, by) = verts[(i + k.min(max_k)) % n];
            pp_line(grid, ax, ay, bx, by, darken(col, 8));
        }
        for &v in &verts {
            pp_put(grid, v.0, v.1, '◇', lighten(col, 8));
        }
        tier_verts.push(verts);
    }

    for (bi, &r) in [0.92f32, 0.62].iter().enumerate() {
        let ins_n = ((max_rx + max_ry) * r * 0.5).round().clamp(14.0, 40.0) as usize;
        for i in 0..ins_n {
            let a = phase + i as f32 * TAU / ins_n as f32;
            let p = pp_point_on(cx, cy, max_rx * r, max_ry * r, a);
            pp_put(grid, p.0, p.1, runes[(i + seed as usize + bi * 3) % runes.len()], lighten(gold, 8));
        }
    }

    for (ti, verts) in tier_verts.iter().enumerate() {
        let (rx, ry) = if ti + 1 == tier_count { (5, 2) } else { (4, 2) };
        for (i, &(nx, ny)) in verts.iter().enumerate() {
            let (gx, gy) = gaze_for(nx, ny, rx, ry);
            draw_node_eye(grid, nx, ny, rx, ry, gx, gy, darken(lid, 6), shift_hue(iris, (i as f64 * 40.0 + ti as f64 * 17.0) % 360.0));
            pp_put(grid, nx, ny + ry + 1, runes[(i + ti) % runes.len()], lighten(gold, 12));
        }
    }

    let hero_rx = (width as f32 * 0.13) as i32;
    let hero_ry = (height as f32 * 0.15) as i32;
    let (hgx, hgy) = gaze_for(cx, cy, hero_rx, hero_ry);
    draw_hero_eye(grid, cx, cy, hero_rx, hero_ry, hgx, hgy, lighten(iris, 16), lighten(lid, 10));

    pp_arc(grid, lure_x, lure_y, 3.0, 1.5, 0.0, TAU, shift_hue(gold, 40.0), 4);
    pp_put(grid, lure_x, lure_y, '◆', lighten(gold, 20));

    for _ in 0..(tier_count + 2) {
        let a = phase + rng.random::<f32>() * TAU;
        let p1 = pp_point_on(cx, cy, max_rx * rng.random_range(0.22..0.45), max_ry * rng.random_range(0.22..0.45), a);
        let p2 = pp_point_on(cx, cy, max_rx * rng.random_range(0.60..0.90), max_ry * rng.random_range(0.60..0.90), a + rng.random_range(0.3..1.3));
        pp_line(grid, p1.0, p1.1, p2.0, p2.1, darken(iris, 18));
    }
}

// --- trees++ : a lush grounded gallery of tree variants on grassy hillocks,
//     varied spreads + hues, fruit/flower accents, no debug labels. ---
fn draw_trees_pp(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng) {
    let cols = (width / 18).clamp(3, 6);
    let rows = (height / 14).clamp(2, 4);
    let cell_w = width / cols;
    let cell_h = height / rows;
    let grass = darken(palette[2], 30);
    let gc = ['v', 'w', '\u{2c4}', '\u{1d1b}'];

    for row in 0..rows {
        for col in 0..cols {
            let kind = (row * cols + col + seed as usize) % 19;
            let cx = col * cell_w + cell_w / 2;
            let ground_y = (row + 1) * cell_h - 2;
            let canopy_y = row * cell_h + 2;
            let spread = (cell_w / 4).max(3);
            // grass line under the tree
            for x in (col * cell_w)..((col + 1) * cell_w).min(width) {
                let gy = ground_y + 1;
                if gy < height {
                    grid[gy][x] = Cell::new(gc[(x + row) % gc.len()], grass);
                }
            }
            let base = palette[1 + kind % 3];
            let color = shift_hue(base, (kind as f64 * 23.0) % 120.0 - 60.0);
            draw_tree(grid, cx, ground_y, canopy_y, spread, kind, color, rng);
            // accent: fruit hanging in canopy or flower at the base
            if (row + col + seed as usize) % 2 == 0 {
                draw_flower(grid, (cx + spread).min(width.saturating_sub(1)), ground_y.saturating_sub(1), kind % 5, palette[3]);
            } else {
                draw_fruit(grid, cx.saturating_sub(spread / 2), (canopy_y + 3).min(height.saturating_sub(1)), kind % 5, lighten(palette[3], 10));
            }
        }
    }
}

// --- forest++ : layered depth. star sky + disc, far dark pines, mid mix,
//     foreground hero trees, grass band, scattered flowers/fruit. ---
fn draw_forest_pp(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng) {
    let ground_y = height.saturating_sub(4);
    let ground_color = darken(palette[1], 90);
    let tiles = ['╱', '╲'];

    // sky (sparse) + ground (truchet)
    for y in 0..height {
        for x in 0..width {
            if y >= ground_y {
                grid[y][x] = Cell::new(tiles[(x + y) % 2], ground_color);
            } else {
                grid[y][x] = Cell::blank();
            }
        }
    }
    // stars in upper sky
    for _ in 0..(width / 2) {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..(ground_y / 2).max(1));
        let ch = if rng.random_range(0..4) == 0 { '✦' } else { '·' };
        grid[y][x] = Cell::new(ch, darken(palette[4], 60));
    }
    // sun / moon disc
    let disc_cx = (width / 5 + seed as usize % (width / 2).max(1)) as i32;
    let disc_cy = (2 + seed as usize % 3) as i32;
    let disc_col = lighten(palette[3], 14);
    for dy in -2i32..=2 {
        for dx in -4i32..=4 {
            let m = (dx as f32 / 4.0).powi(2) + (dy as f32 / 2.0).powi(2);
            if m <= 1.0 {
                pp_put(grid, disc_cx + dx, disc_cy + dy, '·', disc_col);
            }
        }
    }
    pp_arc(grid, disc_cx, disc_cy, 4.0, 2.0, 0.0, std::f32::consts::TAU, lighten(disc_col, 10), 0);

    // far hills: small, desaturated, dark pines on the horizon
    let far_color = darken(palette[2], 40);
    let mut x = 2usize;
    while x < width.saturating_sub(2) {
        let h = 4 + (x + seed as usize) % 3;
        draw_pine(grid, x, ground_y.saturating_sub(1), 3, h, far_color);
        x += 4 + (x + seed as usize) % 3;
    }

    // mid trees: medium mixed, slightly darkened
    let mid_color = darken(palette[1], 30);
    let mid_xs = [width / 6, width / 3, width / 2, (width * 2) / 3, (width * 5) / 6];
    for (i, &mx) in mid_xs.iter().enumerate() {
        let canopy = ground_y.saturating_sub(8);
        match (i + seed as usize) % 3 {
            0 => draw_pine(grid, mx, ground_y.saturating_sub(1), 4, 8, mid_color),
            1 => grow_tree(grid, mx, ground_y.saturating_sub(1), canopy, 4, mid_color, rng),
            _ => draw_palm(grid, mx, ground_y.saturating_sub(1), 9, darken(palette[3], 20), rng),
        }
    }

    // foreground hero trees (clear bounding boxes first; willow needs blanks)
    let clear = |grid: &mut Grid, x0: usize, x1: usize, y0: usize, y1: usize| {
        for yy in y0..y1.min(height) {
            for xx in x0..x1.min(width) {
                if yy < ground_y {
                    grid[yy][xx] = Cell::blank();
                }
            }
        }
    };
    let fg_a = width / 6;
    clear(grid, fg_a.saturating_sub(8), fg_a + 8, ground_y.saturating_sub(14), ground_y);
    grow_tree(grid, fg_a, ground_y.saturating_sub(1), ground_y.saturating_sub(13), 6, palette[1], rng);

    let fg_b = width / 2;
    draw_pine(grid, fg_b, ground_y.saturating_sub(1), 5, 12, palette[2]);

    let fg_c = (width * 3) / 4;
    clear(grid, fg_c.saturating_sub(9), fg_c + 9, ground_y.saturating_sub(16), ground_y);
    draw_willow(grid, fg_c, ground_y.saturating_sub(1), ground_y.saturating_sub(14), 7, palette[1]);

    let fg_d = width.saturating_sub(8);
    draw_palm(grid, fg_d, ground_y.saturating_sub(1), 15, palette[3], rng);

    // undergrowth: flowers + fallen fruit
    for _ in 0..(width / 6) {
        let fx = rng.random_range(1..width.saturating_sub(1));
        let fy = ground_y.saturating_sub(1);
        if rng.random_range(0..2) == 0 {
            draw_flower(grid, fx, fy, rng.random_range(0..5), palette[3]);
        } else {
            draw_fruit(grid, fx, ground_y, rng.random_range(0..5), rgb(200, 60, 50));
        }
    }
}

// --- phyllotaxis : golden-angle sunflower spiral; glyph scales with radius,
//     color ramps outward through the palette. ---
fn draw_phyllotaxis(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32) {
    let bg = darken(palette[0], 6);
    for y in 0..height {
        for x in 0..width {
            grid[y][x] = Cell::new(' ', bg);
        }
    }
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let golden = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
    let n = 520 + (seed as usize % 280);
    let sx = width as f32 * 0.47 / (n as f32).sqrt();
    let sy = height as f32 * 0.47 / (n as f32).sqrt();
    // t rotates the whole spiral (the florets wheel around the center).
    let rot = (seed as f32 % 360.0).to_radians() + rng.random_range(0.0f32..0.5) + t * 0.15;
    let glyphs = ['·', '∙', '•', '◦', '○', '◌', '✦', '◆', '❀', '✺'];
    for i in 0..n {
        let a = i as f32 * golden + rot;
        let rr = (i as f32).sqrt();
        let x = (cx + a.cos() * sx * rr).round() as i32;
        let y = (cy + a.sin() * sy * rr).round() as i32;
        let t = i as f32 / n as f32;
        let mid = lerp_color(palette[3], palette[1], (t * 2.0).min(1.0));
        let col = lerp_color(mid, palette[2], (t * 2.0 - 1.0).max(0.0));
        let gi = ((1.0 - t) * (glyphs.len() - 1) as f32).round() as usize;
        pp_put(grid, x, y, glyphs[gi.min(glyphs.len() - 1)], col);
    }
    pp_put(grid, cx as i32, cy as i32, '❁', lighten(palette[3], 20));
}

// --- moire : two radial sine gratings interfering; shade ramp + color blend. ---
fn draw_moire(grid: &mut Grid, width: usize, height: usize, _seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32) {
    let ramp = [' ', '·', ':', '-', '=', '+', '*', '#', '%', '@'];
    // t drifts the two centers in a slow orbit so the interference fringes flow.
    // Offsets use (cos-1, sin) so they're exactly zero at t=0 (snapshot identity).
    let ax = width as f32 * rng.random_range(0.2..0.4) + ((t * 0.7).cos() - 1.0) * width as f32 * 0.05;
    let ay = height as f32 * rng.random_range(0.3..0.6) + (t * 0.7).sin() * height as f32 * 0.05;
    let bx = width as f32 * rng.random_range(0.6..0.8) - ((t * 0.6).cos() - 1.0) * width as f32 * 0.05;
    let by = height as f32 * rng.random_range(0.4..0.7) - (t * 0.6).sin() * height as f32 * 0.05;
    let f1 = rng.random_range(0.5f32..0.95);
    let f2 = rng.random_range(0.5f32..0.95);
    for y in 0..height {
        for x in 0..width {
            let dx1 = x as f32 - ax;
            let dy1 = (y as f32 - ay) * 2.0;
            let dx2 = x as f32 - bx;
            let dy2 = (y as f32 - by) * 2.0;
            let d1 = (dx1 * dx1 + dy1 * dy1).sqrt();
            let d2 = (dx2 * dx2 + dy2 * dy2).sqrt();
            let v = (d1 * f1 * 0.5).sin() + (d2 * f2 * 0.5).sin();
            let t = (v + 2.0) / 4.0;
            let idx = (t * (ramp.len() - 1) as f32).round() as usize;
            let col = lerp_color(palette[1], palette[3], t);
            grid[y][x] = Cell::new(ramp[idx.min(ramp.len() - 1)], col);
        }
    }
}

fn pp_hash2(x: i32, y: i32, seed: u64) -> f32 {
    let mut h = (x as i64)
        .wrapping_mul(374761393)
        ^ (y as i64).wrapping_mul(668265263)
        ^ (seed as i64).wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h & 0xffff) as f32) / 65535.0
}

fn pp_vnoise(fx: f32, fy: f32, seed: u64) -> f32 {
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let n00 = pp_hash2(x0, y0, seed);
    let n10 = pp_hash2(x0 + 1, y0, seed);
    let n01 = pp_hash2(x0, y0 + 1, seed);
    let n11 = pp_hash2(x0 + 1, y0 + 1, seed);
    let a = n00 + (n10 - n00) * sx;
    let b = n01 + (n11 - n01) * sx;
    a + (b - a) * sy
}

fn pp_fbm(fx: f32, fy: f32, seed: u64) -> f32 {
    let mut v = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    for o in 0..4u64 {
        v += amp * pp_vnoise(fx * freq, fy * freq, seed.wrapping_add(o * 101));
        amp *= 0.5;
        freq *= 2.0;
    }
    v
}

// --- nebula : fbm cloud field with a shade ramp, palette gradient, scattered
//     stars in the dark voids. ---
fn draw_nebula(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32) {
    let ramp = [' ', ' ', '·', '∙', ':', '*', '▒', '▓'];
    // t pans the cloud field; the starfield (rng-placed) stays put behind it.
    for y in 0..height {
        for x in 0..width {
            let fx = x as f32 / 11.0 + t * 0.08;
            let fy = y as f32 / 5.5;
            let n = pp_fbm(fx, fy, seed);
            let t = ((n - 0.25) * 1.7).clamp(0.0, 1.0);
            let idx = (t * (ramp.len() - 1) as f32).round() as usize;
            let body = lerp_color(darken(palette[0], 4), palette[2], t);
            let col = lerp_color(body, palette[3], t * t);
            grid[y][x] = Cell::new(ramp[idx.min(ramp.len() - 1)], col);
        }
    }
    let star_count = (width * height) / 36;
    for _ in 0..star_count {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);
        let n = pp_fbm(x as f32 / 11.0, y as f32 / 5.5, seed);
        if n < 0.42 {
            let ch = match rng.random_range(0..3) {
                0 => '✦',
                1 => '✧',
                _ => '·',
            };
            grid[y][x] = Cell::new(ch, lighten(palette[4], 0));
        }
    }
}

// --- delta : recursive branching river/lightning system fanning down-screen. ---
fn draw_solar_system(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // solar-system [bodies] -- 3D-ish orbital diagram with planets, cubes, and space hardware
        let body_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(8);
        let body_count = body_count.clamp(3, 12);

        macro_rules! set_cell {
            ($x:expr, $y:expr, $ch:expr, $fg:expr) => {{
                let sx = $x;
                let sy = $y;
                if sx >= 0 && sy >= 0 && (sx as usize) < width && (sy as usize) < height {
                    grid[sy as usize][sx as usize] = Cell::new($ch, $fg);
                }
            }};
        }

        let space = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &space,
            NoiseVariant::Dot,
            darken(palette[2], 94),
            darken(palette[3], 88),
            &mut rng,
        );

        for _ in 0..(width * height / 36).max(8) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);
            let ch = ['·', '∙', '°', '*', '✦'][rng.random_range(0..5usize)];
            grid[y][x] = Cell::new(
                ch,
                darken(lighten(palette[4], 10), rng.random_range(15..75)),
            );
        }

        let center_phase = seed as f32 * 0.073 + t_anim * 0.25;
        let center_x_ratio = (0.47 + center_phase.sin() * 0.12).clamp(0.34, 0.62);
        let center_y_ratio = (0.51 + (center_phase * 1.41).cos() * 0.14).clamp(0.34, 0.66);
        let cx = width as f32 * center_x_ratio;
        let cy = height as f32 * center_y_ratio;
        let max_rx = ((cx - 3.0).min(width as f32 - cx - 3.0))
            .max(12.0)
            .min(width as f32 * 0.46);
        let min_rx = (width as f32 * 0.10).max(5.0);
        let max_ry = ((cy - 2.0).min(height as f32 - cy - 2.0))
            .max(4.0)
            .min(height as f32 * 0.38);
        let min_ry = (height as f32 * 0.08).max(2.0).min(max_ry * 0.55);
        let orbit_count = body_count.min(10);
        let sun_rx = (width as f32 / 16.0).clamp(4.0, 8.0);
        let sun_ry = (height as f32 / 8.0).clamp(2.0, 4.0);

        // Perspective orbital plane: rear arcs now complete behind the solar sphere.
        for i in 0..orbit_count {
            let t = i as f32 / orbit_count.max(1) as f32;
            let rx = min_rx + (max_rx - min_rx) * t;
            let ry = min_ry + (max_ry - min_ry) * t;
            let tilt = (i as f32 - orbit_count as f32 * 0.5) * 0.22;
            for s in 0..360 {
                if s % 3 == 1 && i > 6 {
                    continue;
                }
                let a = s as f32 / 360.0 * std::f32::consts::TAU;
                let x = cx + a.cos() * rx + a.sin() * tilt;
                let y = cy + a.sin() * ry;
                if x < 0.0 || y < 0.0 || x >= width as f32 || y >= height as f32 {
                    continue;
                }
                let near = a.sin() > 0.0;
                let ch = if near {
                    if s % 7 == 0 { '═' } else { '─' }
                } else if s % 11 == 0 {
                    '∙'
                } else if s % 4 == 0 {
                    '·'
                } else {
                    ' '
                };
                if ch != ' ' {
                    let color = if near {
                        darken(palette[4], 45)
                    } else {
                        darken(palette[2], 72)
                    };
                    set_cell!(x.round() as i32, y.round() as i32, ch, color);
                }
            }
        }

        // Solar sphere with shaded cells.
        for yy in (cy - sun_ry - 1.0).floor() as i32..=(cy + sun_ry + 1.0).ceil() as i32 {
            for xx in (cx - sun_rx - 1.0).floor() as i32..=(cx + sun_rx + 1.0).ceil() as i32 {
                let dx = (xx as f32 - cx) / sun_rx;
                let dy = (yy as f32 - cy) / sun_ry;
                let d = (dx * dx + dy * dy).sqrt();
                if d > 1.0 {
                    continue;
                }
                let ch = if d < 0.28 {
                    '◉'
                } else if dx < -0.2 || dy > 0.35 {
                    '▒'
                } else if d > 0.78 {
                    '░'
                } else {
                    '●'
                };
                let color = if d < 0.35 {
                    lighten(palette[3], 35)
                } else if dx < -0.2 || dy > 0.35 {
                    darken(palette[3], 25)
                } else {
                    lighten(palette[3], 10)
                };
                set_cell!(xx, yy, ch, color);
            }
        }

        // Planets, moons, and little labels/ticks.
        let planet_glyphs = ['●', '◐', '◑', '◉', '◆', '○'];
        for i in 0..body_count {
            let t = i as f32 / body_count.max(1) as f32;
            let rx = min_rx + (max_rx - min_rx) * (0.12 + t * 0.88);
            let ry = min_ry + (max_ry - min_ry) * (0.12 + t * 0.88);
            let angle = seed as f32 * 0.017 + i as f32 * 1.37 + rng.random::<f32>() * 0.35;
            let px = cx + angle.cos() * rx + angle.sin() * (i as f32 - 3.0) * 0.18;
            let py = cy + angle.sin() * ry;
            let radius = match i % 5 {
                0 => 1,
                1 => 2,
                2 => 1,
                3 => 3,
                _ => 2,
            };
            let color = shift_hue(lighten(palette[1 + i % 3], 12), (i * 37) as f64);

            if radius == 1 {
                set_cell!(
                    px.round() as i32,
                    py.round() as i32,
                    planet_glyphs[i % planet_glyphs.len()],
                    color
                );
            } else {
                for dy in -(radius as i32)..=(radius as i32) {
                    for dx in -(radius as i32 * 2)..=(radius as i32 * 2) {
                        let nx = dx as f32 / (radius as f32 * 2.0);
                        let ny = dy as f32 / radius as f32;
                        if nx * nx + ny * ny > 1.0 {
                            continue;
                        }
                        let shade = nx * -0.7 + ny * 0.45;
                        let ch = if shade > 0.35 {
                            '░'
                        } else if shade < -0.35 {
                            '▓'
                        } else if dx == 0 && dy == 0 {
                            '◉'
                        } else {
                            '●'
                        };
                        let fg = if shade > 0.35 {
                            darken(color, 35)
                        } else if shade < -0.35 {
                            lighten(color, 25)
                        } else {
                            color
                        };
                        set_cell!(px.round() as i32 + dx, py.round() as i32 + dy, ch, fg);
                    }
                }
            }

            if i % 3 == 1 {
                let moon_angle = angle * 1.9 + 0.8;
                let mx = px + moon_angle.cos() * (radius as f32 * 3.2 + 3.0);
                let my = py + moon_angle.sin() * (radius as f32 * 1.3 + 1.4);
                set_cell!(
                    mx.round() as i32,
                    my.round() as i32,
                    '○',
                    lighten(palette[4], 5)
                );
                set_cell!(
                    ((px + mx) * 0.5).round() as i32,
                    ((py + my) * 0.5).round() as i32,
                    '·',
                    darken(palette[4], 45)
                );
            }

            if i % 4 == 2 {
                let lx = px.round() as i32 + radius as i32 * 2 + 2;
                let ly = py.round() as i32;
                for (j, ch) in format!("p{}", i + 1).chars().enumerate() {
                    set_cell!(lx + j as i32, ly, ch, darken(palette[4], 38));
                }
            }
        }

        // Isometric orbital stations: seed-driven boxes riding different orbital lanes.
        let station_count = 2 + (seed as usize % 2);
        for s in 0..station_count {
            let lane = (0.56 + s as f32 * 0.17).min(0.92);
            let station_angle = center_phase * (0.52 + s as f32 * 0.21)
                + s as f32 * std::f32::consts::TAU / station_count as f32
                + 0.85;
            let sx = cx + station_angle.cos() * max_rx * lane;
            let sy = cy + station_angle.sin() * max_ry * (0.66 + s as f32 * 0.08);
            let cube_w = (width as i32 / (10 + s as i32)).clamp(6, 13);
            let cube_h = (height as i32 / (5 + s as i32)).clamp(4, 8);
            let max_cube_x = (width as i32 - cube_w - 7).max(1);
            let max_cube_y = (height as i32 - cube_h - 4).max(2);
            let cube_x = (sx.round() as i32 - cube_w / 2).clamp(1, max_cube_x);
            let cube_y = (sy.round() as i32 - cube_h / 2).clamp(2, max_cube_y);
            let off_x: i32 = if sx >= cx { 4 } else { -4 };
            let off_y: i32 = if s % 2 == 0 { -2 } else { 2 };
            let back_x = cube_x + off_x;
            let back_y = cube_y + off_y;
            let cube_color = shift_hue(lighten(palette[2], 25), s as f64 * 46.0);
            let back_color = darken(cube_color, 18);

            for x in 0..=cube_w {
                set_cell!(cube_x + x, cube_y, '─', cube_color);
                set_cell!(cube_x + x, cube_y + cube_h, '─', cube_color);
                set_cell!(back_x + x, back_y, '─', back_color);
                set_cell!(back_x + x, back_y + cube_h, '─', back_color);
            }
            for y in 0..=cube_h {
                set_cell!(cube_x, cube_y + y, '│', cube_color);
                set_cell!(cube_x + cube_w, cube_y + y, '│', cube_color);
                set_cell!(back_x, back_y + y, '│', back_color);
                set_cell!(back_x + cube_w, back_y + y, '│', back_color);
            }
            for &(x, y, ch) in &[
                (cube_x, cube_y, '┌'),
                (cube_x + cube_w, cube_y, '┐'),
                (cube_x, cube_y + cube_h, '└'),
                (cube_x + cube_w, cube_y + cube_h, '┘'),
                (back_x, back_y, '┌'),
                (back_x + cube_w, back_y, '┐'),
                (back_x, back_y + cube_h, '└'),
                (back_x + cube_w, back_y + cube_h, '┘'),
            ] {
                set_cell!(x, y, ch, lighten(cube_color, 10));
            }

            let connector = if off_x > 0 { '╱' } else { '╲' };
            for k in 1..=off_x.abs() {
                let dx = if off_x > 0 { k } else { -k };
                let dy = off_y * k / off_x.abs();
                set_cell!(cube_x + dx, cube_y + dy, connector, darken(cube_color, 5));
                set_cell!(
                    cube_x + cube_w + dx,
                    cube_y + dy,
                    connector,
                    darken(cube_color, 5)
                );
                set_cell!(
                    cube_x + dx,
                    cube_y + cube_h + dy,
                    connector,
                    darken(cube_color, 5)
                );
                set_cell!(
                    cube_x + cube_w + dx,
                    cube_y + cube_h + dy,
                    connector,
                    darken(cube_color, 5)
                );
            }
            for y in 1..cube_h {
                for x in 1..cube_w {
                    if (x * 2 + y + s as i32) % 4 == 0 {
                        set_cell!(cube_x + x, cube_y + y, '▪', darken(cube_color, 30));
                    }
                }
            }

            let dock_dir: i32 = if sx < cx { 1 } else { -1 };
            let dock_y = cube_y + cube_h / 2;
            for k in 1..=5 {
                set_cell!(
                    cube_x + if dock_dir > 0 { cube_w + k } else { -k },
                    dock_y,
                    '─',
                    darken(cube_color, 20)
                );
            }
            set_cell!(
                cube_x + if dock_dir > 0 { cube_w + 6 } else { -6 },
                dock_y,
                '◇',
                lighten(palette[3], 20)
            );
        }

        // Solar panel squares and a probe mast, also attached to a seed-shifting lane.
        let panel_angle = (center_phase * 0.88 - 0.20).rem_euclid(std::f32::consts::TAU);
        let panel_anchor_x = cx + panel_angle.cos() * max_rx * 0.78;
        let panel_anchor_y = cy + panel_angle.sin() * max_ry * 0.86;
        let panel_x_max = (width as i32 - 32).max(2);
        let panel_y_max = (height as i32 - 7).max(2);
        let panel_x = (panel_anchor_x.round() as i32 - 13).clamp(2, panel_x_max);
        let panel_y = (panel_anchor_y.round() as i32 - 2).clamp(2, panel_y_max);
        for p in 0..3 {
            let x0 = panel_x + p * 9;
            let y0 = panel_y + if p % 2 == 0 { 0 } else { -1 };
            for x in 0..7 {
                set_cell!(x0 + x, y0, '─', lighten(palette[1], 20));
                set_cell!(x0 + x, y0 + 4, '─', lighten(palette[1], 20));
            }
            for y in 0..=4 {
                set_cell!(x0, y0 + y, '│', lighten(palette[1], 20));
                set_cell!(x0 + 7, y0 + y, '│', lighten(palette[1], 20));
            }
            set_cell!(x0, y0, '┌', lighten(palette[1], 35));
            set_cell!(x0 + 7, y0, '┐', lighten(palette[1], 35));
            set_cell!(x0, y0 + 4, '└', lighten(palette[1], 35));
            set_cell!(x0 + 7, y0 + 4, '┘', lighten(palette[1], 35));
            for x in 1..7 {
                for y in 1..4 {
                    if (x + y + p) % 2 == 0 {
                        set_cell!(x0 + x, y0 + y, '□', darken(palette[1], 15));
                    }
                }
            }
        }
        let mast_x = panel_x + 27;
        for y in panel_y - 5..=panel_y + 2 {
            set_cell!(mast_x, y, '│', palette[4]);
        }
        set_cell!(mast_x, panel_y - 6, '◇', lighten(palette[3], 25));
        set_cell!(mast_x - 1, panel_y - 3, '╱', palette[4]);
        set_cell!(mast_x + 1, panel_y - 3, '╲', palette[4]);

        // Perspective rays from the star through the orbital plane.
        for ray in -3..=3 {
            let angle = ray as f32 * 0.18 + 0.9;
            for step in 6..(width / 3).max(8) {
                let x = cx as i32 + (angle.cos() * step as f32 * 1.8).round() as i32;
                let y = cy as i32 + (angle.sin() * step as f32 * 0.55).round() as i32;
                if x >= 0
                    && y >= 0
                    && (x as usize) < width
                    && (y as usize) < height
                    && grid[y as usize][x as usize].ch == ' '
                    && step % 4 == 0
                {
                    set_cell!(x, y, '·', darken(palette[3], 55));
                }
            }
        }
    grid
}

fn draw_eyes3(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // eyes3 [rays=0] [satellites=0] -- radiant all-seeing eye in a stepped pyramid
        let ray_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let ray_count = if ray_arg == 0 {
            18 + (seed as usize % 14)
        } else {
            ray_arg.clamp(8, 48)
        };
        let sat_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let sat_count = if sat_arg == 0 {
            2 + (seed as usize % 3)
        } else {
            sat_arg.clamp(0, 6)
        };

        let bg = darken(palette[0], 14);
        let chalk = lighten(palette[4], 14);
        let gold = lighten(palette[1], 32);
        let iris_outer = shift_hue(lighten(palette[3], 26), 18.0);
        let iris_inner = lighten(palette[2], 20);
        let lid_color = lighten(palette[1], 18);
        let pupil = darken(palette[0], 2);
        let sclera = lighten(palette[4], 6);
        let ray_color = lighten(palette[4], 18);
        let hush = darken(palette[2], 60);
        let base_c = darken(gold, 8);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 17 + y * 29 + seed as usize * 3) % 97;
                let ch = match n {
                    0 => '·',
                    1 if (x + y) % 3 == 0 => '∙',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, hush)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid,
                         mut x0: i32,
                         mut y0: i32,
                         x1: i32,
                         y1: i32,
                         fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let curve_char = |prev: (i32, i32), here: (i32, i32), next: (i32, i32)| {
            let dx1 = (here.0 - prev.0).signum();
            let dy1 = (here.1 - prev.1).signum();
            let dx2 = (next.0 - here.0).signum();
            let dy2 = (next.1 - here.1).signum();
            if (dx1, dy1) == (dx2, dy2) {
                if dy1 == 0 {
                    '─'
                } else if dx1 == 0 {
                    '│'
                } else if dx1 == dy1 {
                    '╲'
                } else {
                    '╱'
                }
            } else if dy1 == 0 && dx2 == 0 {
                match (dx1, dy2) {
                    (1, 1) => '╮',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╰',
                    _ => '╮',
                }
            } else if dx1 == 0 && dy2 == 0 {
                match (dy1, dx2) {
                    (1, 1) => '╰',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╮',
                    _ => '╰',
                }
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else if dx2 == dy2 {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color| {
            let span = (end - start).abs().max(0.05);
            let samples = ((rx + ry) * span * 3.8).max(18.0) as usize;
            let mut pts: Vec<(i32, i32)> = Vec::new();
            for i in 0..=samples {
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if pts.last().copied() != Some(p) {
                    pts.push(p);
                }
            }
            if pts.len() > 2 {
                for p in 1..pts.len() - 1 {
                    let ch = curve_char(pts[p - 1], pts[p], pts[p + 1]);
                    put(grid, pts[p].0, pts[p].1, ch, fg);
                }
            }
        };
        let draw_small_eye = |grid: &mut Grid,
                              ex: i32,
                              ey: i32,
                              rx: i32,
                              ry: i32,
                              iris_color: Color,
                              lid: Color| {
            let rx = rx.max(3);
            let ry = ry.max(1);
            let iris_rx = (rx / 3).max(1);
            let iris_ry = (ry / 2).max(1);
            for dy in -ry - 1..=ry + 1 {
                for dx in -rx - 1..=rx + 1 {
                    let nx = dx as f32 / rx as f32;
                    let ny = dy as f32 / ry as f32;
                    let metric = nx * nx + ny * ny;
                    if metric > 1.30 {
                        continue;
                    }
                    let edge = (metric - 1.0).abs();
                    let x = ex + dx;
                    let y = ey + dy;
                    if edge < 0.24 {
                        let ch = if dy < -ry / 3 {
                            if dx < -rx / 2 { '╭' } else if dx > rx / 2 { '╮' } else { '─' }
                        } else if dy > ry / 3 {
                            if dx < -rx / 2 { '╰' } else if dx > rx / 2 { '╯' } else { '─' }
                        } else if dx < 0 {
                            '╱'
                        } else if dx > 0 {
                            '╲'
                        } else {
                            '│'
                        };
                        put(grid, x, y, ch, lid);
                        continue;
                    }
                    let im = (dx as f32 / iris_rx as f32).powi(2)
                        + (dy as f32 / iris_ry as f32).powi(2);
                    if im <= 1.0 {
                        put(grid, x, y, '●', pupil);
                    } else {
                        put(grid, x, y, ' ', sclera);
                    }
                }
            }
            put(grid, ex - iris_rx / 2, ey - iris_ry / 2, '˙', chalk);
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 1.0).max(10.0);
        let max_ry = (height as f32 / 2.0 - 1.0).max(5.0);
        let phase = rng.random_range(0.0..std::f32::consts::TAU) + t_anim * 0.15;

        // radiating light rays from center (alternating long/short)
        for i in 0..ray_count {
            let a = phase + i as f32 * std::f32::consts::TAU / ray_count as f32;
            let long = i % 2 == 0;
            let r0 = if long { 7.0 } else { 5.0 };
            let r1 = if long { 0.98 } else { 0.66 };
            let p0 = point_on(cx, cy, r0, r0 * 0.5, a);
            let p1 = point_on(cx, cy, max_rx * r1, max_ry * r1, a);
            draw_line(
                &mut grid,
                p0.0,
                p0.1,
                p1.0,
                p1.1,
                darken(ray_color, if long { 8 } else { 28 }),
            );
            if long {
                put(&mut grid, p1.0, p1.1, '◇', darken(ray_color, 18));
            }
        }

        // faint halo arcs behind the eye
        for r in [0.46_f32, 0.58, 0.72] {
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * r,
                max_ry * r,
                0.0,
                std::f32::consts::TAU,
                darken(gold, 28 + (r * 18.0) as u8),
            );
        }

        // focal lure (seeded) -- eyes track it for per-seed variation
        let lure_x = (cx + rng.random_range(-(width as i32 / 6)..=(width as i32 / 6)))
            .clamp(8, width as i32 - 8);
        let lure_y = (cy + rng.random_range(-(height as i32 / 5)..=(height as i32 / 5)))
            .clamp(6, height as i32 - 4);
        let lure_col = shift_hue(lighten(iris_inner, 26), rng.random_range(-40..=40) as f64);
        for dy in -2i32..=2 {
            for dx in -4i32..=4 {
                let m = (dx as f32 / 4.0).powi(2) + (dy as f32 / 2.0).powi(2);
                if m <= 1.0 && (dx.abs() + dy.abs()) % 2 == 0 {
                    put(&mut grid, lure_x + dx, lure_y + dy, '·', darken(lure_col, 24));
                }
            }
        }

        // gaze helper: iris offset toward lure, clamped to eye size
        let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| -> (i32, i32) {
            let dx = (lure_x - ex) as f32;
            let dy = (lure_y - ey) as f32;
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let gx = ((dx / d) * (rx as f32 * 0.30)).round() as i32;
            let gy = ((dy / d) * (ry as f32 * 0.55)).round() as i32;
            (
                gx.clamp(-(rx / 3).max(1), (rx / 3).max(1)),
                gy.clamp(-(ry / 2).max(1), (ry / 2).max(1)),
            )
        };
        // equilateral triangle, drawn via 3 line edges (rotatable -> "angled")
        let draw_equilateral = |grid: &mut Grid, tcx: i32, tcy: i32, r: f32, rot: f32, fg: Color| {
            let mut verts = [(0i32, 0i32); 3];
            for i in 0..3usize {
                let a = rot + i as f32 * std::f32::consts::TAU / 3.0;
                verts[i] = point_on(tcx, tcy, r, r * 0.5, a);
            }
            for i in 0..3usize {
                let (ax, ay) = verts[i];
                let (bx, by) = verts[(i + 1) % 3];
                draw_line(grid, ax, ay, bx, by, fg);
            }
        };
        // layered almond eye with radial-fiber iris, gaze-shifted toward lure
        let draw_layered_eye = |grid: &mut Grid,
                                ex: i32,
                                ey: i32,
                                erx: i32,
                                ery: i32,
                                gx: i32,
                                gy: i32,
                                io: Color,
                                ii: Color,
                                lid: Color,
                                fibers: usize| {
            let iris_rx = (erx as f32 * 0.44).round() as i32;
            let iris_ry = (ery as f32 * 0.92).round() as i32;
            let pupil_rx = (iris_rx as f32 * 0.42).round() as i32;
            let pupil_ry = (iris_ry as f32 * 0.62).round() as i32;
            let icx = ex + gx;
            let icy = ey + gy;
            for dx in -erx - 1..=erx + 1 {
                let nx = dx as f32 / erx as f32;
                if nx.abs() > 1.04 {
                    continue;
                }
                let curve = (1.0 - nx.abs().powf(1.8)).max(0.0).powf(0.6);
                let top = (-ery as f32 * curve).round() as i32;
                let bot = (ery as f32 * curve).round() as i32;
                for dy in top..=bot {
                    let x = ex + dx;
                    let y = ey + dy;
                    let idx = dx - gx;
                    let idy = dy - gy;
                    let im = (idx as f32 / iris_rx as f32).powi(2)
                        + (idy as f32 / iris_ry as f32).powi(2);
                    if im <= 1.0 {
                        let pm = (idx as f32 / pupil_rx as f32).powi(2)
                            + (idy as f32 / pupil_ry as f32).powi(2);
                        if pm <= 1.0 {
                            put(grid, x, y, '●', pupil);
                        } else {
                            put(grid, x, y, '·', darken(ii, 14));
                        }
                    } else {
                        put(grid, x, y, ' ', sclera);
                    }
                }
            }
            for i in 0..fibers {
                let a = i as f32 * std::f32::consts::TAU / fibers as f32;
                let p0 = point_on(icx, icy, pupil_rx as f32 * 1.1, pupil_ry as f32 * 1.1, a);
                let p1 = point_on(icx, icy, iris_rx as f32 * 0.96, iris_ry as f32 * 0.96, a);
                let col = if i % 2 == 0 { io } else { darken(ii, 6) };
                draw_line(grid, p0.0, p0.1, p1.0, p1.1, col);
            }
            draw_arc(
                grid,
                icx,
                icy,
                pupil_rx as f32 * 1.1,
                pupil_ry as f32 * 1.1,
                0.0,
                std::f32::consts::TAU,
                pupil,
            );
            put(grid, icx, icy, '◉', pupil);
            put(grid, icx - iris_rx / 2, icy - iris_ry / 2, '˙', chalk);
            for dx in -erx - 1..=erx + 1 {
                let nx = dx as f32 / erx as f32;
                if nx.abs() > 1.04 {
                    continue;
                }
                let curve = (1.0 - nx.abs().powf(1.8)).max(0.0).powf(0.6);
                let top = (-ery as f32 * curve).round() as i32;
                let bot = (ery as f32 * curve).round() as i32;
                let cht = if dx < -erx / 2 { '╭' } else if dx > erx / 2 { '╮' } else { '─' };
                let chb = if dx < -erx / 2 { '╰' } else if dx > erx / 2 { '╯' } else { '─' };
                put(grid, ex + dx, ey + top, cht, lighten(lid, 10));
                put(grid, ex + dx, ey + bot, chb, darken(lid, 4));
            }
            put(grid, ex - erx, ey, '<', lighten(lid, 6));
            put(grid, ex + erx, ey, '>', lighten(lid, 6));
        };

        // TRIANGLE CLUSTER -- angled, numerous, overlapping (seeded rotation/count)
        let tri_count = 4 + (seed as usize % 4);
        for t in 0..tri_count {
            let r = max_rx * rng.random_range(0.55..0.95);
            let rot = phase * 0.4
                + t as f32 * std::f32::consts::TAU / 3.0
                + rng.random_range(-0.35..0.35);
            let col = match t % 4 {
                0 => darken(gold, 4),
                1 => chalk,
                2 => darken(gold, 18),
                _ => darken(ray_color, 8),
            };
            draw_equilateral(&mut grid, cx, cy, r, rot, col);
            for i in 0..3usize {
                let a = rot + i as f32 * std::f32::consts::TAU / 3.0;
                let v = point_on(cx, cy, r, r * 0.5, a);
                let g = if (v.0 + v.1 + t as i32) % 2 == 0 { '△' } else { '◆' };
                put(&mut grid, v.0, v.1, g, lighten(col, 8));
            }
        }

        // LAYERED EYE CLUSTER -- secondary eyes behind, gazing at the lure
        let extra = 2 + (seed as usize % 3);
        for i in 0..extra {
            let ang = phase
                + i as f32 * std::f32::consts::TAU / extra as f32
                + rng.random_range(-0.4..0.4);
            let dist = rng.random_range(0.30..0.62);
            let ex = (cx as f32 + ang.cos() * max_rx * dist * 0.8).round() as i32;
            let ey = (cy as f32 + ang.sin() * max_ry * dist * 0.9).round() as i32;
            if ex < 6 || ey < 3 || ex >= width as i32 - 6 || ey >= height as i32 - 3 {
                continue;
            }
            let rx = rng.random_range(7..=12);
            let ry = rng.random_range(3..=5);
            let (gx, gy) = gaze_for(ex, ey, rx, ry);
            let io = shift_hue(iris_outer, rng.random_range(-80..=80) as f64);
            let ii = shift_hue(iris_inner, rng.random_range(-60..=60) as f64);
            let lid = shift_hue(lid_color, rng.random_range(-40..=40) as f64);
            let fibers = 12 + (i + seed as usize) % 5;
            draw_layered_eye(&mut grid, ex, ey, rx, ry, gx, gy, io, ii, lid, fibers);
        }

        // HERO eye on top (biggest, fullest detail), gazing at the lure
        let hero_rx = ((width as f32 * 0.205).round() as i32).clamp(10, 22);
        let hero_ry = ((height as f32 * 0.16).round() as i32).clamp(3, 7);
        let hero_x = cx;
        let hero_y = cy - (height as i32 / 14).max(1);
        let (hgx, hgy) = gaze_for(hero_x, hero_y, hero_rx, hero_ry);
        draw_layered_eye(
            &mut grid,
            hero_x,
            hero_y,
            hero_rx,
            hero_ry,
            hgx,
            hgy,
            iris_outer,
            iris_inner,
            lid_color,
            18 + (seed as usize % 8),
        );
        // hero brow chevron
        let brow_y = hero_y - hero_ry - 2;
        let brow_half = hero_rx - 1;
        draw_line(
            &mut grid,
            hero_x - brow_half,
            brow_y + 1,
            hero_x,
            brow_y - 1,
            darken(lid_color, 6),
        );
        draw_line(
            &mut grid,
            hero_x,
            brow_y - 1,
            hero_x + brow_half,
            brow_y + 1,
            darken(lid_color, 6),
        );

        // tiny corner satellites, staring at the viewer (gaze 0,0)
        let corners = [
            (cx - max_rx as i32 + 3, cy - max_ry as i32 + 2),
            (cx + max_rx as i32 - 3, cy - max_ry as i32 + 2),
            (cx - max_rx as i32 + 4, cy + max_ry as i32 - 2),
            (cx + max_rx as i32 - 4, cy + max_ry as i32 - 2),
            (cx, cy + max_ry as i32 - 1),
        ];
        for i in 0..sat_count.min(corners.len()) {
            let (sx, sy) = corners[i];
            if sx < 3 || sy < 2 || sx >= width as i32 - 3 || sy >= height as i32 - 2 {
                continue;
            }
            let srx = rng.random_range(3..=5);
            let sry = rng.random_range(2..=3);
            let iris = shift_hue(iris_outer, rng.random_range(-60..=60) as f64);
            let lid = shift_hue(lid_color, rng.random_range(-40..=40) as f64);
            draw_small_eye(&mut grid, sx, sy, srx, sry, iris, lid);
        }

        // lure core on top of everything so the focal point always reads
        put(&mut grid, lure_x, lure_y, '◆', lighten(lure_col, 12));
    grid
}

fn draw_fullmetal_eyes(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // fullmetal-eyes [nodes] [runes] -- alchemical eye seal with watching glyph nodes
        let node_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(8);
        let node_count = node_count.clamp(5, 12);
        let rune_count: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(72);
        let rune_count = rune_count.clamp(16, 180);

        let bg = darken(palette[0], 12);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 30);
        let iris = lighten(palette[3], 30);
        let lid = lighten(palette[2], 18);
        let pupil = darken(palette[0], 2);
        let shadow = darken(palette[2], 60);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 19 + y * 43 + seed as usize * 5) % 151;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, shadow)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        gap: usize| {
            let samples = ((rx + ry) * 16.0).max(90.0) as usize;
            let mut prev: Option<(i32, i32)> = None;
            for i in 0..=samples {
                if gap > 0 && i % gap == gap - 1 {
                    prev = None;
                    continue;
                }
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if let Some(q) = prev {
                    draw_line(grid, q.0, q.1, p.0, p.1, fg);
                } else {
                    put(grid, p.0, p.1, '·', fg);
                }
                prev = Some(p);
            }
        };
        let draw_small_eye = |grid: &mut Grid,
                              cx: i32,
                              cy: i32,
                              rx: i32,
                              ry: i32,
                              lid_color: Color,
                              iris_color: Color,
                              style: usize| {
            for dx in -rx..=rx {
                let nx = dx as f32 / rx as f32;
                let curve = (1.0 - nx.abs().powf(1.65)).max(0.0).powf(0.55);
                let top = (-(ry as f32) * curve).round() as i32;
                let bottom = ((ry as f32) * 0.75 * curve).round() as i32;
                for dy in top..=bottom {
                    let x = cx + dx;
                    let y = cy + dy;
                    if dy == top || dy == bottom {
                        let ch = if dx < -rx / 2 {
                            if dy == top { '╱' } else { '╲' }
                        } else if dx > rx / 2 {
                            if dy == top { '╲' } else { '╱' }
                        } else {
                            '─'
                        };
                        put(grid, x, y, ch, lid_color);
                    } else {
                        let im = (dx as f32 / (rx as f32 * 0.32)).powi(2)
                            + (dy as f32 / ry.max(1) as f32).powi(2);
                        if im <= 0.18 {
                            put(grid, x, y, if style % 2 == 0 { '◐' } else { '◑' }, pupil);
                        } else if im <= 1.0 {
                            let ch = if im > 0.74 {
                                '○'
                            } else if (dx.abs() + dy.abs() + style as i32) % 4 == 0 {
                                '╎'
                            } else {
                                '·'
                            };
                            put(grid, x, y, ch, iris_color);
                        }
                    }
                }
            }
            put(grid, cx - 1, cy - 1, '˙', chalk);
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 4.0).max(12.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let phase = seed as f32 * 0.031 - std::f32::consts::FRAC_PI_2 + t_anim * 0.12;

        for i in 0..3 {
            let rx = max_rx * (0.92 - i as f32 * 0.16);
            let ry = max_ry * (0.92 - i as f32 * 0.16);
            draw_arc(
                &mut grid,
                cx,
                cy,
                rx,
                ry,
                phase,
                phase + std::f32::consts::TAU,
                if i == 1 { gold } else { chalk },
                if i == 2 { 7 } else { 0 },
            );
        }

        let mut nodes = Vec::new();
        for i in 0..node_count {
            let a = phase + i as f32 * std::f32::consts::TAU / node_count as f32;
            let outer = point_on(cx, cy, max_rx * 0.82, max_ry * 0.82, a);
            let inner = point_on(cx, cy, max_rx * 0.45, max_ry * 0.45, a);
            nodes.push(outer);
            draw_line(
                &mut grid,
                inner.0,
                inner.1,
                outer.0,
                outer.1,
                darken(gold, 8),
            );
        }
        for i in 0..nodes.len() {
            let j = (i + 2) % nodes.len();
            draw_line(
                &mut grid,
                nodes[i].0,
                nodes[i].1,
                nodes[j].0,
                nodes[j].1,
                darken(chalk, 18),
            );
        }

        let runes = [
            '△', '▽', '□', '◇', '☉', '☽', '☿', '♄', '♃', '✦', '∴', '∵', '⊕', '⊗',
        ];
        for i in 0..rune_count {
            let lane = match i % 4 {
                0 => 0.92,
                1 => 0.74,
                2 => 0.58,
                _ => rng.random_range(0.38..0.88),
            };
            let a = phase
                + i as f32 / rune_count as f32 * std::f32::consts::TAU
                + rng.random_range(-0.035..0.035);
            let p = point_on(cx, cy, max_rx * lane, max_ry * lane, a);
            put(
                &mut grid,
                p.0,
                p.1,
                runes[(i + rng.random_range(0..runes.len())) % runes.len()],
                shift_hue(gold, rng.random_range(-50..=65) as f64),
            );
        }

        draw_small_eye(
            &mut grid,
            cx,
            cy,
            (width as i32 / 5).clamp(12, 20),
            (height as i32 / 5).clamp(4, 7),
            lighten(lid, 10),
            lighten(iris, 16),
            seed as usize,
        );
        for (i, &(nx, ny)) in nodes.iter().enumerate() {
            draw_small_eye(
                &mut grid,
                nx,
                ny,
                5 + (i as i32 % 2),
                2,
                darken(lid, 8),
                shift_hue(iris, i as f64 * 38.0),
                i,
            );
            put(
                &mut grid,
                nx,
                ny + 3,
                runes[i % runes.len()],
                lighten(gold, 12),
            );
        }

        for _ in 0..node_count {
            let a = phase + rng.random::<f32>() * std::f32::consts::TAU;
            let p1 = point_on(
                cx,
                cy,
                max_rx * rng.random_range(0.22..0.45),
                max_ry * rng.random_range(0.22..0.45),
                a,
            );
            let p2 = point_on(
                cx,
                cy,
                max_rx * rng.random_range(0.60..0.90),
                max_ry * rng.random_range(0.60..0.90),
                a + rng.random_range(0.3..1.3),
            );
            draw_line(&mut grid, p1.0, p1.1, p2.0, p2.1, darken(iris, 18));
        }
    grid
}

fn draw_fullmetal_eyes2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // fullmetal-eyes2 [tiers=0] [runes=0] -- multi-tier watching seal; every eye tracks a seeded lure
        let tier_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let tier_count = if tier_arg == 0 {
            2 + (seed as usize % 2) // 2..3
        } else {
            tier_arg.clamp(1, 4)
        };
        let rune_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let rune_count = if rune_arg == 0 {
            60 + (seed as usize % 60) // 60..119
        } else {
            rune_arg.clamp(16, 200)
        };

        let bg = darken(palette[0], 12);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 30);
        let iris = lighten(palette[3], 30);
        let lid = lighten(palette[2], 18);
        let pupil = darken(palette[0], 2);
        let sclera = lighten(palette[4], 4);
        let shadow = darken(palette[2], 60);

        // bg haze
        for y in 0..height {
            for x in 0..width {
                let n = (x * 19 + y * 43 + seed as usize * 5) % 151;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, shadow)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        gap: usize| {
            let samples = ((rx + ry) * 16.0).max(90.0) as usize;
            let mut prev: Option<(i32, i32)> = None;
            for i in 0..=samples {
                if gap > 0 && i % gap == gap - 1 {
                    prev = None;
                    continue;
                }
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if let Some(q) = prev {
                    draw_line(grid, q.0, q.1, p.0, p.1, fg);
                } else {
                    put(grid, p.0, p.1, '·', fg);
                }
                prev = Some(p);
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 4.0).max(12.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let phase = seed as f32 * 0.031 - std::f32::consts::FRAC_PI_2 + t_anim * 0.12;

        // seeded focal lure: the lever that makes every eye gaze a different way per seed
        let lure_x = cx + (rng.random_range(-0.35..0.35) * max_rx * 0.9) as i32;
        let lure_y = cy + (rng.random_range(-0.30..0.30) * max_ry * 0.9) as i32;
        draw_arc(
            &mut grid,
            lure_x,
            lure_y,
            3.0,
            1.5,
            0.0,
            std::f32::consts::TAU,
            shift_hue(gold, 40.0),
            4,
        );
        put(&mut grid, lure_x, lure_y, '◆', lighten(gold, 20));

        // gaze helper: unit vector toward lure, scaled to eye radius
        let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| -> (i32, i32) {
            let dx = (lure_x - ex) as f32;
            let dy = (lure_y - ey) as f32;
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let mx = rx as f32 * 0.34;
            let my = ry as f32 * 0.6;
            let gx = ((dx / d) * mx).round() as i32;
            let gy = ((dy / d) * my).round() as i32;
            (
                gx.clamp(-(rx / 3).max(1), (rx / 3).max(1)),
                gy.clamp(-(ry / 2).max(1), (ry / 2).max(1)),
            )
        };

        // hero layered eye: almond sclera + lid rims + gaze-shifted radial-fiber iris + ◉ pupil
        let draw_hero_eye = |grid: &mut Grid,
                             ex: i32,
                             ey: i32,
                             rx: i32,
                             ry: i32,
                             gx: i32,
                             gy: i32,
                             iris_c: Color,
                             lid_c: Color| {
            for dx in -rx..=rx {
                let nx = dx as f32 / rx as f32;
                let curve = (1.0 - nx.abs().powf(1.6)).max(0.0).powf(0.55);
                let top = (-(ry as f32) * curve).round() as i32;
                let bottom = ((ry as f32) * 0.78 * curve).round() as i32;
                for dy in top..=bottom {
                    let x = ex + dx;
                    let y = ey + dy;
                    if dy == top || dy == bottom {
                        let ch = if dx < -rx / 2 {
                            if dy == top { '╱' } else { '╲' }
                        } else if dx > rx / 2 {
                            if dy == top { '╲' } else { '╱' }
                        } else {
                            '─'
                        };
                        put(grid, x, y, ch, lid_c);
                    } else {
                        let ix = (dx - gx) as f32 / (rx as f32 * 0.42);
                        let iy = (dy - gy) as f32 / ry.max(1) as f32;
                        let im = ix * ix + iy * iy;
                        if im <= 0.16 {
                            put(grid, x, y, '◉', pupil);
                        } else if im <= 1.0 {
                            let ang =
                                (iy.atan2(ix) + std::f32::consts::TAU) % std::f32::consts::TAU;
                            let fiber = ((ang / (std::f32::consts::TAU / 16.0)).round() as i32) % 2
                                == 0;
                            let ch = if im > 0.78 {
                                '○'
                            } else if fiber {
                                '╎'
                            } else {
                                '·'
                            };
                            put(grid, x, y, ch, iris_c);
                        } else {
                            put(grid, x, y, '·', sclera);
                        }
                    }
                }
            }
            put(grid, ex + gx - 1, ey + gy - 1, '˙', chalk);
            put(grid, ex + gx, ey + gy - 1, '˙', chalk);
        };

        // small gazing node eye
        let draw_node_eye = |grid: &mut Grid,
                             ncx: i32,
                             ncy: i32,
                             rx: i32,
                             ry: i32,
                             gx: i32,
                             gy: i32,
                             lid_c: Color,
                             iris_c: Color| {
            for dx in -rx..=rx {
                let nx = dx as f32 / rx as f32;
                let curve = (1.0 - nx.abs().powf(1.65)).max(0.0).powf(0.55);
                let top = (-(ry as f32) * curve).round() as i32;
                let bottom = ((ry as f32) * 0.75 * curve).round() as i32;
                for dy in top..=bottom {
                    let x = ncx + dx;
                    let y = ncy + dy;
                    if dy == top || dy == bottom {
                        let ch = if dx < -rx / 2 {
                            if dy == top { '╱' } else { '╲' }
                        } else if dx > rx / 2 {
                            if dy == top { '╲' } else { '╱' }
                        } else {
                            '─'
                        };
                        put(grid, x, y, ch, lid_c);
                    } else {
                        let ix = (dx - gx) as f32 / (rx as f32 * 0.5);
                        let iy = (dy - gy) as f32 / ry.max(1) as f32;
                        let im = ix * ix + iy * iy;
                        if im <= 0.5 {
                            put(grid, x, y, '◉', pupil);
                        } else if im <= 1.0 {
                            put(grid, x, y, '·', iris_c);
                        }
                    }
                }
            }
            put(grid, ncx + gx - 1, ncy + gy - 1, '˙', chalk);
        };

        let runes = [
            '△', '▽', '□', '◇', '☉', '☽', '☿', '♄', '♃', '✦', '∴', '∵', '⊕', '⊗', '✶', '◈',
        ];

        // arc bands: 3 concentric, alternating chalk/gold/iris, iris dashed
        let band_count = 3;
        for i in 0..band_count {
            let t = i as f32 / (band_count - 1).max(1) as f32;
            let r = 0.50 + t * 0.40;
            let col = match i % 3 {
                0 => chalk,
                1 => gold,
                _ => iris,
            };
            let gap = if i % 3 == 2 { 9 } else { 0 };
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * r,
                max_ry * r,
                phase,
                phase + std::f32::consts::TAU,
                col,
                gap,
            );
        }

        // multi-tier node web: each tier is a {n/k} star polygon, seeded n/k/rotation
        let mut tier_verts: Vec<Vec<(i32, i32)>> = Vec::new();
        for ti in 0..tier_count {
            let rad = 0.42 + ti as f32 * 0.16;
            let n = 5 + (seed as usize + ti * 7) % 6;
            let rot = phase + ti as f32 * 0.4 + seed as f32 * 0.01 * ti as f32;
            let mut verts: Vec<(i32, i32)> = Vec::with_capacity(n);
            for i in 0..n {
                let a = rot + i as f32 * std::f32::consts::TAU / n as f32;
                verts.push(point_on(cx, cy, max_rx * rad, max_ry * rad, a));
            }
            let max_k = ((n - 1) / 2).max(1);
            let k = if max_k >= 2 {
                2 + (seed as usize + ti) % (max_k - 1)
            } else {
                1
            };
            let col = if ti % 2 == 0 { gold } else { chalk };
            for i in 0..n {
                let (ax, ay) = verts[i];
                let (bx, by) = verts[(i + k.min(max_k)) % n];
                draw_line(&mut grid, ax, ay, bx, by, darken(col, 8));
            }
            for &v in &verts {
                put(&mut grid, v.0, v.1, '◇', lighten(col, 8));
            }
            tier_verts.push(verts);
        }

        // curved rune inscription band running around the outer ring
        let inscribe_r = 0.92;
        let ins_n = (rune_count / 4).clamp(14, 30);
        for i in 0..ins_n {
            let a = phase + i as f32 * std::f32::consts::TAU / ins_n as f32;
            let p = point_on(cx, cy, max_rx * inscribe_r, max_ry * inscribe_r, a);
            put(
                &mut grid,
                p.0,
                p.1,
                runes[(i + seed as usize) % runes.len()],
                lighten(gold, 8),
            );
        }

        // hero eye at center, gazing at lure
        let hero_rx = (width as f32 * 0.12) as i32;
        let hero_ry = (height as f32 * 0.14) as i32;
        let (hgx, hgy) = gaze_for(cx, cy, hero_rx, hero_ry);
        draw_hero_eye(
            &mut grid,
            cx,
            cy,
            hero_rx,
            hero_ry,
            hgx,
            hgy,
            lighten(iris, 16),
            lighten(lid, 10),
        );

        // node eyes at outer tier, each gazing at the lure
        if let Some(outer) = tier_verts.last() {
            for (i, &(nx, ny)) in outer.iter().enumerate() {
                let (ngx, ngy) = gaze_for(nx, ny, 5, 2);
                draw_node_eye(
                    &mut grid,
                    nx,
                    ny,
                    5,
                    2,
                    ngx,
                    ngy,
                    darken(lid, 6),
                    shift_hue(iris, i as f64 * 40.0),
                );
                put(
                    &mut grid,
                    nx,
                    ny + 3,
                    runes[i % runes.len()],
                    lighten(gold, 12),
                );
            }
        }

        // a few seeded crossing chords for energy
        for _ in 0..(tier_count + 1) {
            let a = phase + rng.random::<f32>() * std::f32::consts::TAU;
            let p1 = point_on(
                cx,
                cy,
                max_rx * rng.random_range(0.22..0.45),
                max_ry * rng.random_range(0.22..0.45),
                a,
            );
            let p2 = point_on(
                cx,
                cy,
                max_rx * rng.random_range(0.60..0.90),
                max_ry * rng.random_range(0.60..0.90),
                a + rng.random_range(0.3..1.3),
            );
            draw_line(&mut grid, p1.0, p1.1, p2.0, p2.1, darken(iris, 18));
        }
    grid
}

fn draw_spiro(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // spiro [curves=0] [density=0] -- layered hypotrochoid / harmonograph curves
        let curve_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let curve_count = if curve_arg == 0 {
            2 + (seed as usize % 3)
        } else {
            curve_arg.clamp(1, 6)
        };
        let density_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let density = if density_arg == 0 {
            1400 + (seed as usize % 800)
        } else {
            density_arg.clamp(400, 6000)
        };

        let bg = darken(palette[0], 14);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 30);
        let cyan = shift_hue(lighten(palette[3], 34), 35.0);
        let magenta = shift_hue(lighten(palette[2], 40), -42.0);
        let lime = shift_hue(lighten(palette[1], 30), 90.0);
        let curve_colors = [chalk, gold, cyan, magenta, lime, chalk];

        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(' ', bg);
            }
        }
        for _ in 0..(width * height / 90) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);
            grid[y][x] = Cell::new('·', darken(chalk, 60));
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let curve_char = |prev: (i32, i32), here: (i32, i32), next: (i32, i32)| {
            let dx1 = (here.0 - prev.0).signum();
            let dy1 = (here.1 - prev.1).signum();
            let dx2 = (next.0 - here.0).signum();
            let dy2 = (next.1 - here.1).signum();
            if (dx1, dy1) == (dx2, dy2) {
                if dy1 == 0 {
                    '─'
                } else if dx1 == 0 {
                    '│'
                } else if dx1 == dy1 {
                    '╲'
                } else {
                    '╱'
                }
            } else if dy1 == 0 && dx2 == 0 {
                match (dx1, dy2) {
                    (1, 1) => '╮',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╰',
                    _ => '╮',
                }
            } else if dx1 == 0 && dy2 == 0 {
                match (dy1, dx2) {
                    (1, 1) => '╰',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╮',
                    _ => '╰',
                }
            } else if dx1 != dx2 && dy1 != dy2 {
                match (dx1, dy1, dx2, dy2) {
                    (1, 1, 1, -1) | (-1, -1, -1, 1) => '╯',
                    (1, -1, 1, 1) | (-1, 1, -1, -1) => '╮',
                    (1, 1, -1, 1) | (-1, -1, 1, -1) => '╰',
                    (1, -1, -1, -1) | (-1, 1, 1, 1) => '╭',
                    _ => '○',
                }
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else if dx2 == dy2 {
                '╲'
            } else {
                '╱'
            }
        };

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let scale = ((width as f32 / 2.0) - 2.0).min(height as f32 - 2.0).max(8.0);

        for ci in 0..curve_count {
            let r_big = scale * rng.random_range(0.55..0.95);
            let k = rng.random_range(2..9) as f32;
            let r_small = r_big / k;
            let d = r_small * rng.random_range(0.5..1.8);
            let rot = rng.random_range(0.0..std::f32::consts::TAU) + t_anim * (0.1 + ci as f32 * 0.03);
            let turns = (k as i32 + 1) * 2;
            let samples = density;
            let color = curve_colors[ci % curve_colors.len()];
            let accent = lighten(color, 14);
            let cosr = rot.cos();
            let sinr = rot.sin();

            let mut pts: Vec<(i32, i32)> = Vec::with_capacity(samples);
            for i in 0..=samples {
                let t = i as f32 / samples as f32 * std::f32::consts::TAU * turns as f32;
                let xg = (r_big - r_small) * t.cos()
                    + d * ((r_big - r_small) / r_small * t).cos();
                let yg = (r_big - r_small) * t.sin()
                    - d * ((r_big - r_small) / r_small * t).sin();
                let xr = xg * cosr - yg * sinr;
                let yr = xg * sinr + yg * cosr;
                let px = (cx + xr).round() as i32;
                let py = (cy + yr * 0.5).round() as i32;
                if pts.last().copied() != Some((px, py)) {
                    pts.push((px, py));
                }
            }
            for i in 1..pts.len().saturating_sub(1) {
                let ch = curve_char(pts[i - 1], pts[i], pts[i + 1]);
                let col = if (i + ci * 5) % 11 == 0 {
                    accent
                } else {
                    color
                };
                put(&mut grid, pts[i].0, pts[i].1, ch, col);
            }
        }
        put(&mut grid, cx.round() as i32, cy.round() as i32, '⊙', lighten(chalk, 10));
    grid
}

fn draw_spiro_tile(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // spiro-tile [cols=0] [rows=0] [vary=0] -- tessellated grid of small spiro motifs
        let col_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let cols = if col_arg == 0 {
            4 + (seed as usize % 3)
        } else {
            col_arg.clamp(2, 10)
        };
        let row_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let rows = if row_arg == 0 {
            2 + (seed as usize % 3)
        } else {
            row_arg.clamp(2, 8)
        };
        let vary_arg: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let vary = vary_arg != 0 || (seed as usize % 4 == 0);

        let bg = darken(palette[0], 13);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 28);
        let cyan = shift_hue(lighten(palette[3], 32), 35.0);
        let rose = shift_hue(lighten(palette[2], 38), -40.0);
        let tile_colors = [chalk, gold, cyan, rose, chalk];
        let border_color = darken(chalk, 50);

        let base_k = (3 + seed as usize % 5) as f32;
        let base_dp = 0.72 + (seed as f32 * 0.1).fract() * 0.6;
        let turns_base = (base_k as i32 + 1) * 2;

        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(' ', bg);
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let curve_char = |prev: (i32, i32), here: (i32, i32), next: (i32, i32)| {
            let dx1 = (here.0 - prev.0).signum();
            let dy1 = (here.1 - prev.1).signum();
            let dx2 = (next.0 - here.0).signum();
            let dy2 = (next.1 - here.1).signum();
            if (dx1, dy1) == (dx2, dy2) {
                if dy1 == 0 {
                    '─'
                } else if dx1 == 0 {
                    '│'
                } else if dx1 == dy1 {
                    '╲'
                } else {
                    '╱'
                }
            } else if dy1 == 0 && dx2 == 0 {
                match (dx1, dy2) {
                    (1, 1) => '╮',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╰',
                    _ => '╮',
                }
            } else if dx1 == 0 && dy2 == 0 {
                match (dy1, dx2) {
                    (1, 1) => '╰',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╮',
                    _ => '╰',
                }
            } else if dx1 != dx2 && dy1 != dy2 {
                match (dx1, dy1, dx2, dy2) {
                    (1, 1, 1, -1) | (-1, -1, -1, 1) => '╯',
                    (1, -1, 1, 1) | (-1, 1, -1, -1) => '╮',
                    (1, 1, -1, 1) | (-1, -1, 1, -1) => '╰',
                    (1, -1, -1, -1) | (-1, 1, 1, 1) => '╭',
                    _ => '○',
                }
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else if dx2 == dy2 {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_box = |grid: &mut Grid, x0: i32, y0: i32, x1: i32, y1: i32, fg: Color| {
            for x in x0 + 1..x1 {
                put(grid, x, y0, '─', fg);
                put(grid, x, y1, '─', fg);
            }
            for y in y0 + 1..y1 {
                put(grid, x0, y, '│', fg);
                put(grid, x1, y, '│', fg);
            }
            put(grid, x0, y0, '╭', fg);
            put(grid, x1, y0, '╮', fg);
            put(grid, x0, y1, '╰', fg);
            put(grid, x1, y1, '╯', fg);
        };

        let tile_w = width / cols;
        let tile_h = height / rows;

        for ry in 0..rows {
            for rx in 0..cols {
                let x0 = (rx * tile_w) as i32;
                let y0 = (ry * tile_h) as i32;
                let x1 = (((rx + 1) * tile_w).min(width - 1)) as i32;
                let y1 = (((ry + 1) * tile_h).min(height - 1)) as i32;
                draw_box(&mut grid, x0, y0, x1, y1, border_color);

                let tx = ((rx * tile_w + tile_w / 2) as f32).min(width as f32 - 1.5);
                let ty = ((ry * tile_h + tile_h / 2) as f32).min(height as f32 - 1.0);
                let hw = (tile_w as f32 / 2.0 - 1.5).max(2.0);
                let hh = (tile_h as f32 / 2.0 - 1.0).max(1.5);
                let scale = hw.min(hh * 2.0).max(2.0);

                let (k, dp, rot, flip) = if vary {
                    let k = (base_k + ((rx as i32 - ry as i32) as f32 * 0.16)).max(2.0);
                    let dp = base_dp + (rx as f32 * 0.05).sin() * 0.30 + (ry as f32 * 0.07).cos() * 0.20;
                    let rot = (rx as f32 + ry as f32 * 1.3) * 0.42 + t_anim * 0.12;
                    let flip = (rx + ry) % 2 == 0;
                    (k, dp, rot, flip)
                } else {
                    (base_k, base_dp, t_anim * 0.12, (rx + ry) % 2 == 0)
                };

                let color = tile_colors[(rx + ry * 2) % tile_colors.len()];
                let accent = shift_hue(color, 55.0);

                for ci in 0..2usize {
                    let r_big = scale * (0.88 - ci as f32 * 0.18);
                    let r_small = r_big / k;
                    let d = r_small * dp * if ci == 0 { 1.0 } else { 1.45 };
                    let cosr = rot.cos();
                    let sinr = rot.sin();
                    let turns = turns_base + ci as i32;
                    let samples = 420;
                    let mut pts: Vec<(i32, i32)> = Vec::new();
                    for i in 0..=samples {
                        let t = i as f32 / samples as f32 * std::f32::consts::TAU * turns as f32;
                        let xg = (r_big - r_small) * t.cos()
                            + d * ((r_big - r_small) / r_small * t).cos();
                        let yg = (r_big - r_small) * t.sin()
                            - d * ((r_big - r_small) / r_small * t).sin();
                        let (xg, yg) = if flip { (xg, -yg) } else { (xg, yg) };
                        let xr = xg * cosr - yg * sinr;
                        let yr = xg * sinr + yg * cosr;
                        let px = (tx + xr).round() as i32;
                        let py = (ty + yr * 0.5).round() as i32;
                        if pts.last().copied() != Some((px, py)) {
                            pts.push((px, py));
                        }
                    }
                    let col = if ci == 0 { color } else { darken(accent, 4) };
                    for i in 1..pts.len().saturating_sub(1) {
                        let ch = curve_char(pts[i - 1], pts[i], pts[i + 1]);
                        put(&mut grid, pts[i].0, pts[i].1, ch, col);
                    }
                }
                put(
                    &mut grid,
                    tx.round() as i32,
                    ty.round() as i32,
                    '·',
                    darken(color, 26),
                );
            }
        }
    grid
}

fn draw_weave(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // weave [horiz=0] [vert=0] -- interlaced wavy warp/weft strands
        let h_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let h_count = if h_arg == 0 {
            3 + (seed as usize % 3)
        } else {
            h_arg.clamp(2, 8)
        };
        let v_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let v_count = if v_arg == 0 {
            3 + (seed as usize % 4)
        } else {
            v_arg.clamp(2, 8)
        };

        let bg = darken(palette[0], 10);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 28);
        let cyan = shift_hue(lighten(palette[3], 32), 35.0);
        let rose = shift_hue(lighten(palette[2], 38), -40.0);
        let lime = shift_hue(lighten(palette[1], 26), 90.0);
        let h_colors = [chalk, gold, cyan, rose, lime, chalk, gold, cyan];
        let v_colors = [gold, rose, lime, chalk, cyan, gold, rose, lime];

        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(' ', bg);
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let curve_char = |prev: (i32, i32), here: (i32, i32), next: (i32, i32)| {
            let dx1 = (here.0 - prev.0).signum();
            let dy1 = (here.1 - prev.1).signum();
            let dx2 = (next.0 - here.0).signum();
            let dy2 = (next.1 - here.1).signum();
            if (dx1, dy1) == (dx2, dy2) {
                if dy1 == 0 {
                    '─'
                } else if dx1 == 0 {
                    '│'
                } else if dx1 == dy1 {
                    '╲'
                } else {
                    '╱'
                }
            } else if dy1 == 0 && dx2 == 0 {
                match (dx1, dy2) {
                    (1, 1) => '╮',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╰',
                    _ => '╮',
                }
            } else if dx1 == 0 && dy2 == 0 {
                match (dy1, dx2) {
                    (1, 1) => '╰',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╮',
                    _ => '╰',
                }
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else {
                '┼'
            }
        };

        let mut h_spec: Vec<(f32, f32, f32, f32, Color)> = Vec::new();
        let mut v_spec: Vec<(f32, f32, f32, f32, Color)> = Vec::new();
        for i in 0..h_count {
            let base = (i as f32 + 0.5) * (height as f32 / h_count as f32);
            let amp = (height as f32 / (h_count as f32 + 1.0)) * rng.random_range(0.35..0.7);
            let freq = rng.random_range(0.06..0.16);
            let phase = rng.random_range(0.0..std::f32::consts::TAU) + t_anim * 0.6; // warp strands drift
            h_spec.push((base, amp, freq, phase, h_colors[i % h_colors.len()]));
        }
        for i in 0..v_count {
            let base = (i as f32 + 0.5) * (width as f32 / v_count as f32);
            let amp = (width as f32 / (v_count as f32 + 1.0)) * rng.random_range(0.30..0.62);
            let freq = rng.random_range(0.06..0.16);
            let phase = rng.random_range(0.0..std::f32::consts::TAU) - t_anim * 0.6; // weft strands counter-drift
            v_spec.push((base, amp, freq, phase, v_colors[i % v_colors.len()]));
        }

        let y_h = |i: usize, x: i32| -> i32 {
            let (base, amp, freq, phase, _) = h_spec[i];
            (base + amp * (freq * x as f32 + phase).sin()).round() as i32
        };
        let x_v = |i: usize, y: i32| -> i32 {
            let (base, amp, freq, phase, _) = v_spec[i];
            (base + amp * (freq * y as f32 + phase).sin()).round() as i32
        };

        // occupancy: which strand ids pass through a cell
        let mut occ_h: Vec<Vec<Vec<u8>>> = vec![vec![Vec::new(); width]; height];
        let mut occ_v: Vec<Vec<Vec<u8>>> = vec![vec![Vec::new(); width]; height];
        for i in 0..h_count {
            for x in 0..width {
                let y = y_h(i, x as i32);
                if y >= 0 && (y as usize) < height {
                    occ_h[y as usize][x].push(i as u8);
                }
            }
        }
        for i in 0..v_count {
            for y in 0..height {
                let x = x_v(i, y as i32);
                if x >= 0 && (x as usize) < width {
                    occ_v[y][x as usize].push(i as u8);
                }
            }
        }

        // draw horizontal strands
        for i in 0..h_count {
            let color = h_spec[i].4;
            let mut prev = (0, y_h(i, 0));
            let mut here = prev;
            for x in 0..width {
                let next = ((x + 1) as i32, y_h(i, (x + 1) as i32));
                if x > 0 {
                    prev = ((x - 1) as i32, y_h(i, (x - 1) as i32));
                }
                here = (x as i32, y_h(i, x as i32));
                let y = here.1;
                if y < 0 || (y as usize) >= height {
                    continue;
                }
                // over/under: skip if a vertical dominates here
                let vs = &occ_v[y as usize][x];
                let mut under = false;
                if !vs.is_empty() {
                    let v0 = vs[0] as usize;
                    if (i + v0) % 2 != 0 {
                        under = true;
                    }
                }
                if !under {
                    let ch = if x == 0 || x == width - 1 {
                        '─'
                    } else {
                        curve_char(prev, here, next)
                    };
                    put(&mut grid, here.0, here.1, ch, color);
                }
            }
        }
        // draw vertical strands
        for i in 0..v_count {
            let color = v_spec[i].4;
            let mut prev = (x_v(i, 0), 0);
            let mut here = prev;
            for y in 0..height {
                let next = (x_v(i, (y + 1) as i32), (y + 1) as i32);
                if y > 0 {
                    prev = (x_v(i, (y - 1) as i32), (y - 1) as i32);
                }
                here = (x_v(i, y as i32), y as i32);
                let x = here.0;
                if x < 0 || (x as usize) >= width {
                    continue;
                }
                let hs = &occ_h[y][x as usize];
                let mut under = false;
                if !hs.is_empty() {
                    let h0 = hs[0] as usize;
                    if (h0 + i) % 2 == 0 {
                        under = true;
                    }
                }
                if !under {
                    let ch = if y == 0 || y == height - 1 {
                        '│'
                    } else {
                        curve_char(prev, here, next)
                    };
                    put(&mut grid, here.0, here.1, ch, color);
                }
            }
        }
    grid
}

fn draw_gears(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // gears [count=0] [teeth=0] -- interlocking clockwork mechanism
        let count_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let gear_count = if count_arg == 0 {
            2 + (seed as usize % 3)
        } else {
            count_arg.clamp(2, 4)
        };
        let teeth_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let base_teeth = if teeth_arg == 0 {
            8 + (seed as usize % 9)
        } else {
            teeth_arg.clamp(6, 24)
        };

        let bg = darken(palette[0], 12);
        let chalk = lighten(palette[4], 14);
        let brass = lighten(palette[1], 30);
        let steel = lighten(palette[3], 28);
        let copper = shift_hue(lighten(palette[2], 36), -22.0);
        let patina = shift_hue(lighten(palette[1], 24), 92.0);
        let gear_colors = [chalk, brass, steel, copper, patina, brass];
        let hush = darken(palette[2], 66);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 17 + y * 23 + seed as usize * 7) % 149;
                let ch = if n == 0 { '·' } else { ' ' };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, hush)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid,
                         mut x0: i32,
                         mut y0: i32,
                         x1: i32,
                         y1: i32,
                         fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let curve_char = |prev: (i32, i32), here: (i32, i32), next: (i32, i32)| {
            let dx1 = (here.0 - prev.0).signum();
            let dy1 = (here.1 - prev.1).signum();
            let dx2 = (next.0 - here.0).signum();
            let dy2 = (next.1 - here.1).signum();
            if (dx1, dy1) == (dx2, dy2) {
                if dy1 == 0 {
                    '─'
                } else if dx1 == 0 {
                    '│'
                } else if dx1 == dy1 {
                    '╲'
                } else {
                    '╱'
                }
            } else if dy1 == 0 && dx2 == 0 {
                match (dx1, dy2) {
                    (1, 1) => '╮',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╰',
                    _ => '╮',
                }
            } else if dx1 == 0 && dy2 == 0 {
                match (dy1, dx2) {
                    (1, 1) => '╰',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╮',
                    _ => '╰',
                }
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else if dx2 == dy2 {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color| {
            let span = (end - start).abs().max(0.05);
            let samples = ((rx + ry) * span * 3.8).max(18.0) as usize;
            let mut pts: Vec<(i32, i32)> = Vec::new();
            for i in 0..=samples {
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if pts.last().copied() != Some(p) {
                    pts.push(p);
                }
            }
            if pts.len() > 2 {
                for p in 1..pts.len() - 1 {
                    let ch = curve_char(pts[p - 1], pts[p], pts[p + 1]);
                    put(grid, pts[p].0, pts[p].1, ch, fg);
                }
            }
        };
        let draw_gear = |grid: &mut Grid,
                         cx: i32,
                         cy: i32,
                         r: i32,
                         teeth: usize,
                         phase: f32,
                         fg: Color,
                         accent: Color| {
            let rf = r as f32;
            let rx = rf;
            let ry = rf * 0.5;
            // teeth: radial bars from rim to tip
            for i in 0..teeth {
                let a = phase + i as f32 * std::f32::consts::TAU / teeth as f32;
                let inner = point_on(cx, cy, rx * 0.92, ry * 0.92, a);
                let outer = point_on(cx, cy, rx + 1.4, ry + 0.7, a);
                draw_line(grid, inner.0, inner.1, outer.0, outer.1, accent);
                let tip = point_on(cx, cy, rx + 1.4, ry + 0.7, a);
                put(grid, tip.0, tip.1, '◆', lighten(accent, 10));
            }
            // rim
            draw_arc(grid, cx, cy, rx, ry, 0.0, std::f32::consts::TAU, fg);
            draw_arc(grid, cx, cy, rx * 0.82, ry * 0.82, 0.0, std::f32::consts::TAU, darken(fg, 8));
            // spokes
            let spokes = (teeth / 2).clamp(3, 6);
            for s in 0..spokes {
                let a = phase + s as f32 * std::f32::consts::TAU / spokes as f32;
                let p_in = point_on(cx, cy, rx * 0.30, ry * 0.30, a);
                let p_out = point_on(cx, cy, rx * 0.80, ry * 0.80, a);
                draw_line(grid, p_in.0, p_in.1, p_out.0, p_out.1, darken(fg, 6));
                put(grid, p_out.0, p_out.1, '○', darken(accent, 6));
            }
            // hub
            draw_arc(grid, cx, cy, rx * 0.30, ry * 0.30, 0.0, std::f32::consts::TAU, accent);
            put(grid, cx, cy, '⊙', lighten(accent, 14));
        };

        // layout: place gears, each tangent to an existing one
        let cx0 = (width as f32 / 2.0).round() as i32;
        let cy0 = (height as f32 / 2.0).round() as i32;
        let r0 = (width.min(height * 2) as f32 / 7.0).round() as i32;
        let mut placed: Vec<(i32, i32, i32, usize, f32, f32, Color)> = Vec::new();
        placed.push((
            cx0,
            cy0,
            r0,
            base_teeth,
            rng.random_range(0.0..std::f32::consts::TAU),
            1.0,
            gear_colors[0],
        ));
        let mut attempts = 0;
        while placed.len() < gear_count && attempts < 60 {
            attempts += 1;
            let anchor = placed[rng.random_range(0..placed.len())];
            let (ax, ay, ar, _, _, _, _) = anchor;
            let new_r = ((ar as f32) * rng.random_range(0.6..1.1))
                .clamp(4.0, (width.min(height * 2) as f32 / 5.0));
            let new_ri = new_r.round() as i32;
            let ang = rng.random_range(0.0..std::f32::consts::TAU);
            let dist = (ar + new_ri) as f32;
            let nx = ax + (dist * ang.cos()).round() as i32;
            let ny = ay + (dist * ang.sin() * 0.5).round() as i32;
            let margin = new_ri + 2;
            if nx < margin
                || ny < margin / 2
                || nx >= width as i32 - margin
                || ny >= height as i32 - margin / 2
            {
                continue;
            }
            // reject overlap
            let mut overlap = false;
            for (ox, oy, or_, _, _, _, _) in &placed {
                let dx = nx - ox;
                let dy = (ny - oy) * 2;
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d < (new_ri + or_) as f32 * 0.92 {
                    overlap = true;
                    break;
                }
            }
            if overlap {
                continue;
            }
            let teeth = (base_teeth as f32 * new_r / r0 as f32)
                .round()
                .max(6.0) as usize;
            // mesh tooth phase: half-tooth offset relative to anchor
            let anchor_teeth = anchor.3;
            let anchor_phase = anchor.4;
            let anchor_speed = anchor.5;
            let mesh_angle = ang + std::f32::consts::PI;
            let off = std::f32::consts::TAU / anchor_teeth as f32 / 2.0;
            let phase = mesh_angle
                + off
                - (std::f32::consts::TAU / teeth as f32)
                * ((mesh_angle - anchor_phase) / (std::f32::consts::TAU / teeth as f32)).round();
            let speed = -anchor_speed * anchor_teeth as f32 / teeth as f32;
            let color = gear_colors[placed.len() % gear_colors.len()];
            placed.push((nx, ny, new_ri, teeth, phase, speed, color));
        }

        for &(_, _, _, _, _, _, color) in &placed {
            // shadow lines: none; gears drawn next
        }
        for &(gx, gy, gr, gteeth, gphase, gspeed, color) in &placed {
            draw_gear(
                &mut grid,
                gx,
                gy,
                gr,
                gteeth,
                gphase + gspeed * t_anim * 0.15,
                darken(color, 6),
                lighten(color, 12),
            );
        }
    grid
}

fn draw_delta(grid: &mut Grid, width: usize, height: usize, _seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32) {
    use std::f32::consts::FRAC_PI_2;
    let bg = darken(palette[0], 6);
    for y in 0..height {
        for x in 0..width {
            grid[y][x] = Cell::new(' ', bg);
        }
    }
    // Physics tree. Each branch is a torsional spring-damper at its joint: it has
    // a rest angle relative to its parent, an angular deflection `theta`, and an
    // angular velocity `omega`. A turbulent wind force field pushes on each
    // segment; the spring pulls it back to rest; damping bleeds energy. Children
    // hang off the parent's *current* tip and inherit its swayed world angle, so
    // it's a kinematic chain -- trunk motion propagates to twigs, and lighter
    // (shorter) tips have less inertia so they flutter faster than the trunk.
    //
    // Frames are stateless (each render gets a single time `t`), so we re-integrate
    // from a bounded warm-up window each frame: start at rest at t0 = t - WARM and
    // step to t. The artificial rest start's transient decays within WARM, leaving
    // the true forced steady-state at t -- constant cost no matter how large t grows.
    struct Node {
        parent: i32,
        rel: f32,  // rest angle relative to parent (absolute for roots)
        len: f32,
        depth: i32,
        bx: f32,   // root base position (children derive base from parent tip)
        by: f32,
    }
    // Build topology with the exact same rng call order as the static tree, so a
    // t==0 render is byte-identical to the snapshot / plain mode.
    let mut nodes: Vec<Node> = Vec::new();
    struct Pending {
        parent: i32,
        rel: f32,
        len: f32,
        depth: i32,
        bx: f32,
        by: f32,
    }
    let roots = 3;
    let mut stack: Vec<Pending> = Vec::new();
    for r in 0..roots {
        let x = width as f32 * (r as f32 + 1.0) / (roots as f32 + 1.0);
        stack.push(Pending {
            parent: -1,
            rel: FRAC_PI_2 + rng.random_range(-0.25f32..0.25),
            len: height as f32 * 0.30,
            depth: 0,
            bx: x,
            by: 1.0,
        });
    }
    while let Some(p) = stack.pop() {
        if p.depth > 7 || p.len < 2.0 {
            continue;
        }
        let idx = nodes.len() as i32;
        nodes.push(Node {
            parent: p.parent,
            rel: p.rel,
            len: p.len,
            depth: p.depth,
            bx: p.bx,
            by: p.by,
        });
        let children = if p.depth < 2 { 3 } else { 2 };
        for _ in 0..children {
            let da = rng.random_range(-0.65f32..0.65);
            let len = p.len * rng.random_range(0.6f32..0.78);
            stack.push(Pending {
                parent: idx,
                rel: da,
                len,
                depth: p.depth + 1,
                bx: 0.0,
                by: 0.0,
            });
        }
    }
    let n = nodes.len();

    // Per-joint physics constants, tunable from the demo options pane (env knobs;
    // see DELTA_PARAMS). Stiffness scales with branch thickness (~len), inertia
    // with mass*len^2 (~len^3 for a uniform rod). Natural frequency then goes as
    // 1/len: the trunk sways slowly, twigs flutter fast.
    let kk = param_f32("K", 4.0); // stiffness coefficient
    let dd = param_f32("D", 0.0055); // inertia density
    let zeta = param_f32("ZETA", 0.18); // damping ratio (underdamped -> lively)
    let wind_amt = param_f32("WIND", 1.0); // gust strength multiplier
    let turb_amt = param_f32("TURB", 1.0); // turbulence multiplier
    let rbow = param_f32("RBOW", 0.0); // 0 = palette colors, 1 = full rainbow gradient
    let stiff = |len: f32| kk * len;
    let inertia = |len: f32| (dd * len * len * len).max(1e-4);
    let damp = |len: f32| 2.0 * zeta * (stiff(len) * inertia(len)).sqrt();

    // Turbulent wind: a rightward gust that swells over time plus spatial chop.
    let wind = |x: f32, y: f32, time: f32| -> (f32, f32) {
        let gust = (0.6 + 0.5 * (time * 0.45).sin() + 0.25 * (time * 1.3 + 1.7).sin()) * wind_amt;
        let turb =
            turb_amt * (0.4 * (x * 0.15 + time * 1.1).sin() + 0.3 * (y * 0.2 - time * 0.9).sin());
        let fx = gust + turb;
        let fy = 0.15 * (x * 0.1 + time * 2.0).sin();
        (fx, fy)
    };

    let mut theta = vec![0.0f32; n];
    let mut omega = vec![0.0f32; n];
    // scratch for the per-step kinematic forward pass
    let mut wang = vec![0.0f32; n]; // world angle
    let mut tipx = vec![0.0f32; n];
    let mut tipy = vec![0.0f32; n];

    let windy = t != 0.0; // t==0 -> rest tree, byte-identical to the static render
    if windy {
        const WARM: f32 = 6.0;
        const DT: f32 = 0.08;
        let t0 = (t - WARM).max(0.0);
        let steps = (((t - t0) / DT).round() as i32).max(1);
        let mut time = t0;
        for _ in 0..steps {
            // forward pass: world angle + tip position from current deflections.
            for i in 0..n {
                let nd = &nodes[i];
                let (bx, by, pang) = if nd.parent < 0 {
                    (nd.bx, nd.by, 0.0)
                } else {
                    let pi = nd.parent as usize;
                    (tipx[pi], tipy[pi], wang[pi])
                };
                let a = pang + nd.rel + theta[i];
                wang[i] = a;
                tipx[i] = bx + a.cos() * nd.len * 1.8;
                tipy[i] = by + a.sin() * nd.len;
            }
            // integrate each joint (semi-implicit Euler).
            for i in 0..n {
                let nd = &nodes[i];
                let (bx, by) = if nd.parent < 0 {
                    (nd.bx, nd.by)
                } else {
                    let pi = nd.parent as usize;
                    (tipx[pi], tipy[pi])
                };
                let mx = (bx + tipx[i]) * 0.5; // segment midpoint (force sample point)
                let my = (by + tipy[i]) * 0.5;
                let (fx, fy) = wind(mx, my, time);
                let a = wang[i];
                // force component perpendicular to the branch -> bending torque.
                let perp = fx * (-a.sin()) + fy * a.cos();
                let exposure = 1.0 + 0.3 * nd.depth as f32; // tips catch more wind
                let torque = perp * nd.len * exposure
                    - stiff(nd.len) * theta[i]
                    - damp(nd.len) * omega[i];
                let alpha = torque / inertia(nd.len);
                omega[i] += alpha * DT;
                theta[i] += omega[i] * DT;
                theta[i] = theta[i].clamp(-0.7, 0.7); // keep branches from folding over
            }
            time += DT;
        }
    }

    // final forward pass + draw.
    for i in 0..n {
        let nd = &nodes[i];
        let (bx, by, pang) = if nd.parent < 0 {
            (nd.bx, nd.by, 0.0)
        } else {
            let pi = nd.parent as usize;
            (tipx[pi], tipy[pi], wang[pi])
        };
        let a = pang + nd.rel + theta[i];
        wang[i] = a;
        let ex = bx + a.cos() * nd.len * 1.8;
        let ey = by + a.sin() * nd.len;
        tipx[i] = ex;
        tipy[i] = ey;
        let mut col = lerp_color(palette[1], palette[3], nd.depth as f32 / 7.0);
        if rbow > 0.0 {
            // hue sweeps with depth (trunk -> tips) plus horizontal position, so the
            // canopy reads as a rainbow gradient. `rbow` blends it over the palette.
            let hue = ((nd.depth as f32 / 7.0) * 280.0 + (ex / width as f32) * 80.0).rem_euclid(360.0);
            let rainbow = hsl_to_rgb(hue as f64, 0.75, 0.55);
            col = lerp_color(col, rainbow, rbow);
        }
        pp_line(grid, bx.round() as i32, by.round() as i32, ex.round() as i32, ey.round() as i32, col);
        pp_put(grid, ex.round() as i32, ey.round() as i32, '◆', lighten(col, 10));
    }
}

// --- stained : Voronoi glass cells with dark leading + jewel seeds. ---
fn draw_stained(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng) {
    let nseeds = 10 + seed as usize % 12;
    let mut sites: Vec<(i32, i32, Color)> = Vec::new();
    for i in 0..nseeds {
        let x = rng.random_range(0..width) as i32;
        let y = rng.random_range(0..height) as i32;
        let base = [palette[1], palette[2], palette[3]][i % 3];
        let col = shift_hue(base, rng.random_range(-90..=90) as f64);
        sites.push((x, y, col));
    }
    let mut id = vec![vec![0usize; width]; height];
    for y in 0..height {
        for x in 0..width {
            let mut best = 0usize;
            let mut bd = i64::MAX;
            for (k, &(sx, sy, _)) in sites.iter().enumerate() {
                let dx = (x as i32 - sx) as i64;
                let dy = ((y as i32 - sy) * 2) as i64;
                let d = dx * dx + dy * dy;
                if d < bd {
                    bd = d;
                    best = k;
                }
            }
            id[y][x] = best;
            let glass = sites[best].2;
            let ch = if (x + y) % 2 == 0 { '∙' } else { '·' };
            grid[y][x] = Cell::new(ch, darken(glass, 8));
        }
    }
    let lead = darken(palette[0], 0);
    for y in 0..height {
        for x in 0..width {
            let here = id[y][x];
            let right = x + 1 < width && id[y][x + 1] != here;
            let down = y + 1 < height && id[y + 1][x] != here;
            if right && down {
                grid[y][x] = Cell::new('┼', lead);
            } else if right {
                grid[y][x] = Cell::new('│', lead);
            } else if down {
                grid[y][x] = Cell::new('─', lead);
            }
        }
    }
    for &(sx, sy, col) in &sites {
        pp_put(grid, sx, sy, '◆', lighten(col, 40));
    }
}

// ============================================================================
// Grid morphing. Tween two finished grids (any modes/seeds) at the pixel layer.
// Four strategies: dissolve, field, transport (glyphs travel), sdf (shapes melt).
// emit_grid + ASCII_GRID_DUMP let the morph driver capture frames by re-running
// the binary, so it works for every mode with no per-mode rewrite.
// ============================================================================

/// Render path used by every mode: dump a serialized grid when ASCII_GRID_DUMP
/// is set (for the morph driver to capture), otherwise paint to the terminal.
fn emit_grid(grid: &Grid) {
    use std::io::Write;
    if std::env::var("ASCII_GRID_DUMP").is_ok() {
        let s = serialize_grid(grid);
        let mut out = io::stdout().lock();
        let _ = out.write_all(s.as_bytes());
        let _ = out.flush();
    } else {
        render_grid(grid);
    }
}

fn grid_color_code(c: Color) -> String {
    match c {
        Color::Rgb { r, g, b } => format!("{},{},{}", r, g, b),
        _ => "x".to_string(),
    }
}

fn parse_color_code(s: &str) -> Color {
    if s == "x" {
        return Color::Reset;
    }
    let mut it = s.split(',');
    let r = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let g = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let b = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    Color::Rgb { r, g, b }
}

/// Lossless text serialization: "w h" header, then one "char_u32 fg bg" line per cell.
fn serialize_grid(grid: &Grid) -> String {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { 0 };
    let mut s = String::with_capacity(w * h * 12 + 16);
    s.push_str(&format!("{} {}\n", w, h));
    for row in grid {
        for c in row {
            s.push_str(&format!(
                "{} {} {}\n",
                c.ch as u32,
                grid_color_code(c.fg),
                grid_color_code(c.bg)
            ));
        }
    }
    s
}

fn parse_grid(s: &str) -> Grid {
    let mut lines = s.lines();
    let header = lines.next().unwrap_or("0 0");
    let mut hi = header.split_whitespace();
    let w: usize = hi.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let h: usize = hi.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut grid = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        for x in 0..w {
            if let Some(line) = lines.next() {
                let mut p = line.split_whitespace();
                let ch = p
                    .next()
                    .and_then(|v| v.parse::<u32>().ok())
                    .and_then(char::from_u32)
                    .unwrap_or(' ');
                let fg = parse_color_code(p.next().unwrap_or("x"));
                let bg = parse_color_code(p.next().unwrap_or("x"));
                grid[y][x] = Cell::with_bg(ch, fg, bg);
            }
        }
    }
    grid
}

/// Force a grid to (w, h) by truncating / padding with blanks.
fn fit_grid(g: Grid, w: usize, h: usize) -> Grid {
    let mut out = vec![vec![Cell::blank(); w]; h];
    for y in 0..h.min(g.len()) {
        for x in 0..w.min(g[y].len()) {
            out[y][x] = g[y][x];
        }
    }
    out
}

const MORPH_RAMP: [char; 9] = [' ', '·', '∙', ':', '+', '*', '#', '%', '@'];

fn morph_is_ink(c: &Cell) -> bool {
    c.ch != ' '
}

/// Coerce any color to an Rgb so lerp_color interpolates instead of snapping.
fn rgb_of(c: Color) -> Color {
    match c {
        Color::Rgb { .. } => c,
        _ => Color::Rgb { r: 10, g: 10, b: 12 },
    }
}

/// Approximate ink density of a glyph, for the `field` strategy.
fn ink_weight(ch: char) -> f32 {
    match ch {
        ' ' => 0.0,
        '·' | '˙' | '\'' | '°' | '`' => 0.15,
        '∙' | ':' | '.' | ',' => 0.28,
        '-' | '─' | '╌' | '│' | '╎' | '|' | '╵' | '╷' => 0.42,
        '+' | '=' | '◦' | '○' | '╱' | '╲' | '╭' | '╮' | '╰' | '╯' => 0.55,
        '*' | '◇' | '△' | '▽' | '□' | '◌' | '✦' | '✧' => 0.68,
        '#' | '◆' | '●' | '◉' | '▪' | '▫' | '◐' | '◑' => 0.82,
        '%' | '▒' | '▓' | '█' | '@' | '❀' | '❁' | '✺' => 0.95,
        _ => 0.6,
    }
}

struct Ink {
    x: f32,
    y: f32,
    ch: char,
    fg: Color,
}

fn ink_points(g: &Grid) -> Vec<Ink> {
    let mut v = Vec::new();
    for (y, row) in g.iter().enumerate() {
        for (x, c) in row.iter().enumerate() {
            if morph_is_ink(c) {
                v.push(Ink {
                    x: x as f32,
                    y: y as f32,
                    ch: c.ch,
                    fg: rgb_of(c.fg),
                });
            }
        }
    }
    v
}

/// 2-pass chamfer distance transform; vertical cost is doubled for the 2:1 cell
/// aspect so distances are visually round.
fn chamfer(mask: &[Vec<bool>], w: usize, h: usize) -> Vec<Vec<f32>> {
    let big = 1.0e6_f32;
    let (wh, wv, wd) = (1.0_f32, 2.0_f32, 2.236_f32);
    let mut d = vec![vec![big; w]; h];
    for y in 0..h {
        for x in 0..w {
            if mask[y][x] {
                d[y][x] = 0.0;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut m = d[y][x];
            if x > 0 {
                m = m.min(d[y][x - 1] + wh);
            }
            if y > 0 {
                m = m.min(d[y - 1][x] + wv);
                if x > 0 {
                    m = m.min(d[y - 1][x - 1] + wd);
                }
                if x + 1 < w {
                    m = m.min(d[y - 1][x + 1] + wd);
                }
            }
            d[y][x] = m;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let mut m = d[y][x];
            if x + 1 < w {
                m = m.min(d[y][x + 1] + wh);
            }
            if y + 1 < h {
                m = m.min(d[y + 1][x] + wv);
                if x + 1 < w {
                    m = m.min(d[y + 1][x + 1] + wd);
                }
                if x > 0 {
                    m = m.min(d[y + 1][x - 1] + wd);
                }
            }
            d[y][x] = m;
        }
    }
    d
}

/// Signed distance field: negative inside ink, positive outside, ~0 at the edge.
fn signed_df(g: &Grid, w: usize, h: usize) -> Vec<Vec<f32>> {
    let mut ink = vec![vec![false; w]; h];
    let mut bg = vec![vec![false; w]; h];
    for y in 0..h.min(g.len()) {
        for x in 0..w.min(g[y].len()) {
            let i = morph_is_ink(&g[y][x]);
            ink[y][x] = i;
            bg[y][x] = !i;
        }
    }
    let din = chamfer(&ink, w, h);
    let dbg = chamfer(&bg, w, h);
    let mut s = vec![vec![0.0_f32; w]; h];
    for y in 0..h {
        for x in 0..w {
            s[y][x] = din[y][x] - dbg[y][x];
        }
    }
    s
}

/// Precomputed morph between two same-size grids. Build once, sample many `t`.
struct MorphState {
    w: usize,
    h: usize,
    a: Grid,
    b: Grid,
    ta: Vec<Ink>,
    tb: Vec<Ink>,
    sa: Vec<Vec<f32>>,
    sb: Vec<Vec<f32>>,
}

impl MorphState {
    fn new(a: Grid, b: Grid) -> Self {
        let h = a.len();
        let w = if h > 0 { a[0].len() } else { 0 };
        let b = fit_grid(b, w, h);
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        // sort both point sets by the same space key (angle, then radius) so a
        // modulo-zip pairing reads as a coherent swirl rather than noise.
        let key = |p: &Ink| -> (f32, f32) {
            let ang = (p.y - cy).atan2(p.x - cx);
            let r = ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt();
            (ang, r)
        };
        let mut ta = ink_points(&a);
        let mut tb = ink_points(&b);
        ta.sort_by(|p, q| key(p).partial_cmp(&key(q)).unwrap_or(std::cmp::Ordering::Equal));
        tb.sort_by(|p, q| key(p).partial_cmp(&key(q)).unwrap_or(std::cmp::Ordering::Equal));
        let sa = signed_df(&a, w, h);
        let sb = signed_df(&b, w, h);
        MorphState { w, h, a, b, ta, tb, sa, sb }
    }

    fn frame(&self, t: f32, strategy: &str) -> Grid {
        let t = t.clamp(0.0, 1.0);
        match strategy {
            "transport" => {
                if self.ta.is_empty() || self.tb.is_empty() {
                    self.dissolve(t)
                } else {
                    self.transport(t)
                }
            }
            "sdf" => self.sdf(t),
            "field" => self.field(t),
            _ => self.dissolve(t),
        }
    }

    fn dissolve(&self, t: f32) -> Grid {
        let mut g = vec![vec![Cell::blank(); self.w]; self.h];
        for y in 0..self.h {
            for x in 0..self.w {
                let thr = pp_hash2(x as i32, y as i32, 1234);
                let src = if t < thr { &self.a[y][x] } else { &self.b[y][x] };
                g[y][x] = Cell::new(src.ch, rgb_of(src.fg));
            }
        }
        g
    }

    fn field(&self, t: f32) -> Grid {
        let mut g = vec![vec![Cell::blank(); self.w]; self.h];
        for y in 0..self.h {
            for x in 0..self.w {
                let wa = ink_weight(self.a[y][x].ch);
                let wb = ink_weight(self.b[y][x].ch);
                let wt = wa + (wb - wa) * t;
                let idx = (wt * (MORPH_RAMP.len() - 1) as f32).round() as usize;
                let col = lerp_color(rgb_of(self.a[y][x].fg), rgb_of(self.b[y][x].fg), t);
                g[y][x] = Cell::new(MORPH_RAMP[idx.min(MORPH_RAMP.len() - 1)], col);
            }
        }
        g
    }

    fn transport(&self, t: f32) -> Grid {
        let mut g = vec![vec![Cell::blank(); self.w]; self.h];
        let la = self.ta.len();
        let lb = self.tb.len();
        let n = la.max(lb);
        for i in 0..n {
            let pa = &self.ta[i % la];
            let pb = &self.tb[i % lb];
            let x = (pa.x + (pb.x - pa.x) * t).round() as i32;
            let y = (pa.y + (pb.y - pa.y) * t).round() as i32;
            let ch = if t < 0.5 { pa.ch } else { pb.ch };
            let fg = lerp_color(pa.fg, pb.fg, t);
            pp_put(&mut g, x, y, ch, fg);
        }
        g
    }

    fn sdf(&self, t: f32) -> Grid {
        let mut g = vec![vec![Cell::blank(); self.w]; self.h];
        for y in 0..self.h {
            for x in 0..self.w {
                let d = (1.0 - t) * self.sa[y][x] + t * self.sb[y][x];
                if d < 0.0 {
                    let depth = (-d).min(8.0);
                    let idx = ((depth / 8.0) * (MORPH_RAMP.len() - 1) as f32).round() as usize;
                    let ch = MORPH_RAMP[idx.clamp(1, MORPH_RAMP.len() - 1)];
                    let col = lerp_color(rgb_of(self.a[y][x].fg), rgb_of(self.b[y][x].fg), t);
                    g[y][x] = Cell::new(ch, col);
                }
            }
        }
        g
    }
}

/// In-process iterate render for modes that support it -- no subprocess fork,
/// no serialize/parse round trip. Returns None for modes not handled here so the
/// caller can fall back to the subprocess path. Reads the same ASCII_P_* knob env
/// as the dispatch, so live tuning still applies.
fn iterate_grid(mode: &str, seed: u64, theme: &str, w: usize, h: usize, t: f32) -> Option<Grid> {
    if w == 0 || h == 0 {
        return None;
    }
    let palette = if theme.is_empty() {
        make_palette(seed)
    } else {
        named_theme(theme).unwrap_or_else(|| make_palette(seed))
    };
    let mut grid = vec![vec![Cell::blank(); w]; h];
    let mut rng = StdRng::seed_from_u64(seed);
    match mode {
        "delta" => {
            draw_delta(&mut grid, w, h, seed, &palette, &mut rng, t);
            Some(grid)
        }
        "phyllotaxis" => {
            draw_phyllotaxis(&mut grid, w, h, seed, &palette, &mut rng, t);
            Some(grid)
        }
        "moire" => {
            draw_moire(&mut grid, w, h, seed, &palette, &mut rng, t);
            Some(grid)
        }
        "nebula" => {
            draw_nebula(&mut grid, w, h, seed, &palette, &mut rng, t);
            Some(grid)
        }
        "spiro" => Some(draw_spiro(grid, w, h, seed, palette, rng, t, &[])),
        "spiro-tile" => Some(draw_spiro_tile(grid, w, h, seed, palette, rng, t, &[])),
        "weave" => Some(draw_weave(grid, w, h, seed, palette, rng, t, &[])),
        "gears" => Some(draw_gears(grid, w, h, seed, palette, rng, t, &[])),
        "solar-system" => Some(draw_solar_system(grid, w, h, seed, palette, rng, t, &[])),
        "eyes3" => Some(draw_eyes3(grid, w, h, seed, palette, rng, t, &[])),
        "fullmetal-eyes" => Some(draw_fullmetal_eyes(grid, w, h, seed, palette, rng, t, &[])),
        "fullmetal-eyes2" => Some(draw_fullmetal_eyes2(grid, w, h, seed, palette, rng, t, &[])),
        _ => None,
    }
}

/// Render any (mode, seed) to a Grid by re-running this binary with the dump flag.
fn render_frame(exe: &std::path::Path, seed: u64, mode: &str, theme: &str, w: usize, h: usize) -> Option<Grid> {
    render_frame_t(exe, seed, mode, theme, w, h, 0.0)
}

/// Same, but pass an animation time `t` (ASCII_T) so parametric modes that read
/// it advance their phase -- the native "iterate" path.
fn render_frame_t(exe: &std::path::Path, seed: u64, mode: &str, theme: &str, w: usize, h: usize, t: f32) -> Option<Grid> {
    use std::process::Command;
    let mut cmd = Command::new(exe);
    cmd.arg(seed.to_string()).arg(mode);
    if !theme.is_empty() {
        cmd.arg(theme);
    }
    cmd.env("ASCII_GRID_DUMP", "1")
        .env("ASCII_GRID_W", w.to_string())
        .env("ASCII_GRID_H", h.to_string())
        .env("ASCII_T", format!("{}", t));
    let out = cmd.output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let g = parse_grid(&s);
    if g.is_empty() { None } else { Some(g) }
}

/// Paint a grid with each row positioned by an absolute cursor escape and NO
/// newlines, so the terminal can never scroll (the definitive anti-scrollback
/// measure for the morph player).
/// Write the SGR escape for a color directly into `s` (no per-call String alloc,
/// unlike crossterm's `SetForegroundColor(..).to_string()`). `fg` selects the
/// foreground (38/39) vs background (48/49) parameter group.
fn write_sgr(s: &mut String, c: Color, fg: bool) {
    use std::fmt::Write as _;
    match c {
        Color::Rgb { r, g, b } => {
            let lead = if fg { 38 } else { 48 };
            let _ = write!(s, "\x1b[{};2;{};{};{}m", lead, r, g, b);
        }
        Color::Reset => s.push_str(if fg { "\x1b[39m" } else { "\x1b[49m" }),
        other => {
            // rare named/ansi variants: fall back to crossterm's formatter.
            use crossterm::style::{SetBackgroundColor, SetForegroundColor};
            if fg {
                let _ = write!(s, "{}", SetForegroundColor(other));
            } else {
                let _ = write!(s, "{}", SetBackgroundColor(other));
            }
        }
    }
}

fn grid_to_ansi(grid: &Grid) -> String {
    use std::fmt::Write as _;
    // preallocate roughly enough for chars + cursor escapes + some color runs.
    let approx = grid.len() * (grid.first().map_or(0, |r| r.len()) + 8) + 64;
    let mut s = String::with_capacity(approx);
    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;
    for (y, row) in grid.iter().enumerate() {
        let _ = write!(s, "\x1b[{};1H", y + 1); // home of this row (1-based)
        let mut skip = false;
        for cell in row {
            if skip {
                skip = false;
                continue;
            }
            if cell.fg != cur_fg {
                write_sgr(&mut s, cell.fg, true);
                cur_fg = cell.fg;
            }
            if cell.bg != cur_bg {
                write_sgr(&mut s, cell.bg, false);
                cur_bg = cell.bg;
            }
            s.push(cell.ch);
            if char_width(cell.ch) == 2 {
                skip = true;
            }
        }
        if cur_bg != Color::Reset {
            write_sgr(&mut s, Color::Reset, false);
            cur_bg = Color::Reset;
        }
    }
    s.push_str("\x1b[0m");
    s
}

/// Smootherstep easing (6p^5 - 15p^4 + 10p^3): near-zero velocity at both ends,
/// fast through the middle. Gives a pleasant ease-in / ease-out.
fn ease_in_out(p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    p * p * p * (p * (p * 6.0 - 15.0) + 10.0)
}

/// Wind warp: horizontal shear of a single grid, strongest at the top (canopy
/// sways, roots stay put) and oscillating + gusting over `time`. No second frame
/// needed -- this animates one rendered scene "through the wind".
fn warp_wind(src: &Grid, time: f32, amp: f32) -> Grid {
    let h = src.len();
    let w = if h > 0 { src[0].len() } else { 0 };
    let mut g = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        // height factor: 1 at the top, 0 at the bottom (squared for a whip feel).
        let hf = if h > 1 { 1.0 - (y as f32 / (h as f32 - 1.0)) } else { 0.0 };
        let gust = 0.5 * (time * 0.37).sin() + 0.5; // 0..1 slow swell
        let sway = amp
            * hf
            * hf
            * (1.0 + gust)
            * ((time + y as f32 * 0.16).sin() + 0.45 * (time * 1.9 + y as f32 * 0.07).sin());
        let dx = sway.round() as i32;
        for x in 0..w {
            let sx = x as i32 - dx;
            if sx >= 0 && (sx as usize) < w {
                g[y][x] = src[y][sx as usize];
            }
        }
    }
    g
}

/// Nearest-cell sample from a source grid (out of bounds -> blank).
fn warp_sample(src: &Grid, sx: f32, sy: f32) -> Cell {
    let xi = sx.round() as i32;
    let yi = sy.round() as i32;
    if xi >= 0 && yi >= 0 && (yi as usize) < src.len() && (xi as usize) < src[0].len() {
        src[yi as usize][xi as usize]
    } else {
        Cell::blank()
    }
}

/// Toroidal drift: scroll the whole grid diagonally over time, wrapping around.
fn warp_drift(src: &Grid, time: f32, amp: f32) -> Grid {
    let h = src.len();
    let w = if h > 0 { src[0].len() } else { 0 };
    let dx = (time * amp).round() as i32;
    let dy = (time * amp * 0.4).round() as i32;
    let mut g = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        for x in 0..w {
            let sx = (x as i32 - dx).rem_euclid(w as i32) as usize;
            let sy = (y as i32 - dy).rem_euclid(h as i32) as usize;
            g[y][x] = src[sy][sx];
        }
    }
    g
}

/// Vortex swirl: rotate around the center, faster near the middle, spinning over time.
fn warp_swirl(src: &Grid, time: f32, amp: f32) -> Grid {
    let h = src.len();
    let w = if h > 0 { src[0].len() } else { 0 };
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let mut g = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = (y as f32 - cy) * 2.0; // square space
            let r = (dx * dx + dy * dy).sqrt();
            let ang = dy.atan2(dx) - time * amp * 0.3 / (1.0 + r * 0.05);
            let sx = cx + ang.cos() * r;
            let sy = cy + ang.sin() * r / 2.0; // undo aspect
            g[y][x] = warp_sample(src, sx, sy);
        }
    }
    g
}

/// Concentric ripple: radial sine displacement moving outward over time.
fn warp_ripple(src: &Grid, time: f32, amp: f32) -> Grid {
    let h = src.len();
    let w = if h > 0 { src[0].len() } else { 0 };
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let mut g = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = (y as f32 - cy) * 2.0;
            let r = (dx * dx + dy * dy).sqrt().max(0.001);
            let off = amp * (r * 0.4 - time * 1.2).sin();
            let nx = dx / r;
            let ny = dy / r;
            g[y][x] = warp_sample(src, x as f32 - nx * off, y as f32 - ny * off / 2.0);
        }
    }
    g
}

/// Breathe: gentle zoom pulse in/out around the center.
fn warp_breathe(src: &Grid, time: f32, amp: f32) -> Grid {
    let h = src.len();
    let w = if h > 0 { src[0].len() } else { 0 };
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let scale = 1.0 + amp * 0.06 * (time * 0.9).sin();
    let mut g = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        for x in 0..w {
            let sx = cx + (x as f32 - cx) / scale;
            let sy = cy + (y as f32 - cy) / scale;
            g[y][x] = warp_sample(src, sx, sy);
        }
    }
    g
}

/// Animated Voronoi: the `stained` tessellation with sites drifting on small
/// orbits over `time`, so the glass cells flow and re-tile continuously. Site
/// base positions/colors are deterministic from `seed`; only the orbit offset
/// moves, so it loops smoothly.
fn voronoi_flow_frame(w: usize, h: usize, seed: u64, time: f32, palette: &[Color; 5]) -> Grid {
    let mut rng = StdRng::seed_from_u64(seed);
    let nseeds = 10 + (seed as usize % 12);
    struct Site {
        bx: f32,
        by: f32,
        ox: f32,
        oy: f32,
        ph: f32,
        sp: f32,
        col: Color,
    }
    let mut sites: Vec<Site> = Vec::with_capacity(nseeds);
    for i in 0..nseeds {
        let bx = rng.random_range(0.0..w.max(1) as f32);
        let by = rng.random_range(0.0..h.max(1) as f32);
        let ox = rng.random_range(3.0..9.0);
        let oy = rng.random_range(1.5..4.5);
        let ph = rng.random_range(0.0..std::f32::consts::TAU);
        let sp = rng.random_range(0.3..0.9) * if rng.random_range(0..2) == 0 { -1.0 } else { 1.0 };
        let base = [palette[1], palette[2], palette[3]][i % 3];
        let col = shift_hue(base, rng.random_range(-90..=90) as f64);
        sites.push(Site { bx, by, ox, oy, ph, sp, col });
    }
    let pos: Vec<(f32, f32, Color)> = sites
        .iter()
        .map(|s| {
            (
                s.bx + (time * s.sp + s.ph).cos() * s.ox,
                s.by + (time * s.sp * 1.3 + s.ph).sin() * s.oy,
                s.col,
            )
        })
        .collect();

    let mut g = vec![vec![Cell::blank(); w]; h];
    let mut id = vec![vec![0usize; w]; h];
    for y in 0..h {
        for x in 0..w {
            let mut best = 0usize;
            let mut bd = f32::MAX;
            for (k, &(sx, sy, _)) in pos.iter().enumerate() {
                let dx = x as f32 - sx;
                let dy = (y as f32 - sy) * 2.0; // cell aspect
                let d = dx * dx + dy * dy;
                if d < bd {
                    bd = d;
                    best = k;
                }
            }
            id[y][x] = best;
            let glass = pos[best].2;
            let ch = if (x + y) % 2 == 0 { '∙' } else { '·' };
            g[y][x] = Cell::new(ch, darken(glass, 8));
        }
    }
    let lead = darken(palette[0], 0);
    for y in 0..h {
        for x in 0..w {
            let here = id[y][x];
            let right = x + 1 < w && id[y][x + 1] != here;
            let down = y + 1 < h && id[y + 1][x] != here;
            if right && down {
                g[y][x] = Cell::new('┼', lead);
            } else if right {
                g[y][x] = Cell::new('│', lead);
            } else if down {
                g[y][x] = Cell::new('─', lead);
            }
        }
    }
    for &(sx, sy, col) in &pos {
        pp_put(&mut g, sx.round() as i32, sy.round() as i32, '◆', lighten(col, 40));
    }
    g
}

/// Interactive morph player (standalone CLI entry). Owns the alt-screen/raw-mode
/// lifecycle, then delegates the loop to `morph_session`.
///   morph <modeA> <seedA> <modeB> <seedB> [strategy]
/// Keys: space play/pause · 1-4 strategy · ←/→ scrub · w walk seeds · n next · q quit
fn run_morph(args: &[String], default_seed: u64, theme: &str) {
    use crossterm::{cursor, execute, terminal};

    let mode_a = args.get(4).map(|s| s.as_str()).unwrap_or("forest").to_string();
    let seed_a: u64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(default_seed);
    let mode_b = args.get(6).map(|s| s.as_str()).unwrap_or(&mode_a).to_string();
    let seed_b: u64 = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(seed_a.wrapping_add(1));
    let strat = args.get(8).map(|s| s.as_str()).unwrap_or("transport").to_string();

    terminal::enable_raw_mode().unwrap();
    execute!(io::stdout(), terminal::EnterAlternateScreen).unwrap();
    morph_session(&mode_a, seed_a, &mode_b, seed_b, &strat, theme);
    execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen).unwrap();
    terminal::disable_raw_mode().unwrap();
}

/// The morph player loop. Assumes raw mode + alternate screen are already active
/// (so it composes inside `demo`). Returns when the user presses q/esc.
fn morph_session(mode_a: &str, seed_a: u64, mode_b: &str, seed_b: u64, strat0: &str, theme: &str) {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal,
    };
    use std::io::Write;
    use std::time::Duration;

    // stained morphs as Voronoi mush; flow its sites instead.
    let mut strat = if mode_a == "stained" && mode_a == mode_b {
        "vflow".to_string()
    } else {
        strat0.to_string()
    };
    let exe = std::env::current_exe().unwrap();
    let (tw, mut th) = terminal::size().unwrap_or((80, 45));
    let mut w = tw as usize;
    let mut h = (th as usize).saturating_sub(1).max(1); // leave a status row

    // palette for native animators (wind/vflow) that synthesize rather than morph.
    let palette = if theme.is_empty() {
        make_palette(seed_a)
    } else {
        named_theme(theme).unwrap_or_else(|| make_palette(seed_a))
    };

    let mut blank = vec![vec![Cell::blank(); w]; h];
    let fa = render_frame(&exe, seed_a, mode_a, theme, w, h).unwrap_or_else(|| blank.clone());
    let fb = render_frame(&exe, seed_b, mode_b, theme, w, h).unwrap_or_else(|| blank.clone());
    let mut st = MorphState::new(fa, fb);

    // walk state: when on, finishing 0->1 shifts B into A and loads the next seed.
    let mut walk = mode_a == mode_b;
    let mut walk_seed = seed_b;

    execute!(io::stdout(), cursor::Hide).unwrap();

    // `phase` is the linear clock; `t` is the eased morph position fed to the
    // renderer. Easing the value (not the clock) is what makes playback slow at
    // the ends and fast through the middle. `clock` free-runs for native
    // animators (wind/vflow) that aren't an A->B sweep.
    let mut phase = 0.0_f32;
    let mut dir = 1.0_f32;
    let mut playing = true;
    let mut clock = 0.0_f32;
    let speed = 0.011_f32;

    // Live knob editing while animating: same declared config + pane as the demo
    // browser. Auto-open when the mode declares knobs so they're visible on entry.
    let spec = mode_spec(mode_a);
    let mut pvals: Vec<f32> = spec.params.iter().map(|p| p.default).collect();
    let mut psel: usize = 0;
    let mut pane_open = !spec.params.is_empty();
    let has_params = !spec.params.is_empty();

    loop {
        // Push knob values to env so the iterate subprocess picks up live edits.
        // SAFETY: morph_session runs on the single demo thread.
        for (p, v) in spec.params.iter().zip(pvals.iter()) {
            unsafe { std::env::set_var(format!("ASCII_P_{}", p.key), format!("{}", v)) };
        }
        // When the pane is open, render the animation narrower so the tree isn't
        // hidden behind it (width-parametric strats only; warps/morph overlay).
        let pane_w = if pane_open { 34.min(w / 2) } else { 0 };
        let rw = w.saturating_sub(pane_w).max(1);

        if playing {
            clock += 0.06;
            phase += dir * speed;
            if phase >= 1.0 {
                if walk {
                    walk_seed = walk_seed.wrapping_add(1);
                    let next = render_frame(&exe, walk_seed, mode_a, theme, w, h)
                        .unwrap_or_else(|| blank.clone());
                    let prev_b = st.b.clone();
                    st = MorphState::new(prev_b, next);
                    phase = 0.0;
                    dir = 1.0;
                } else {
                    phase = 1.0;
                    dir = -1.0;
                }
            } else if phase <= 0.0 {
                phase = 0.0;
                dir = 1.0;
            }
        }

        let t = ease_in_out(phase);
        // native animators / warps ignore the A->B sweep; everything else morphs.
        let g = match strat.as_str() {
            "wind" => warp_wind(&st.a, clock, (h as f32 * 0.18).clamp(3.0, 8.0)),
            "drift" => warp_drift(&st.a, clock, 1.4),
            "swirl" => warp_swirl(&st.a, clock, 1.0),
            "ripple" => warp_ripple(&st.a, clock, 2.2),
            "breathe" => warp_breathe(&st.a, clock, 1.0),
            "vflow" => voronoi_flow_frame(rw, h, seed_a, clock, &palette),
            // Native T if the mode renders in-process; otherwise warp the base
            // frame over time (no per-frame fork -- the old fallback re-ran the
            // binary every frame and froze the player).
            "iterate" => iterate_grid(mode_a, seed_a, theme, rw, h, clock)
                .unwrap_or_else(|| warp_wind(&st.a, clock, (h as f32 * 0.12).clamp(2.0, 6.0))),
            _ => st.frame(t, &strat),
        };
        let body = grid_to_ansi(&g);
        // Overwrite in place: every grid row is full-width so it repaints every
        // cell -- no Clear needed (Clear + full-width writes were causing the
        // bottom-right autoscroll that spammed scrollback).
        let status = if pane_open && has_params {
            format!(
                " morph {} | {} | t={:.2} | {} | o=close opts  \u{2191}\u{2193}=select  \u{2190}\u{2192}=adjust  r=reset  i=iterate  q ",
                mode_a, strat, t, if playing { "\u{25b6}" } else { "\u{2161}" },
            )
        } else {
            format!(
                " morph {}:{} \u{2192} {}:{} | {} | t={:.2} | {} | space 1-4=morph 5-0=warp i=iterate o=opts \u{2190}\u{2192} w n q ",
                mode_a,
                seed_a,
                if walk { mode_a } else { mode_b },
                if walk { walk_seed } else { seed_b },
                strat,
                t,
                if playing { "\u{25b6}" } else { "\u{2161}" },
            )
        };
        // Leave the last cell of the last row untouched to avoid corner autoscroll.
        let status_w = w.saturating_sub(1);
        let status: String = status.chars().take(status_w).collect();
        let pad = status_w.saturating_sub(status.chars().count());
        let mut buf = String::new();
        buf.push_str(&body); // each row self-positions; no newlines
        buf.push_str(&format!("\x1b[{};1H", th)); // status on last row (1-based)
        buf.push_str(&format!("\x1b[7m{}{}\x1b[0m", status, " ".repeat(pad)));
        print!("{}", buf);
        io::stdout().flush().unwrap();
        if pane_open {
            // overlay the knob pane on the right; covers columns rw..w each frame.
            draw_options_pane(rw, th, mode_a, &spec, &pvals, psel, seed_a, theme);
        }

        if event::poll(Duration::from_millis(16)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key)) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char(' ') => playing = !playing,
                    KeyCode::Char('1') => strat = "dissolve".to_string(),
                    KeyCode::Char('2') => strat = "field".to_string(),
                    KeyCode::Char('3') => strat = "transport".to_string(),
                    KeyCode::Char('4') => strat = "sdf".to_string(),
                    KeyCode::Char('5') => strat = "wind".to_string(),
                    KeyCode::Char('6') => strat = "vflow".to_string(),
                    KeyCode::Char('7') => strat = "swirl".to_string(),
                    KeyCode::Char('8') => strat = "ripple".to_string(),
                    KeyCode::Char('9') => strat = "breathe".to_string(),
                    KeyCode::Char('0') => strat = "drift".to_string(),
                    KeyCode::Char('i') => strat = "iterate".to_string(),
                    KeyCode::Char('w') => walk = !walk,
                    KeyCode::Char('o') => pane_open = !pane_open,
                    KeyCode::Up if pane_open && has_params => {
                        psel = (psel + spec.params.len() - 1) % spec.params.len();
                    }
                    KeyCode::Down if pane_open && has_params => {
                        psel = (psel + 1) % spec.params.len();
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') if pane_open && has_params => {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] - p.step).max(p.min);
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') if pane_open && has_params => {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] + p.step).min(p.max);
                    }
                    KeyCode::Char('r') if pane_open && has_params => {
                        pvals[psel] = spec.params[psel].default;
                    }
                    KeyCode::Left if pane_open && has_params => {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] - p.step).max(p.min);
                    }
                    KeyCode::Right if pane_open && has_params => {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] + p.step).min(p.max);
                    }
                    KeyCode::Left => {
                        playing = false;
                        phase = (phase - 0.02).max(0.0);
                    }
                    KeyCode::Right => {
                        playing = false;
                        phase = (phase + 0.02).min(1.0);
                    }
                    KeyCode::Char('n') => {
                        // jump to next seed pair immediately
                        walk_seed = walk_seed.wrapping_add(1);
                        let next = render_frame(&exe, walk_seed, mode_a, theme, w, h)
                            .unwrap_or_else(|| blank.clone());
                        let prev_b = st.b.clone();
                        st = MorphState::new(prev_b, next);
                        phase = 0.0;
                        dir = 1.0;
                    }
                    _ => {}
                },
                Ok(Event::Resize(nw, nh)) => {
                    // re-render both frames at the new size and rebuild state.
                    th = nh;
                    w = nw as usize;
                    h = (nh as usize).saturating_sub(1).max(1);
                    blank = vec![vec![Cell::blank(); w]; h];
                    let (b_seed, b_mode) = if walk { (walk_seed, mode_a) } else { (seed_b, mode_b) };
                    let na = render_frame(&exe, seed_a, mode_a, theme, w, h)
                        .unwrap_or_else(|| blank.clone());
                    let nb = render_frame(&exe, b_seed, b_mode, theme, w, h)
                        .unwrap_or_else(|| blank.clone());
                    st = MorphState::new(na, nb);
                    phase = 0.0;
                    dir = 1.0;
                    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All)).unwrap();
                }
                _ => {}
            }
        }
    }

    // restore cursor; caller owns alt-screen/raw-mode teardown.
    execute!(io::stdout(), cursor::Show).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    fn assert_uniform_display_width(grid: &Grid, expected: usize) {
        let lines = grid_to_plain(grid);
        for (i, line) in lines.iter().enumerate() {
            let w = UnicodeWidthStr::width(line.as_str());
            assert_eq!(
                w, expected,
                "row {} has display width {} (expected {}): {:?}",
                i, w, expected, line,
            );
        }
    }

    fn make_grid(width: usize, height: usize, seed: u64) -> (Grid, StdRng, [Color; 5]) {
        let grid = vec![vec![Cell::blank(); width]; height];
        let rng = StdRng::seed_from_u64(seed);
        let palette = make_palette(seed);
        (grid, rng, palette)
    }

    fn grid_to_string(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn ease_in_out_shape() {
        assert!((ease_in_out(0.0) - 0.0).abs() < 1e-6);
        assert!((ease_in_out(1.0) - 1.0).abs() < 1e-6);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-6); // symmetric
        // middle is faster than the ends: derivative (delta over equal step) bigger at 0.5
        let d_end = ease_in_out(0.1) - ease_in_out(0.0);
        let d_mid = ease_in_out(0.55) - ease_in_out(0.45);
        assert!(d_mid > d_end, "middle should advance faster than the ends");
        // monotonic non-decreasing
        let mut prev = -1.0;
        for i in 0..=20 {
            let v = ease_in_out(i as f32 / 20.0);
            assert!(v >= prev - 1e-6, "must be monotonic");
            prev = v;
        }
    }

    #[test]
    fn warps_are_deterministic_and_animate() {
        let (mut g, mut rng, pal) = make_grid(80, 24, 4);
        draw_nebula(&mut g, 80, 24, 4, &pal, &mut rng, 0.0);
        for warp in [warp_drift, warp_swirl, warp_ripple, warp_breathe, warp_wind] {
            let a = warp(&g, 1.0, 2.0);
            assert_eq!(grid_to_string(&a), grid_to_string(&warp(&g, 1.0, 2.0)), "deterministic");
            let b = warp(&g, 4.5, 2.0);
            assert_ne!(grid_to_string(&a), grid_to_string(&b), "time should animate the warp");
        }
    }

    #[test]
    fn voronoi_flow_deterministic_and_moves() {
        let pal = make_palette(3);
        let f0 = voronoi_flow_frame(80, 24, 3, 0.0, &pal);
        let f0b = voronoi_flow_frame(80, 24, 3, 0.0, &pal);
        let f1 = voronoi_flow_frame(80, 24, 3, 5.0, &pal);
        assert_eq!(grid_to_string(&f0), grid_to_string(&f0b), "same args -> same frame");
        assert_ne!(grid_to_string(&f0), grid_to_string(&f1), "time should move the cells");
    }

    #[test]
    fn voronoi_flow_snapshot() {
        let pal = make_palette(3);
        insta::assert_snapshot!("voronoi_flow_t2", grid_to_string(&voronoi_flow_frame(80, 24, 3, 2.0, &pal)));
    }

    #[test]
    fn warp_wind_moves_and_zero_amp_identity() {
        let (mut g, mut rng, pal) = make_grid(80, 24, 1);
        draw_phyllotaxis(&mut g, 80, 24, 1, &pal, &mut rng, 0.0);
        // amplitude 0 -> no displacement -> identity
        assert_eq!(grid_to_string(&warp_wind(&g, 5.0, 0.0)), grid_to_string(&g));
        // deterministic, and different times differ
        let a = warp_wind(&g, 0.3, 5.0);
        assert_eq!(grid_to_string(&a), grid_to_string(&warp_wind(&g, 0.3, 5.0)));
        assert_ne!(grid_to_string(&a), grid_to_string(&warp_wind(&g, 2.1, 5.0)));
    }

    #[test]
    fn grid_serialize_roundtrip() {
        let (mut grid, mut rng, palette) = make_grid(20, 6, 42);
        draw_phyllotaxis(&mut grid, 20, 6, 42, &palette, &mut rng, 0.0);
        let restored = parse_grid(&serialize_grid(&grid));
        assert_eq!(restored.len(), grid.len());
        assert_eq!(restored[0].len(), grid[0].len());
        for y in 0..grid.len() {
            for x in 0..grid[0].len() {
                assert_eq!(restored[y][x].ch, grid[y][x].ch, "ch at {},{}", x, y);
                assert_eq!(restored[y][x].fg, grid[y][x].fg, "fg at {},{}", x, y);
            }
        }
    }

    fn morph_pair() -> MorphState {
        let (mut a, mut ra, pa) = make_grid(80, 24, 1);
        draw_phyllotaxis(&mut a, 80, 24, 1, &pa, &mut ra, 0.0);
        let (mut b, mut rb, pb) = make_grid(80, 24, 7);
        draw_phyllotaxis(&mut b, 80, 24, 7, &pb, &mut rb, 0.0);
        MorphState::new(a, b)
    }

    #[test]
    fn morph_dissolve_mid() {
        insta::assert_snapshot!("morph_dissolve_mid", grid_to_string(&morph_pair().frame(0.5, "dissolve")));
    }

    #[test]
    fn morph_field_mid() {
        insta::assert_snapshot!("morph_field_mid", grid_to_string(&morph_pair().frame(0.5, "field")));
    }

    #[test]
    fn morph_transport_mid() {
        insta::assert_snapshot!("morph_transport_mid", grid_to_string(&morph_pair().frame(0.5, "transport")));
    }

    #[test]
    fn morph_sdf_mid() {
        insta::assert_snapshot!("morph_sdf_mid", grid_to_string(&morph_pair().frame(0.5, "sdf")));
    }

    #[test]
    fn morph_endpoints_recover_inputs() {
        // at t=0 dissolve should be ~grid A, at t=1 ~grid B (char identity).
        let st = morph_pair();
        let f0 = st.frame(0.0, "dissolve");
        let f1 = st.frame(1.0, "dissolve");
        let mut a_match = 0;
        let mut b_match = 0;
        for y in 0..st.h {
            for x in 0..st.w {
                if f0[y][x].ch == st.a[y][x].ch {
                    a_match += 1;
                }
                if f1[y][x].ch == st.b[y][x].ch {
                    b_match += 1;
                }
            }
        }
        let total = st.w * st.h;
        assert_eq!(a_match, total, "t=0 should equal A");
        assert_eq!(b_match, total, "t=1 should equal B");
    }

    #[test]
    fn demo_filter_empty_matches_all() {
        let modes = ["party", "soup", "tree"];
        assert_eq!(demo_filter_modes(&modes, ""), vec![0, 1, 2]);
    }

    #[test]
    fn demo_filter_substring_case_insensitive() {
        let modes = ["forest", "forest++", "eyes++", "FullMetal"];
        assert_eq!(demo_filter_modes(&modes, "forest"), vec![0, 1]);
        assert_eq!(demo_filter_modes(&modes, "++"), vec![1, 2]);
        assert_eq!(demo_filter_modes(&modes, "metal"), vec![3]);
        assert!(demo_filter_modes(&modes, "zzz").is_empty());
    }

    #[test]
    fn eyes_pp_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_eyes_pp(&mut grid, 80, 24, 42, &palette, &mut rng);
        insta::assert_snapshot!("eyes_pp_42", grid_to_string(&grid));
    }

    #[test]
    fn fme_pp_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_fme_pp(&mut grid, 80, 24, 42, &palette, &mut rng);
        insta::assert_snapshot!("fme_pp_42", grid_to_string(&grid));
    }

    #[test]
    fn trees_pp_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_trees_pp(&mut grid, 80, 24, 42, &palette, &mut rng);
        insta::assert_snapshot!("trees_pp_42", grid_to_string(&grid));
    }

    #[test]
    fn forest_pp_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_forest_pp(&mut grid, 80, 24, 42, &palette, &mut rng);
        insta::assert_snapshot!("forest_pp_42", grid_to_string(&grid));
    }

    #[test]
    fn phyllotaxis_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_phyllotaxis(&mut grid, 80, 24, 42, &palette, &mut rng, 0.0);
        insta::assert_snapshot!("phyllotaxis_42", grid_to_string(&grid));
    }

    #[test]
    fn moire_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_moire(&mut grid, 80, 24, 42, &palette, &mut rng, 0.0);
        insta::assert_snapshot!("moire_42", grid_to_string(&grid));
    }

    #[test]
    fn nebula_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_nebula(&mut grid, 80, 24, 42, &palette, &mut rng, 0.0);
        insta::assert_snapshot!("nebula_42", grid_to_string(&grid));
    }

    #[test]
    fn delta_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_delta(&mut grid, 80, 24, 42, &palette, &mut rng, 0.0);
        insta::assert_snapshot!("delta_42", grid_to_string(&grid));
    }

    #[test]
    fn stained_42() {
        let (mut grid, mut rng, palette) = make_grid(80, 24, 42);
        draw_stained(&mut grid, 80, 24, 42, &palette, &mut rng);
        insta::assert_snapshot!("stained_42", grid_to_string(&grid));
    }

    #[test]
    fn mondrian_display_width() {
        let (mut grid, mut rng, _) = make_grid(80, 45, 42);
        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar {
                        label: "cpu".into(),
                        value: 72.0,
                        max: 100.0,
                    },
                    ContentItem::Bar {
                        label: "mem".into(),
                        value: 4.8,
                        max: 8.0,
                    },
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                    ContentItem::Text("tree-sit ···· 3".into()),
                ],
                padding: 1,
            },
        ];
        let (_, line_color) = mondrian_colors();
        let text_fg = rgb(20, 20, 20);
        let (fills, _) = mondrian_colors();
        layout_mondrian(
            &mut grid, &blocks, 0, 2, 10, 5, text_fg, line_color, &fills, line_color, &mut rng,
        );
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn mondrian_different_seeds_display_width() {
        for seed in [0, 1, 7, 42, 99, 1234] {
            let (mut grid, mut rng, _) = make_grid(80, 45, seed);
            let blocks = vec![ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("Online.".into()),
                ],
                padding: 1,
            }];
            let (fills, line_color) = mondrian_colors();
            layout_mondrian(
                &mut grid,
                &blocks,
                0,
                2,
                10,
                5,
                rgb(20, 20, 20),
                line_color,
                &fills,
                line_color,
                &mut rng,
            );
            assert_uniform_display_width(&grid, 80);
        }
    }

    #[test]
    fn default_mode_display_width() {
        let (mut grid, mut rng, palette) = make_grid(80, 45, 42);
        let truchet_color = darken(palette[1], 80);
        let tiles = ['╱', '╲'];
        for y in 0..45 {
            for x in 0..80 {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }
        let cx = 40;
        let cy = 22;
        let lines = ["「 技 」 S K I L L S", "", "  typespec ···· 12"];
        for (i, line) in lines.iter().enumerate() {
            let mut col = 0usize;
            for ch in line.chars() {
                let cw = char_width(ch);
                let gx = cx - 15 + col;
                if gx < 80 {
                    grid[cy - 5 + 1 + i][gx] = Cell::new(ch, palette[4]);
                    if cw == 2 && gx + 1 < 80 {
                        grid[cy - 5 + 1 + i][gx + 1] = Cell::blank();
                    }
                }
                col += cw;
            }
        }
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn bsp_display_width() {
        let (mut grid, mut rng, palette) = make_grid(80, 45, 42);
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..45 {
            for x in 0..80 {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }
        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("TASKS".into()),
                    ContentItem::Rule,
                    ContentItem::Text("▪ layout engine".into()),
                ],
                padding: 1,
            },
        ];
        layout_bsp(
            &mut grid, &blocks, 1, 12, 5, palette[4], palette[3], &mut rng,
        );
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn wrap_text_fullwidth_chars() {
        let lines = wrap_text("「 X 」", 7);
        assert_eq!(lines, vec!["「 X 」"]);

        let lines = wrap_text("「 X 」 extra", 7);
        assert_eq!(lines, vec!["「 X 」", "extra"]);
    }

    #[test]
    fn wrap_text_ascii_basic() {
        let lines = wrap_text("hello world foo", 11);
        assert_eq!(lines, vec!["hello world", "foo"]);
    }

    #[test]
    fn min_block_width_accounts_for_fullwidth() {
        let block = ContentBlock {
            items: vec![ContentItem::Text("「 SKILLS 」".into())],
            padding: 1,
        };
        assert_eq!(min_block_width(&block), 14);
    }

    #[test]
    fn bsp_split_gap_leaves_cover_canvas() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut root = BspNode::new(0, 0, 80, 45);
        root.split_with_gap(10, 5, 4, 2, &mut rng);
        let leaves = root.leaves();
        assert!(leaves.len() >= 2, "should produce multiple leaves");
        for leaf in &leaves {
            assert!(leaf.x + leaf.w <= 80, "leaf x overflow");
            assert!(leaf.y + leaf.h <= 45, "leaf y overflow");
            assert!(leaf.w >= 10, "leaf too narrow");
            assert!(leaf.h >= 5, "leaf too short");
        }
    }

    #[test]
    fn bsp_split_gap1_backward_compat() {
        let mut rng1 = StdRng::seed_from_u64(99);
        let mut rng2 = StdRng::seed_from_u64(99);
        let mut a = BspNode::new(0, 0, 80, 45);
        let mut b = BspNode::new(0, 0, 80, 45);
        a.split(10, 5, 4, &mut rng1);
        b.split_with_gap(10, 5, 4, 1, &mut rng2);
        let la: Vec<_> = a.leaves().iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
        let lb: Vec<_> = b.leaves().iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
        assert_eq!(la, lb);
    }

    #[test]
    fn mondrian_content_not_wrapped() {
        let (mut grid, mut rng, _) = make_grid(80, 45, 42);
        let blocks = vec![ContentBlock {
            items: vec![ContentItem::Text("「 SKILLS 」".into())],
            padding: 1,
        }];
        let (fills, line_color) = mondrian_colors();
        layout_mondrian(
            &mut grid,
            &blocks,
            0,
            2,
            10,
            5,
            rgb(20, 20, 20),
            line_color,
            &fills,
            line_color,
            &mut rng,
        );
        let lines = grid_to_plain(&grid);
        let skill_rows: Vec<_> = lines.iter().filter(|l| l.contains("SKILLS")).collect();
        assert_eq!(
            skill_rows.len(),
            1,
            "「 SKILLS 」 should appear on exactly one row"
        );
        assert!(
            skill_rows[0].contains("「 SKILLS 」"),
            "full title should be on one line, got: {:?}",
            skill_rows[0]
        );
    }

    #[test]
    fn scene_walk_produces_layers() {
        let mut rng = StdRng::seed_from_u64(42);
        let palette = make_palette(42);
        let mut root = layout::BspNode::new(0, 0, 80, 45);
        root.split_with_gap(12, 6, 4, 2, &mut rng);
        let leaves: Vec<Rect> = root.leaves().into_iter().copied().collect();
        let layers = walk_to_layers(&leaves, (40, 22), &palette, &mut rng);
        assert!(layers.len() > 0, "walker should produce at least one layer");
        assert!(
            layers.len() <= leaves.len() * 4,
            "layers bounded by leaves + scatter"
        );
        for layer in &layers {
            assert!(
                layer.mask.is_some(),
                "every scene-walk layer should be masked"
            );
        }
    }

    #[test]
    fn scene_walk_renders_without_panic() {
        for seed in [0, 1, 7, 42, 99, 1234] {
            let (mut grid, mut rng, palette) = make_grid(80, 45, seed);
            let mut root = layout::BspNode::new(0, 0, 80, 45);
            root.split_with_gap(12, 6, 4, 2, &mut rng);
            let leaves: Vec<Rect> = root.leaves().into_iter().copied().collect();
            let layers = walk_to_layers(&leaves, (40, 22), &palette, &mut rng);
            let scene = Scene { layers };
            let rect = Rect {
                x: 0,
                y: 0,
                w: 80,
                h: 45,
            };
            render_scene(&mut grid, &rect, &scene, &mut rng);
            assert_uniform_display_width(&grid, 80);
        }
    }

    #[test]
    fn scene_walk_deterministic() {
        let run = |seed: u64| {
            let mut rng = StdRng::seed_from_u64(seed);
            let palette = make_palette(seed);
            let mut root = layout::BspNode::new(0, 0, 60, 30);
            root.split_with_gap(10, 5, 4, 2, &mut rng);
            let leaves: Vec<Rect> = root.leaves().into_iter().copied().collect();
            let layers = walk_to_layers(&leaves, (30, 15), &palette, &mut rng);
            let mut grid = vec![vec![Cell::blank(); 60]; 30];
            let rect = Rect {
                x: 0,
                y: 0,
                w: 60,
                h: 30,
            };
            let scene = Scene { layers };
            render_scene(&mut grid, &rect, &scene, &mut rng);
            grid_to_plain(&grid)
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(99));
    }

    #[test]
    fn tile_edge_seigaiha_skew_deterministic() {
        // Seigaiha with skew should produce identical output for same seed
        let run = |seed: u64| {
            let (mut grid, mut rng, palette) = make_grid(40, 20, seed);
            let rect = Rect {
                x: 5,
                y: 3,
                w: 25,
                h: 12,
            };
            let params = TileParams {
                variant: TileVariant::Seigaiha,
                density: 1.0,
                stagger_override: -1,
                rhythm_override: 0,
                jitter: 0.0,
                skew: 60,
            };
            fill_tile_ex(
                &mut grid, &rect, &params, palette[1], palette[2], 0.0, None, &mut rng,
            );
            grid_to_plain(&grid)
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(99));
    }

    #[test]
    fn tile_edge_skew_bleeds_past_rect() {
        // With skew>0, cells outside the rect should get drawn
        let (mut grid, mut rng, palette) = make_grid(40, 20, 42);
        let rect = Rect {
            x: 10,
            y: 5,
            w: 15,
            h: 8,
        };
        let params = TileParams {
            variant: TileVariant::Seigaiha,
            density: 1.0,
            stagger_override: -1,
            rhythm_override: 0,
            jitter: 0.0,
            skew: 80,
        };
        fill_tile_ex(
            &mut grid, &rect, &params, palette[1], palette[2], 0.0, None, &mut rng,
        );

        // Check that at least some cells outside the rect got drawn
        let mut outside_drawn = 0;
        for y in 0..20 {
            for x in 0..40 {
                let inside =
                    x >= rect.x && x < rect.x + rect.w && y >= rect.y && y < rect.y + rect.h;
                if !inside && grid[y][x].ch != ' ' {
                    outside_drawn += 1;
                }
            }
        }
        assert!(
            outside_drawn > 0,
            "skew=80 should bleed chars outside the rect"
        );
    }

    #[test]
    fn tile_edge_all_variants_no_panic() {
        // Every variant with skew should render without panic
        for vi in 0..TILE_VARIANT_COUNT {
            let variant = tile_variant_from_index(vi);
            for skew in [0, 30, 60, 100] {
                let (mut grid, mut rng, palette) = make_grid(30, 15, 42);
                let rect = Rect {
                    x: 3,
                    y: 2,
                    w: 20,
                    h: 10,
                };
                let params = TileParams {
                    variant,
                    density: 1.0,
                    stagger_override: -1,
                    rhythm_override: 0,
                    jitter: 0.0,
                    skew,
                };
                fill_tile_ex(
                    &mut grid, &rect, &params, palette[1], palette[2], 0.0, None, &mut rng,
                );
            }
        }
    }
}
