# Cleanup plan: dirty main tree dispositions + split pricing

Auditor-facing receipts. Companion plain-language doc: `cleanup-plan.visual.human.unga.md`.

## TOC

1. Scope and method
2. Laws cited
3. Disposition table (every inventory row)
4. Snapshots directory resolution
5. src/main.rs split pricing
6. Out-of-scope observations
7. Validation receipts

## 1. Scope and method

Inspected the dirty main tree at `/Users/chrishafley/projects/ascii-renderer` read-only. All diffs quoted against `HEAD` = `9fdb45e`. Worktree base for the docs is `f83ddea`. No code changes made anywhere. Deliverables are the two `docs/cleanup-plan.*` files only.

Three layers read separately:
- unstaged (`git diff`)
- staged (`git diff --cached`)
- untracked (`git status --short`)

`MM` files carry both a staged and an unstaged layer; both were diffed independently.

## 2. Laws cited

From `AGENTS.md` and `CLAUDE.md` (tracked copies at `HEAD`):

| Law | Location |
|---|---|
| "Never remove or break existing modes. Only add." | `AGENTS.md` Ground rules; `CLAUDE.md` |
| "Every mode gets at least one snapshot test with a fixed seed" | `AGENTS.md` Testing rules |
| "When adding a new mode: add a snapshot test before committing" | `AGENTS.md` Testing rules |
| "Never `cargo insta accept` blindly" | `AGENTS.md` Testing rules |
| "Never propose deleting a snapshot" | brief law |
| "Snapshot files live in `src/snapshots/` (insta default)" | `AGENTS.md` Testing rules (stale, see 4) |
| "Commit at each milestone for rewind points" | `AGENTS.md` Ground rules |
| Numbered-file / semantic-module convention; `main.rs` is CLI dispatch + mode wiring | `AGENTS.md` Architecture |

## 3. Disposition table (every inventory row)

| # | Item | Layer | Disposition | Reason |
|---|---|---|---|---|
| 1 | `M AGENTS.md` | unstaged | **commit** | Adds `fa6, fullmetal-alchemist6` and the nine avant modes to the mode list, plus the `avant.rs` architecture line. Mirrors the code landing in the same dirty tree. Docs must stay in sync with the mode set (AGENTS.md is the mode registry doc). Additive, matches the "only add" rule. |
| 2 | `M CLAUDE.md` | unstaged | **commit** | Same additive mode-list change + `avant.rs` line. Note `CLAUDE.md` and `AGENTS.md` already diverge in mode coverage (CLAUDE.md lists `phyllotaxis, moire, nebula, delta, stained, eyes++, forest++` that AGENTS.md lacks). This diff is additive and safe; the pre-existing divergence is separate and out of scope. |
| 3 | `MM src/main.rs` | staged | **commit** | Staged layer (549 lines added) is `draw_fa6` + `fa6` dispatch wiring (`src/main.rs:16109`, arms at `:182`, `:622`, `:11640`, `:17617`). |
| 3b | `MM src/main.rs` | unstaged | **commit** | Unstaged layer (614 lines) is `mod avant` (`:18`), `use avant::*` (`:40`), three native-T draw fns `draw_murmuration`/`draw_lanterns`/`draw_tide` (`:16548`, `:16672`, `:16794`) with dispatch arms, and their in-module determinism tests. Independent of the staged `fa6` work. Recommend committing staged first, then unstaged, as two commits. |
| 4 | `M tests/snapshot_modes.rs` | unstaged only (not MM despite brief) | **commit** | Adds 9 snapshot tests (`fa6`, `rhizome`, `effigy`, `dendrite`, `totem`, `chimera`, `murmuration`, `lanterns`, `tide`), one per new mode at fixed seed 42. Satisfies the "every mode gets a snapshot test" rule. |
| 5 | `M chat_log/LATEST.md` | unstaged | **commit** | `LATEST.md` is a one-line pointer to the newest dated entry. This diff repoints it to `20260626.0...`. Every prior entry and the pointer are committed (see `git ls-tree HEAD chat_log/`). Commit the pointer with its target entry (row 10). |
| 6 | `?? src/avant.rs` | untracked | **commit** | 538 lines. Wired in: `mod avant;` at `src/main.rs:18`, `use avant::*;` at `src/main.rs:40`. Contains the algorithmic tree/face modes (`draw_rhizome`, `draw_effigy`, `draw_dendrite`, `draw_totem`, `draw_chimera`, `avant.rs:160,257,352,393,477`). Additive, complies with "only add". |
| 7 | `?? tests/snapshots/snapshot_modes__*.snap` | untracked | **commit** | 9 files, not 10 (see 4). Integration snapshots for the new modes. `tests/snapshots` is the correct home for them (see section 4). Never delete a snapshot, so commit them with the tests that generate them. |
| 8 | `?? chat_log/20260626.0.snakes-mode-knobs-randomize-persist.md` | untracked | **commit** | 84-line dated session entry matching the committed `chat_log/` convention (`2026MMDD.N.<topic>.md`). It summarizes the snakes/circuit work that is already committed (commits `57fc472`, `d1ec32b`, `f83ddea` all exist; `draw_snakes`/`draw_circuit` present at `HEAD`). Commit so the pointer in row 5 resolves to a tracked file, keeping the log self-consistent. |
| 9 | root design docs (see below) | tracked | **move-under-docs** (price only, do not execute) | 11 files at repo root, all tracked, all June 11 2026, all design/reference material not wired into the build. Move to `docs/` to match the existing `docs/flower_energy_synthesis.md` precedent. See section 5 pricing. |
| 10 | `src/main.rs` ~19k lines | tracked | **commit** (the split itself: later arc) | The dirty-tree change lands as-is. Splitting is a separate later arc. Pricing in section 6. |

### Root design docs (row 9)

| File | Lines | Kind |
|---|---|---|
| `CHARACTER_CONNECTIONS_INDEX.md` | 3409 | design index, references `ORGANIC_CHAR_BUILDER.rs` |
| `CHARACTER_CONNECTION_DESIGN.md` | 10552 | design |
| `CHAR_CHEAT_SHEET.txt` | 7532 | reference |
| `CHAR_CONNECTIONS_README.md` | 10325 | reference |
| `ORGANIC_CHAR_BUILDER.rs` | 7739 | reference Rust (not compiled; cited by the md files) |
| `ORGANIC_CHAR_CONNECTIONS.md` | 15138 | design |
| `QUICK_CHAR_MATRIX.md` | 9913 | reference |
| `RESEARCH.md` | 29290 | research |
| `TREE_PATTERN_EXAMPLES.md` | 12449 | reference |
| `VISUAL_CONNECTION_DIAGRAMS.md` | 10437 | reference |
| `char_exits_reference.rs` | 12200 | reference Rust (not compiled) |

All are tracked and referenced only by each other and by `CHARACTER_CONNECTIONS_INDEX.md`; no `src/` or build file imports the two `.rs` files. `docs/` already holds one design doc (`docs/flower_energy_synthesis.md`), so moving these under `docs/` is consistent. Pricing: `git mv` x11 (no content change), plus updating the relative cross-references inside the md files. The `.rs` files are reference snippets, not build inputs; they move as-is. Do not convert to code. This is priced, not executed.

## 4. Snapshots directory resolution

Two snapshot directories exist and both are canonical for their own test file:

| Directory | Test file | Snapshot prefix | Tracked count |
|---|---|---|---|
| `src/snapshots/` | unit tests in `src/main.rs` `mod tests` (`src/main.rs:18207`) | `ascii_renderer__tests__*` | 35 |
| `tests/snapshots/` | integration tests `tests/snapshot_modes.rs` | `snapshot_modes__*` | 83 |

The 9 untracked snapshots belong to `tests/snapshot_modes.rs`, so `tests/snapshots/` is their correct home. The AGENTS.md line "Snapshot files live in `src/snapshots/` (insta default)" is stale and wrong for integration tests: insta places snapshots next to the test file, which for `tests/snapshot_modes.rs` is `tests/snapshots/`. That doc line should be corrected to name both directories. No snapshots are deleted.

## 5. src/main.rs split pricing (later arc, do not execute)

`src/main.rs` = 18,991 lines. It already compiles a dozen sibling modules (`automata, biomes, borders, color, content, fills, layout, markdown, mondrian, render, scene, sprites, tree_draw, types, walker, avant`) and reaches into them via `use X::*` (`src/main.rs:26-40`). The split follows the same pattern: extract the draw families and infrastructure still sitting in `main.rs` into sibling modules, leaving a thin dispatch in `main.rs`.

Measured sections (line ranges, `src/main.rs`):

| Proposed module | Source lines | Lines | Contents (anchors) |
|---|---|---|---|
| `mode_registry.rs` | 53-204 | 152 | `Param`, `AnimKind`, `ModeSpec`, `ModeForm`, `MODE_FORMS` |
| `options.rs` | 205-368 | 164 | `param_f32`, `options_path`, `load/save_options`, `pvals_for`, `rand_knob`, `effective_pvals`, randomize + persist |
| `demo.rs` | 369-894 | 526 | `demo_filter_modes`, `demo_pick_mode`, `draw_options_pane`, `run_demo` |
| `dispatch.rs` (or keep in main) | 895-11924 | 11030 | `main()` arg parse, help text, per-mode `else if` dispatch. Largest item; the draw bodies move out, dispatch becomes table-driven |
| `draw_pp.rs` | 11925-12473 | 549 | `pp_*` primitives + `draw_eyes_pp`/`draw_fme_pp`/`draw_trees_pp`/`draw_forest_pp` |
| `draw_native.rs` | 12474-12844 | 371 | `draw_circuit`, `snake_seg`, `snake_walk`, `draw_snakes` |
| `draw_fx.rs` | 12845-12973 | 129 | `draw_phyllotaxis`, `draw_moire`, `draw_nebula` |
| `draw_scene.rs` | 12974-17151 | 4178 | `draw_solar_system` .. `draw_stained`, the themed-args family |
| `serialize.rs` | 17152-17236 | 85 | `emit_grid`, color code encode/decode, `serialize/parse_grid`, `fit_grid` |
| `morph.rs` | 17237-17666 | 430 | `MorphState`, `iterate_grid`, `render_frame`, `render_frame_t` |
| `warp.rs` | 17667-17930 | 264 | `warp_*`, `voronoi_flow_frame` |
| `runner.rs` | 17931-18205 | 275 | `run_morph` |
| `tests` | 18206-18991 | 786 | in-module unit tests (move with their subjects) |

Estimate: extracting these ~8,600 lines of draw infrastructure and registry into ~12 sibling modules. `main.rs` shrinks to roughly the dispatch core (help text, arg parse, mode match). Each extracted module is additive, so the "only add, never break modes" rule holds. Risk is mechanical: the draw functions share helpers (`pp_*`, `param_f32`, `Color`, `Grid`, `StdRng`) that must become `pub(crate)`. This is a later arc; the dirty tree ships as-is first.

## 6. Out-of-scope observations

Not in the brief inventory, surfaced by `git status --short`:

| Item | Disposition | Reason |
|---|---|---|
| `?? .boop-worktrees/` | **gitignore** | boop worktree staging dir; tooling, not source |
| `?? lanes/` | **gitignore** | agent lane scratch; tooling |

Neither is source or product. `.gitignore` currently holds only `/target`. Adding these two keeps `git status` clean without touching content.

## 7. Validation receipts

In the worktree at `f83ddea`:

| Check | Result |
|---|---|
| `cargo build` | pass, `Finished dev profile` (no code changed; base tree sound) |
| `git status --short` | exactly the two new `docs/cleanup-plan.*` files |
| disposition-token grep | passes (count in file, see below) |

Disposition-token counts in this file:
- `commit`: 9 occurrences
- `move-under-docs`: 1
- `gitignore`: 2 (section 6)
- `keep-as-is` / `discard`: 0 (no row calls for either)
