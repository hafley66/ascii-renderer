---
name: add-mode
description: Add a rendering mode to ascii-renderer with current module placement, CLI and demo wiring, tunable parameters, deterministic snapshots, and animation integration when requested. Use for new modes or mode aliases. Do not use for a sprite that has no standalone mode.
---

# Add Mode

Add one mode without changing the output of existing modes.

Read [references/0_mode-lifecycle.md](references/0_mode-lifecycle.md) before editing. It records the current dispatch, demo, animation, parameter, and snapshot seams.

## Shape the change

Before writing the body, state the proposed Rust signatures and place pseudocode comments inside the planned body. Then record:

- Instance timeline: grid allocation, RNG seeding, handler call, render, and repeated animation frames.
- Storage: CLI arguments, `ASCII_T`, `ASCII_P_<KEY>`, persisted option values, and where each value is read.
- Uniqueness: canonical mode name, aliases, parameter keys within the mode, and exactly one CLI dispatch path.

Place visual algorithms with the closest mode family. Place the `cli_<mode>` handler with the closest CLI family. Preserve the colocated signature and state-management style. If a new Rust module is required, choose its author-driven numeric prefix from its dependencies and expose it with `#[path = "N_name.rs"] mod name;`.

## Implement

1. Run the existing tests before changing a mode family.
2. Search the canonical name and aliases across `src/`, `tests/`, and `AGENTS.md`.
3. Implement the renderer or composition using the existing grid, palette, RNG, scene, walker, sprite, or tree APIs.
4. Add one `cli_<mode>` handler and one branch in `src/cli.rs`.
5. Add the canonical name to the demo list in `src/opts.rs`.
6. If the mode has live knobs, add one `ModeForm` in `src/registry.rs` and read matching keys through `param_f32` in the renderer.
7. If the mode has native time motion, use the `add-animation` workflow and add its in-process path to `iterate_grid` in `src/morph.rs`.
8. Add a fixed-seed integration snapshot in `tests/snapshot_modes.rs`. Add focused unit tests for invariants that a visual snapshot cannot express.
9. Run `scripts/0_check_mode.sh <mode>` or add `--animated` for a native animation.
10. Run the focused test, inspect pending snapshot output, then run `cargo test`. Accept a snapshot only after visual inspection.

## Constraints

- Keep every existing mode and alias working.
- Treat `(dimensions, seed, palette/theme, arguments, parameters, time)` as the render inputs.
- Recreate randomness deterministically from the supplied seed. A frame must not depend on how many frames preceded it.
- Bound all walks, recursion, particle counts, and writes by the grid dimensions.
- Preserve display width for wide Unicode cells through the existing grid and render helpers.
- Expose visual constants that materially change the result as CLI arguments or `ModeForm` parameters.
