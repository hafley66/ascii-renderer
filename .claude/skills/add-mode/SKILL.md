---
name: add-mode
description: Scaffold a new rendering mode for ascii-renderer. Trigger on "new mode", "add mode", "make a X mode", or any request to create a new visual mode.
---

# Add Mode

Scaffold a new rendering mode in ascii-renderer with snapshot test coverage.
Reference implementation: `src/arboretum.rs` (new-module mode with 10-knob genome, 11 registry params, native lifecycle+sway animation, both snapshot styles).

## Wiring checklist (6 touchpoints)

1. **`src/cli.rs` -- dispatch arm.** Append `} else if mode == "NEW_NAME"` as the LAST arm, directly before the final `} else {` / `cli_default` fallback. Never splice into the middle of the chain (dispatch lives here, not main.rs; main.rs is `cli::run()` + tests):
```rust
} else if mode == "arboretum" {
    let (g, done) = cli_arboretum(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name);
    grid = g;
    if done {
        return;
    }
}
```
Handler signature: `cli_<name>(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, args, mode, theme_name) -> (Grid, bool)`. Handlers live in the `cli_*.rs` family (cli_basic, cli_forest, cli_scenes, cli_city, cli_fa, cli_catalog) or your own module. If the handler is in a new module, add `use crate::<mod>::cli_<fn>;` -- cli.rs only glob-imports the six existing cli_* modules.

2. **`src/main.rs` -- module decl.** Append `mod <name>;` after the last `mod` line. No `use <name>::*;` (cli.rs and morph.rs path-import what they need; glob re-exports collide across modules that share type names).

3. **`src/opts.rs` `run_demo()`.** Append the mode string as the last entry of `all_modes` (before `];`) or it won't show in the demo picker.

4. **`src/registry.rs` `MODE_FORMS`.** Append one `ModeForm` row as the last element (before the closing `];`). Params are read at render time via `param_f32(KEY, default)` from env `ASCII_P_<KEY>`; keys must be unique within the form.

5. **`tests/snapshot_modes.rs`.** Append at end of file. CLI snapshot: `insta::assert_snapshot!(render(&["42", "NEW_NAME", "theme"]))`. In-module `#[cfg(test)]` tests may call draw fns directly (pattern in `src/tree_draw.rs`, `src/arboretum.rs`).

6. **`CLAUDE.md`.** Append the mode name at the end of the modes list.

## Native animation

If the mode does not consume `t_anim`, pressing `a` in demo falls back to `warp_wind` (the `.unwrap_or_else` fallback in morph.rs `iterate_grid`). For native animation:
- Consume the `t` param in the draw fn, and add an arm to `iterate_grid` in `src/morph.rs` calling the draw fn in-process. Append it as the last arm before `_ => false`.
- t=0 MUST render the static frame byte-identical (snapshot stability): gate animation on `t > 0.0`.
- Derive per-item animation phases/cycles from a SIDE rng seeded from a hash of (seed, layer, index) so the main rng stream and static render are untouched.
- Verified pattern: `draw_arboretum` / `grow_tree` in `src/arboretum.rs`.

## Snapshots

`cargo insta` may not be installed. Accept after visual inspection: `mv *.snap.new *.snap`. Snapshot files land in both `src/snapshots/` (in-module tests) and `tests/snapshots/` (integration tests).

## Rules

- Every list append goes at the END (dispatch chain, mod list, all_modes, MODE_FORMS, morph match, tests, CLAUDE.md). Fixed-index or alphabetical splicing is banned.

- Never modify existing modes
- Use fixed seed for deterministic output
- Expose tuning knobs: positional CLI args (`args.get(4).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT)`) and/or registry form params -- don't hardcode
- Keep the mode self-contained in its handler (call into walker/scene/sprites as needed)
- Background fill patterns: use `fill_truchet`, `fill_noise`, or `Cell::blank()`
- Sprites: `draw_tree`, `draw_flower`, `draw_fruit`, `draw_mask` from sprites.rs
- Composition: `render_scene` with `Layer` and `FillGen` from scene.rs

## Gotchas

- `hsl_to_rgb` takes f64; knob values are f32 -- cast.
- `Bole::draw` is a `BoleStyle` trait method, not inherent -- import the trait.
- Stop hook comment-prod: max 2 consecutive comment lines in new code (`///` doc lines count).
- Glyph safety: reuse chars from existing pools (`src/sprites.rs`, `src/tree_draw/species*.rs`) -- all width-1 verified. New exotic glyphs risk unicode-width assertion failures in main.rs display-width tests.
