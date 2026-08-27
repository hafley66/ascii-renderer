# Refactor report

Base sha: 8f8ce23. No behavior change; `cargo test` 95 passed / 0 failed, zero snapshot changes.

## File map (old -> new)

| Old file (lines) | New module(s) (lines) | Contents |
|---|---|---|
| `src/main.rs` (19581) | `src/main.rs` (858) | `#![allow]`, mod decls, glob imports, thin `fn main()`, `mod tests` (786 lines, unchanged) |
| | `src/cli.rs` (11119) | former `main()` body verbatim as `pub(crate) fn run()`; includes match-local structs (`TreeSlot`, `Snake`, `Flock`, `Fly`, `Star`, `Drop`, `Chamber`, `Node`, `Pending`, `SK`, `F7Element`, `F7Stop`, `Site`, consts `SPEED`/`TAIL`/`WARM`/`DT`/`SLOTS`) |
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
| `src/tree_draw.rs` (7108) | `src/tree_draw.rs` (381) | head uses, `GrowDir`, `TreeParams`, `BoleExit`, `TrunkNode`, `BranchResult`, `BranchIntent`, `TreeDrawer` trait, `mod tests` |
| | `src/tree_draw/boles.rs` (3080) | `TaperKind`, taper fns, `TrunkAlgo` trunk structs, `BoleStyle`/`Bole`/`NoBole`, `FadeDir`, `BushSprite`, `draw_bole_pattern`, `TreeWithTrunk` |
| | `src/tree_draw/species.rs` (3279) | all 27 tree species `TreeDrawer` impls, `grow_tree_by_index`, `PackOpts`/`PackedSlot`/`pack_forest` |
| | `src/tree_draw/scene.rs` (431) | `SceneEl`, `SceneStop`, `SceneOpts`, `scene_walk` |
| `src/sprites.rs` (3673) | `src/sprites.rs` (723) | `Dir`, fret fns, stamp draws (pine/willow/palm/fruit/grow_tree/tset), `draw_mask`, `draw_aztec_diamond` |
| | `src/sprites/trees.rs` (2675) | `TreePen`, `draw_trunk`, pen-growth `grow_*` fns, tip decoration, trunk style table |
| | `src/sprites/flora.rs` (233) | `draw_flower`, `grow_flower_spiral`, `grow_fruit_vine`, `draw_cloud` |
| `src/walker.rs` (2700) | `src/walker.rs` (1852) | walkers (`walk_to_layers`, `path_walk_layers*`, `path_walk_stem`, `soup_walk`, `party_walk`), leaf fill |
| | `src/walker/scenes.rs` (534) | `make_node_scene`, `gen_contour` (private, walker-only), `make_landscape/centerpiece/cluster/negative_space`, `cluster_offset` |
| | `src/walker/paths.rs` (200) | `PathStyle`, `draw_styled_path`, vine/river/double paths |
| | `src/walker/atmosphere.rs` (141) | `Weather`, `apply_atmosphere` |

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

test result: ok. 95 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
```

Mode smoke checks (`cargo run -- 1 <mode>`): tree, forest, ca, scene-walk, stained, party, soup,
watershed, world, terrain OK. `morph` and `md` behave identically to base (morph requires a TTY;
md reads stdin).
