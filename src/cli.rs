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
use crate::gridio::*;
use crate::ink::*;
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;
use crate::cli_basic::*;
use crate::cli_catalog::*;
use crate::cli_city::*;
use crate::cli_fa::*;
use crate::cli_forest::*;
use crate::cli_scenes::*;
use crate::modes_geo::draw_weave;

pub(crate) fn run() {
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
        eprintln!("  snakes    Circuit traces slithering around hidden loops, crossover knots [count]");
        eprintln!("  quilt     Stitched patchwork of tile patterns [min_patch] [max_patch]");
        eprintln!("  patchwalk Quilted mondrian crossed by a waypoint trail [stops] [line_w]");
        eprintln!(
            "  hypercube Seeded 4D wireframes rotating through space [copies] [speed] [ghosts]"
        );
        eprintln!(
            "  flux      Curling particle currents with persistent trails [particles] [trail] [speed]"
        );
        eprintln!(
            "  fireworks Looping launches and gravity-bent starbursts [bursts] [sparks] [speed]"
        );
        eprintln!(
            "  fa6       Spatial transmutation engine [chambers] [inscriptions] [speed] [asymmetry%]"
        );
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
        eprintln!("  rhizome   Avant-garde tree-root network [count] [depth]");
        eprintln!("  effigy    Scattered algorithmic face masks [count]");
        eprintln!("  dendrite  Neuronal binary tree growth [seeds] [depth]");
        eprintln!("  totem     Stacked face poles [poles]");
        eprintln!("  chimera   Special tree+face hybrid scene [density]");
        eprintln!(
            "  murmuration  Wheeling starling flocks over a dusk gradient [birds] [flocks] [speed]"
        );
        eprintln!(
            "  lanterns  Paper lanterns rising off dark water, halo + reflection [count] [rise] [sway]"
        );
        eprintln!(
            "  tide      Superposed wave fronts washing a seeded shore, wet-sand recall [waves] [amp] [speed]"
        );
        eprintln!(
            "  fireflies Dusk meadow: drifting glow moths, blink cycles, swaying grass [count] [glow] [speed]"
        );
        eprintln!(
            "  ink       Ink drops blooming in still water, tendrils + differential swirl [drops] [swirl] [speed]"
        );
        eprintln!(
            "  meteors   Night sky: twinkle field, milky band, scheduled shooting stars [stars] [rate] [speed]"
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
        let (g, done) = cli_swatch(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "tree" {
        let (g, done) = cli_tree(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "trees" {
        let (g, done) = cli_trees(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "aztec" {
        let (g, done) = cli_aztec(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fret" {
        let (g, done) = cli_fret(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "flowers" {
        let (g, done) = cli_flowers(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fruits" {
        let (g, done) = cli_fruits(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "forest" {
        let (g, done) = cli_forest(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "layout" {
        let (g, done) = cli_layout(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "md" {
        let (g, done) = cli_md(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "bsp" {
        let (g, done) = cli_bsp(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "mondrian" {
        let (g, done) = cli_mondrian(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "tiles" {
        let (g, done) = cli_tiles(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "tiles-rand" {
        let (g, done) = cli_tiles_rand(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "tiles-skew" {
        let (g, done) = cli_tiles_skew(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "terrain" {
        let (g, done) = cli_terrain(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "flow" {
        let (g, done) = cli_flow(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "watershed" {
        let (g, done) = cli_watershed(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "masks" {
        let (g, done) = cli_masks(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "ca" || (mode.starts_with("ca-") && mode != "ca-layout") {
        let (g, done) = cli_ca(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "ca-layout" {
        let (g, done) = cli_ca_layout(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "shapes" {
        let (g, done) = cli_shapes(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "party" {
        let (g, done) = cli_party(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "soup" {
        let (g, done) = cli_soup(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "stem" {
        let (g, done) = cli_stem(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "scene-walk" {
        let (g, done) = cli_scene_walk(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "scene-walk-2" {
        let (g, done) = cli_scene_walk_2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "scene-walk-3" {
        let (g, done) = cli_scene_walk_3(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "forest2" {
        let (g, done) = cli_forest2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "forest3" {
        let (g, done) = cli_forest3(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "forest4" {
        let (g, done) = cli_forest4(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "forest5" {
        let (g, done) = cli_forest5(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "forest6" {
        let (g, done) = cli_forest6(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "boles1" {
        let (g, done) = cli_boles1(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "boles2" {
        let (g, done) = cli_boles2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "boles3" {
        let (g, done) = cli_boles3(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "boles4" {
        let (g, done) = cli_boles4(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "boles5" {
        let (g, done) = cli_boles5(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "trunks1" {
        let (g, done) = cli_trunks1(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "trees1" {
        let (g, done) = cli_trees1(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "trees2" {
        let (g, done) = cli_trees2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "trees3" {
        let (g, done) = cli_trees3(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "trees4" {
        let (g, done) = cli_trees4(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "trees8" {
        let (g, done) = cli_trees8(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "trees9" {
        let (g, done) = cli_trees9(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "bushes" {
        let (g, done) = cli_bushes(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "forest7" {
        let (g, done) = cli_forest7(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "forest8" {
        let (g, done) = cli_forest8(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "forest9" {
        let (g, done) = cli_forest9(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "boles6" {
        let (g, done) = cli_boles6(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "trees10" {
        let (g, done) = cli_trees10(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "mondrian2" {
        let (g, done) = cli_mondrian2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "kintsugi" {
        let (g, done) = cli_kintsugi(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "constellation" {
        let (g, done) = cli_constellation(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "strata" {
        let (g, done) = cli_strata(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "circuit" {
        let (g, done) = cli_circuit(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "snakes" {
        let (g, done) = cli_snakes(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "quilt" {
        let (g, done) = cli_quilt(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "patchwalk" {
        let (g, done) = cli_patchwalk(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "aurora" {
        let (g, done) = cli_aurora(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "aura2" {
        let (g, done) = cli_aura2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "harbor" {
        let (g, done) = cli_harbor(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "labyrinth" {
        let (g, done) = cli_labyrinth(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "rainfall" {
        let (g, done) = cli_rainfall(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "meadow" {
        let (g, done) = cli_meadow(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "solar-system" {
        let (g, done) = cli_solar_system(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "world2" {
        let (g, done) = cli_world2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "eyes" {
        let (g, done) = cli_eyes(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "eyes2" {
        let (g, done) = cli_eyes2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "eyes3" {
        let (g, done) = cli_eyes3(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fullmetal-eyes" {
        let (g, done) = cli_fullmetal_eyes(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fullmetal-eyes2" {
        let (g, done) = cli_fullmetal_eyes2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fullmetal-alchemist" {
        let (g, done) = cli_fullmetal_alchemist(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fullmetal-alchemist2" {
        let (g, done) = cli_fullmetal_alchemist2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fa3" || mode == "fullmetal-alchemist3" {
        let (g, done) = cli_fa3_fullmetal_alchemist3(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fa4" || mode == "fullmetal-alchemist4" {
        let (g, done) = cli_fa4_fullmetal_alchemist4(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fa5" || mode == "fullmetal-alchemist5" {
        let (g, done) = cli_fa5_fullmetal_alchemist5(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "spiro" {
        let (g, done) = cli_spiro(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "spiro-tile" {
        let (g, done) = cli_spiro_tile(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "weave" {
        let (g, done) = cli_weave(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "gears" {
        let (g, done) = cli_gears(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "kaleido" {
        let (g, done) = cli_kaleido(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "contour" {
        let (g, done) = cli_contour(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "metro" {
        let (g, done) = cli_metro(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "koi" {
        let (g, done) = cli_koi(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "skyline" {
        let (g, done) = cli_skyline(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "hive" {
        let (g, done) = cli_hive(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "jelly" {
        let (g, done) = cli_jelly(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "jelly2" {
        let (g, done) = cli_jelly2(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fa6" || mode == "fullmetal-alchemist6" {
        let (g, done) = cli_fa6_fullmetal_alchemist6(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "hypercube" {
        let (g, done) = cli_hypercube(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "flux" {
        let (g, done) = cli_flux(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fireworks" {
        let (g, done) = cli_fireworks(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "rhizome" {
        let (g, done) = cli_rhizome(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "effigy" {
        let (g, done) = cli_effigy(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "dendrite" {
        let (g, done) = cli_dendrite(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "totem" {
        let (g, done) = cli_totem(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "chimera" {
        let (g, done) = cli_chimera(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "murmuration" {
        let (g, done) = cli_murmuration(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "lanterns" {
        let (g, done) = cli_lanterns(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "tide" {
        let (g, done) = cli_tide(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fireflies" {
        let (g, done) = cli_fireflies(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "ink" {
        let (g, done) = cli_ink(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "meteors" {
        let (g, done) = cli_meteors(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "world" {
        let (g, done) = cli_world(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "noise" {
        let (g, done) = cli_noise(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "eyes++" {
        let (g, done) = cli_eyes(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "fullmetal-eyes++" {
        let (g, done) = cli_fullmetal_eyes(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "trees++" {
        let (g, done) = cli_trees(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "forest++" {
        let (g, done) = cli_forest(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "phyllotaxis" {
        let (g, done) = cli_phyllotaxis(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "moire" {
        let (g, done) = cli_moire(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "nebula" {
        let (g, done) = cli_nebula(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "delta" {
        let (g, done) = cli_delta(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else if mode == "stained" {
        let (g, done) = cli_stained(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    } else {
        let (g, done) = cli_default(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
        grid = g;
        if done {
            return;
        }
    }

    emit_grid(&grid);
}

