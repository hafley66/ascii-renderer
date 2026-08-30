#![allow(warnings)]

use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

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
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;
use crate::modes_geo::draw_weave;
use crate::cli_city::draw_elevator;
use crate::cli_city::draw_ferris;

pub(crate) const MORPH_RAMP: [char; 9] = [' ', '·', '∙', ':', '+', '*', '#', '%', '@'];

pub(crate) fn morph_is_ink(c: &Cell) -> bool {
    c.ch != ' '
}


/// Precomputed morph between two same-size grids. Build once, sample many `t`.
pub(crate) struct MorphState {
    pub(crate) w: usize,
    pub(crate) h: usize,
    pub(crate) a: Grid,
    pub(crate) b: Grid,
    pub(crate) ta: Vec<Ink>,
    pub(crate) tb: Vec<Ink>,
    pub(crate) sa: Vec<Vec<f32>>,
    pub(crate) sb: Vec<Vec<f32>>,
}

impl MorphState {
    pub(crate) fn new(a: Grid, b: Grid) -> Self {
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

    pub(crate) fn frame(&self, t: f32, strategy: &str) -> Grid {
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
pub(crate) fn iterate_grid(mode: &str, seed: u64, theme: &str, w: usize, h: usize, t: f32) -> Option<Grid> {
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
        "circuit" => {
            draw_circuit(&mut grid, w, h, seed, &palette, &mut rng, t, 14);
            Some(grid)
        }
        "snakes" => {
            draw_snakes(&mut grid, w, h, seed, &palette, &mut rng, t, 7);
            Some(grid)
        }
        "nebula" => {
            draw_nebula(&mut grid, w, h, seed, &palette, &mut rng, t);
            Some(grid)
        }
        "arboretum" => {
            let knobs = crate::arboretum::ForestKnobs::from_env();
            Some(crate::arboretum::render_arboretum_frame(w, h, seed, &palette, rng, t, &knobs))
        }
        "astrolabe" => {
            crate::astrolabe::draw_astrolabe(&mut grid, w, h, seed, &palette, t);
            Some(grid)
        }
        "sauron" => {
            crate::sauron::draw_sauron(&mut grid, w, h, seed, &palette, t);
            Some(grid)
        }
        "illuminarium" => {
            let params = crate::illuminarium::IlluminariumParams::from_args(&[]);
            crate::illuminarium::draw_illuminarium(
                &mut grid,
                w,
                h,
                seed,
                &palette,
                &mut rng,
                t,
                &params,
            );
            Some(grid)
        }
        "spiro" => Some(draw_spiro(grid, w, h, seed, palette, rng, t, &[])),
        "spiro-tile" => Some(draw_spiro_tile(grid, w, h, seed, palette, rng, t, &[])),
        "weave" => Some(draw_weave(grid, w, h, seed, palette, rng, t, &[])),
        "gears" => Some(draw_gears(grid, w, h, seed, palette, rng, t, &[])),
        "kaleido" => Some(draw_kaleido(grid, w, h, seed, palette, rng, t, &[])),
        "contour" => Some(draw_contour(grid, w, h, seed, palette, rng, t, &[])),
        "solar-system" => Some(draw_solar_system(grid, w, h, seed, palette, rng, t, &[])),
        "eyes3" => Some(draw_eyes3(grid, w, h, seed, palette, rng, t, &[])),
        "fullmetal-eyes" => Some(draw_fullmetal_eyes(grid, w, h, seed, palette, rng, t, &[])),
        "fullmetal-eyes2" => Some(draw_fullmetal_eyes2(grid, w, h, seed, palette, rng, t, &[])),
        "hypercube" => {
            draw_hypercube(
                &mut grid,
                w,
                h,
                seed,
                &palette,
                &mut rng,
                t,
                param_f32("COPIES", 3.0) as usize,
                param_f32("SPEED", 1.0),
                param_f32("GHOSTS", 2.0) as usize,
            );
            Some(grid)
        }
        "flux" => {
            draw_flux(
                &mut grid,
                w,
                h,
                seed,
                &palette,
                &mut rng,
                t,
                param_f32("COUNT", 58.0) as usize,
                param_f32("TRAIL", 8.0) as usize,
                param_f32("SPEED", 1.0),
            );
            Some(grid)
        }
        "fireworks" => {
            draw_fireworks(
                &mut grid,
                w,
                h,
                seed,
                &palette,
                &mut rng,
                t,
                param_f32("BURSTS", 6.0) as usize,
                param_f32("SPARKS", 22.0) as usize,
                param_f32("SPEED", 1.0),
            );
            Some(grid)
        }
        "elevator" => {
            draw_elevator(
                &mut grid,
                w,
                h,
                seed,
                &palette,
                &mut rng,
                t,
                param_f32("LIFTS", 3.0) as usize,
                param_f32("SPEED", 1.0),
                param_f32("CROWD", 1.0),
            );
            Some(grid)
        }
        "ferris" => {
            draw_ferris(
                &mut grid,
                w,
                h,
                seed,
                &palette,
                &mut rng,
                t,
                param_f32("RADIUS", 8.0) as usize,
                param_f32("GONDOLAS", 10.0) as usize,
                param_f32("SPEED", 1.0),
            );
            Some(grid)
        }
        "murmuration" => {
            draw_murmuration(
                &mut grid,
                w,
                h,
                seed,
                &palette,
                &mut rng,
                t,
                param_f32("BIRDS", 140.0) as usize,
                param_f32("FLOCKS", 3.0) as usize,
                param_f32("SPEED", 1.0),
            );
            Some(grid)
        }
        "lanterns" => {
            draw_lanterns(
                &mut grid,
                w,
                h,
                seed,
                &palette,
                &mut rng,
                t,
                param_f32("COUNT", 7.0) as usize,
                param_f32("RISE", 1.0),
                param_f32("SWAY", 1.0),
            );
            Some(grid)
        }
        "tide" => {
            draw_tide(
                &mut grid,
                w,
                h,
                seed,
                &palette,
                &mut rng,
                t,
                param_f32("WAVES", 2.0) as usize,
                param_f32("AMP", 1.0),
                param_f32("SPEED", 1.0),
            );
            Some(grid)
        }
        "fa6" | "fullmetal-alchemist6" => {
            draw_fa6(
                &mut grid,
                w,
                h,
                seed,
                &palette,
                &mut rng,
                t,
                param_f32("CELLS", 8.0) as usize,
                param_f32("DENS", 55.0) as u32,
                param_f32("SPEED", 0.8),
                param_f32("CHAOS", 42.0) / 100.0,
            );
            Some(grid)
        }
        _ => None,
    }
}


/// Render any (mode, seed) to a Grid by re-running this binary with the dump flag.
pub(crate) fn render_frame(exe: &std::path::Path, seed: u64, mode: &str, theme: &str, w: usize, h: usize) -> Option<Grid> {
    render_frame_t(exe, seed, mode, theme, w, h, 0.0)
}


/// Same, but pass an animation time `t` (ASCII_T) so parametric modes that read
/// it advance their phase -- the native "iterate" path.
pub(crate) fn render_frame_t(exe: &std::path::Path, seed: u64, mode: &str, theme: &str, w: usize, h: usize, t: f32) -> Option<Grid> {
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


/// Interactive morph player (standalone CLI entry). Owns the alt-screen/raw-mode
/// lifecycle, then delegates the loop to `morph_session`.
///   morph <modeA> <seedA> <modeB> <seedB> [strategy]
/// Keys: space play/pause · 1-4 strategy · ←/→ scrub · w walk seeds · n next · q quit
pub(crate) fn run_morph(args: &[String], default_seed: u64, theme: &str) {
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
pub(crate) fn morph_session(mode_a: &str, seed_a: u64, mode_b: &str, seed_b: u64, strat0: &str, theme: &str) {
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
    let mut saved = load_options();
    let mut randomize = load_randomize(&saved);
    let mut roll: u64 = 0; // re-roll nonce for randomize mode
    let mut pvals: Vec<f32> = pvals_for(&spec, mode_a, &saved);
    let mut psel: usize = 0;
    let mut pane_open = !spec.params.is_empty();
    let has_params = !spec.params.is_empty();

    loop {
        // Push knob values to env so the iterate subprocess picks up live edits.
        // Randomize -> per-seed random samples instead of the tuned pvals.
        // SAFETY: morph_session runs on the single demo thread.
        let eff = effective_pvals(&spec, &pvals, seed_a, randomize, roll);
        for (p, v) in spec.params.iter().zip(eff.iter()) {
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
            draw_options_pane(rw, th, mode_a, &spec, &eff, psel, seed_a, theme, randomize);
        }

        if event::poll(Duration::from_millis(16)).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(key)) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('g') => {
                        randomize = !randomize;
                        store_randomize(&mut saved, randomize);
                    }
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
                    KeyCode::Char('-') | KeyCode::Char('_') if pane_open && has_params && randomize => {
                        roll = roll.wrapping_sub(1);
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') if pane_open && has_params && randomize => {
                        roll = roll.wrapping_add(1);
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') if pane_open && has_params => {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] - p.step).max(p.min);
                        store_pvals(mode_a, &spec, &pvals, &mut saved);
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') if pane_open && has_params => {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] + p.step).min(p.max);
                        store_pvals(mode_a, &spec, &pvals, &mut saved);
                    }
                    KeyCode::Char('r') if pane_open && has_params => {
                        pvals[psel] = spec.params[psel].default;
                        store_pvals(mode_a, &spec, &pvals, &mut saved);
                    }
                    KeyCode::Left if pane_open && has_params && randomize => {
                        roll = roll.wrapping_sub(1);
                    }
                    KeyCode::Right if pane_open && has_params && randomize => {
                        roll = roll.wrapping_add(1);
                    }
                    KeyCode::Left if pane_open && has_params => {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] - p.step).max(p.min);
                        store_pvals(mode_a, &spec, &pvals, &mut saved);
                    }
                    KeyCode::Right if pane_open && has_params => {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] + p.step).min(p.max);
                        store_pvals(mode_a, &spec, &pvals, &mut saved);
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
