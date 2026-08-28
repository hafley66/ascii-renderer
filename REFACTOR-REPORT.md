# Refactor report

Base sha: 8f8ce23. No behavior change; `cargo test` 95 passed / 0 failed, zero snapshot changes.

## File map (old -> new)

| Old file (lines) | New module(s) (lines) | Contents |
|---|---|---|
| `src/main.rs` (19581) | `src/main.rs` (865) | `#![allow]`, mod decls, glob imports, thin `fn main()`, `mod tests` (786 lines, unchanged) |
| | `src/cli.rs` (964) | `pub(crate) fn run()`: help text, arg/theme/terminal setup, demo+morph shortcuts, if/else chain of per-mode dispatch calls, `emit_grid` epilogue |
| | `src/cli_basic.rs` (1600) | dispatch arms: swatch, tree(s), aztec, fret, flowers, fruits, forest, layout, md, bsp, mondrian, tiles*, terrain, flow, watershed, masks, ca*, shapes, mondrian2, quilt, default card |
| | `src/cli_forest.rs` (1874) | dispatch arms: forest2-9 |
| | `src/cli_catalog.rs` (1401) | dispatch arms: boles1-6, trunks1, trees1-4/8-10, bushes (tree catalog pages) |
| | `src/cli_scenes.rs` (1966) | dispatch arms: party, soup, stem, scene-walk*, world2, patchwalk, labyrinth, harbor, rainfall, meadow, aurora, aura2, kintsugi, constellation, strata |
| | `src/cli_fa.rs` (2407) | dispatch arms: fullmetal-alchemist family (fa/2/3/4/5/6, fullmetal-eyes/2) |
| | `src/cli_city.rs` (1848) | dispatch arms: eyes, eyes2, jelly, jelly2, koi, metro, skyline, hive |
| | match-local structs (`TreeSlot`, `Snake`, `Flock`, `Fly`, `Star`, `Drop`, `Chamber`, `Node`, `Pending`, `SK`, `F7Element`, `F7Stop`, `Site`, consts `SPEED`/`TAIL`/`WARM`/`DT`/`SLOTS`) | moved inside their arm's extracted `cli_<mode>` fn, verbatim |
| | `src/registry.rs` (286) | `Param`, `AnimKind`, `ModeSpec`, `param!` macro, `ModeForm`, `MODE_FORMS`, `mode_spec`, `anim_strat`, `draw_options_pane` |
| | `src/opts.rs` (601) | `OptMap`, options load/save, `param_f32`, `pvals_for`, `rand_knob`, `effective_pvals`, randomize store/load, `demo_filter_modes`, `demo_pick_mode`, `run_demo` |
| | `src/pp.rs` (180) | `pp_put/point_on/stroke/line/arc/hash2/vnoise/fbm`, `chamfer`, `signed_df`, `ease_in_out` |
| | `src/gridio.rs` (153) | `emit_grid`, `grid_color_code`, `parse_color_code`, `serialize_grid`, `parse_grid`, `fit_grid`, `rgb_of`, `write_sgr`, `grid_to_ansi` |
| | `src/morph.rs` (639) | `MorphState` + impls, `MORPH_RAMP`, `morph_is_ink`, `iterate_grid`, `render_frame`, `render_frame_t`, `run_morph`, `morph_session` |
| | `src/warps.rs` (251) | `warp_wind/sample/drift/swirl/ripple/breathe`, `voronoi_flow_frame` |
| | `src/ink.rs` (151) | `Ink`, `ink_points`, `ink_weight`, `draw_ink` |
| | `src/modes_sky.rs` (1565) | nebula, solar-system, murmuration, fireflies, meteors, tide, flux, fireworks, hypercube, lanterns |
| | `src/modes_geo.rs` (2187) | eyes_pp, eyes3, phyllotaxis, moire, spiro, spiro-tile, weave, gears, kaleido, contour, stained, circuit |
| | `src/modes_tree.rs` (1646) | fme_pp, trees_pp, forest_pp, fullmetal-eyes, fullmetal-eyes2, fa6, delta |
| | `src/modes_creatures.rs` (231) | `snake_seg`, `snake_walk`, `draw_snakes` |
| `src/tree_draw.rs` (7108) | `src/tree_draw.rs` (387) | head uses, `GrowDir`, `TreeParams`, `BoleExit`, `TrunkNode`, `BranchResult`, `BranchIntent`, `TreeDrawer` trait, `mod tests` |
| | `src/tree_draw/boles.rs` (789) | `TaperKind`, taper fns, `TrunkAlgo` trunk structs, `BoleStyle`/`Bole`/`NoBole`, `FadeDir`, `BushSprite`, `TreeWithTrunk` |
| | `src/tree_draw/bole_pattern.rs` (2294) | `draw_bole_pattern` (the monolithic bole renderer) |
| | `src/tree_draw/species.rs` (1326) | species drawers: SpiralTree..PalmTree |
| | `src/tree_draw/species_exotic.rs` (1784) | species drawers: WideTree..HelixTree |
| | `src/tree_draw/pack.rs` (164) | `grow_tree_by_index`, `PackOpts`/`PackedSlot`/`pack_forest` |
| | `src/tree_draw/scene.rs` (217) | `SceneEl`, `SceneStop`, `SceneOpts`, `scene_walk` |
| `src/sprites.rs` (3673) | `src/sprites.rs` (723) | `Dir`, fret fns, stamp draws (pine/willow/palm/fruit/grow_tree/tset), `draw_mask`, `draw_aztec_diamond` |
| | `src/sprites/trees.rs` (2675) | `TreePen`, `draw_trunk`, pen-growth `grow_*` fns, tip decoration, trunk style table |
| | `src/sprites/flora.rs` (233) | `draw_flower`, `grow_flower_spiral`, `grow_fruit_vine`, `draw_cloud` |
| `src/walker.rs` (2700) | `src/walker.rs` (1852) | walkers (`walk_to_layers`, `path_walk_layers*`, `path_walk_stem`, `soup_walk`, `party_walk`), leaf fill |
| | `src/walker/scenes.rs` (535) | `make_node_scene`, `gen_contour` (private, walker-only), `make_landscape/centerpiece/cluster/negative_space`, `cluster_offset` |
| | `src/walker/paths.rs` (200) | `PathStyle`, `draw_styled_path`, vine/river/double paths |
| | `src/walker/atmosphere.rs` (141) | `Weather`, `apply_atmosphere` |

Largest file after refactor: `src/sprites/trees.rs` at 2675 lines; no file in `src/` exceeds 3000 lines.

## cli.rs arm extraction method

Each `} else if mode == "X" { BODY }` arm of `run()` became `pub(crate) fn cli_<x>(grid: Grid,
width, height, seed, palette: [Color; 5], rng: StdRng, t_anim, term_w, term_h, args: &[String],
mode: &str, theme_name: &str) -> (Grid, bool)` in the mode-group module. The body text is the arm
body verbatim (8-space indent kept); the only edits are bare `        return;` lines (present in the
self-printing swatch/trees3/trees4/trees8/trees9 arms) rewritten to `return (grid, true);` and a
trailing `(grid, false)`. Call sites reassign `grid` and `return` when `true`, preserving the old
skip-epilogue semantics. `grid` and `rng` are moved into the extracted fn (the animated
`draw_*` helpers already took them by value); the epilogue only reads `grid`, which is reassigned
from the returned tuple.

Unchanged: `automata.rs`, `avant.rs`, `biomes.rs`, `borders.rs`, `color.rs`, `content.rs`, `fills.rs`, `layout.rs`, `markdown.rs`, `mondrian.rs`, `render.rs`, `scene.rs`, `types.rs`.

`cli.rs` is one function because the former `main()` body holds per-mode logic with match-local
structs and consts; extracting arms would change item scoping. Left verbatim.

## jq commands used (facts.jsonl, 18852 rows)

```bash
# defs per file
jq -r 'select(.record=="scip_def")|.symbol+ "\t" + .file' facts.jsonl \
  | sed 's/^rust-analyzer cargo ascii-renderer 0\.1\.0 //' > symfile.tsv

# main.rs def inventory (311 symbols)
jq -r 'select(.record=="scip_def" and .file=="src/main.rs")|.symbol
  |sub("^rust-analyzer cargo ascii-renderer 0\\.1\\.0 ";"")' facts.jsonl | sort

# call edges, short names
jq -r 'select(.record=="scip_fn_edge")|.caller+" -> "+.callee' facts.jsonl \
  | sed -e 's/rust-analyzer cargo ascii-renderer 0\.1\.0 //g' > edges.txt

# which main.rs types/helpers each draw_* fn touches (cluster basis)
grep '^draw_' edges.txt | awk '{print $1, $3}' | sort -u
```

Cluster decisions came from `scip_fn_edge`: every `draw_*` calls `pp_*`; per-mode helper types
(`Fly`, `Star`, `Snake`, ...) appear only in their own draw fn; forest modes call `sprites/*`.

## Mechanical move method

Python item splitter over column-0 Rust item starts (fn/struct/enum/const/impl/mod/use/trait/macro_rules),
attaching preceding `#[...]`/`///`/`//` doc lines; item end = line before next item. Items were then
grouped per the table and written with a glob-import prelude (`use crate::<other modules>::*;`).
Items were made `pub(crate)`; struct fields used across modules got `pub(crate)`. `mod tests`
stayed in `main.rs` and `tree_draw.rs`.

## cargo test tail

```
test watershed_seed_42 ... ok
test world2_seed_42 ... ok

test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
```

Determinism check: rendered output for tree, forest, mondrian, snakes, solar-system, weave, fa4,
eyes, labyrinth, world2, watershed, stained, jelly2, metro, koi, party, soup, ca, tiles, mondrian2,
delta, nebula, and the no-mode default are byte-identical to the pre-refactor build (`diff -r` clean).

Mode smoke checks (`cargo run -- 1 <mode>`): tree, forest, ca, scene-walk, stained, party, soup,
watershed, world, terrain OK. `morph` and `md` behave identically to base (morph requires a TTY;
md reads stdin).
