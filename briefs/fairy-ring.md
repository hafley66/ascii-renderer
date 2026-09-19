# fairy-ring: design note

Thesis: a fairy ring as a forest-floor diorama. Above the soil, a ring of
mushrooms advances along a projected ground ellipse. Below the soil, the
colony's true body grows by space colonization and conducts nutrient pulses.
One coarse nutrient field couples both worlds: caps deplete it where they
feed, and its gradients bend later hyphal growth.

## Type signatures

```rust
pub(super) struct FairyRing;
pub(super) static MODE: FairyRing = FairyRing;
const NAME: &str = "fairy-ring";
const KNOBS: usize = 11;

const PARAMS: &[Param] = &[
    param!("PERIOD",   "life cycle seconds", 8.0, 90.0, 36.0, 1.0),
    param!("FRONTIER", "ring radius", 0.3, 1.0, 0.78, 0.02),
    param!("SQUASH",   "ring perspective squash", 0.1, 0.6, 0.28, 0.02),
    param!("COLONY",   "mushrooms on the ring", 4.0, 28.0, 14.0, 1.0),
    param!("WEAVE",    "hyphal curl", 0.0, 1.5, 0.6, 0.05),
    param!("FLOW",     "nutrient pulse speed", 0.0, 3.0, 1.0, 0.05),
    param!("FEED",     "soil richness", 0.2, 2.0, 1.0, 0.05),
    param!("GROUND",   "horizon height", 0.45, 0.75, 0.62, 0.01),
    param!("HUE",      "cap hue deg", 0.0, 360.0, 30.0, 5.0),
    param!("GLOW",     "exposure", 0.5, 1.6, 1.0, 0.05),
    param!("ASPECT",   "cols per row", 0.25, 4.0, 2.0, 0.25),
];

struct Look;                     // resolved knobs, geometry, palette, life phase
struct Seg  { ax, ay, bx, by, ord: f32 }                    // one hyphal step
struct Cap  { x, y, depth, th, sprout, rich: f32 }          // ring placement
fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS])
fn life_phase(time: f32, period: f32) -> f32                 // fract, periodic
fn build_field(field: &mut [f32], fw, fh, look, caps)        // coarse grid
fn field_at(field, fw, fh, x, y, look) -> f32                // bilinear
fn grow_network(look, field, fw, fh)                         // space colonization
fn paint_soil / paint_hyphae / paint_ring / paint_spores
```

## The twelve points

1. Timeline: g = fract(time / PERIOD). Spore fall g in [0, 0.18]; hyphal
   expansion [0.10, 0.50]; sprouting [0.32, 0.88]; release [0.68, 1.0].
   Every state is periodic in g, so the loop is seamless and t=0 is a valid
   spore-fall frame.
2. Stored state: none between frames. Per frame: Look, coarse field
   (about w/4 x h/4), Vec<Seg>, hash-derived caps. Scratch in thread_local.
3. Identity: no rng stream. splitmix hash over (seed, layer, index, slot).
   Mushroom i keeps its identity when COLONY changes. Seed moves palette,
   jitter, sampling, phases.
4. Read/write order: build field (reads ring geometry and cap positions)
   then grow network (reads field) then paint soil, hyphae, ring
   back-to-front, spores.
5. Growth: attractors hashed on a lower half-disc fan (inner 0.15 Rg,
   outer up to 1.15 Rg); tips from the center; each step a tip takes its
   nearest unconsumed attractor within reach, extends one aspect-corrected
   segment, consumes it; steering adds curl noise and a positive nutrient
   gradient term; tips retire on length or when nothing is in reach.
6. Uniqueness: pure hash inputs, fixed iteration order, single-threaded
   growth, row-parallel paint only. Same inputs give the same frame.
7. Bounds: attractors <= 320, segments <= 420, steps per cord <= 22,
   spores <= 64, caps <= 28, growth iterations <= 420. Open tips are
   bounded by the segment budget: every branch spends one segment and
   tips retire at 22 steps, so tips can never outrun STEW.
8. Clipping: every put() bounds-checked; ASPECT maps columns to rows;
   frontier radius scales with w, h and GROUND; clean at tiny sizes.
9. Layer order: soil gradient and litter, hyphae (never above ground),
   mushrooms far to near (occlusion), spores in the sky.
10. Animation invariants: per-element phases from hash, pulses ride raw
    time, growth rides g; adjacent frames share structure, no flicker.
11. Parameter vector: PERIOD, FRONTIER, SQUASH, COLONY, WEAVE, FLOW, FEED,
    GROUND, HUE, GLOW, ASPECT. All continuous, all feeding one pipeline.
    No knob selects a renderer.
12. Feedback: caps write consumption blobs into the field; hyphal steering
    reads depletion gradients and arcs around fed caps; cap growth scales
    with local field value, so rich zones grow larger caps that consume
    more. Emergent: cords avoid the depleted interior, tips cluster at the
    frontier, cap sizes differentiate along the ring.

## Verification plan

- Snapshots: t0 spore fall, colonization g 0.25, mature g 0.55, release
  g 0.90, FEED/COLONY variant, 40x12 clip, FRONTIER 0.68 vs 0.72 pair.
- Sweep: FRONTIER 0.40 0.55 0.70 0.85 1.00 via CLI, plus adjacent mid pair.
- cargo test, layer coverage, knob sweep, scripts/13_e2e.sh.

## Receipts (2026-09-19)

### Verification

- `cargo test`: 524 passed, 1 failed. The failure
  (`morph::iterate_frame_tests::gem_bad_roll6_ansi_regression`) reproduces on
  the pristine tree via `git stash` / `cargo test` / `git stash pop`;
  pre-existing and untouched by this mode.
- Snapshots: 8 unit (`src/modes/snapshots/`) + 2 integration
  (`tests/snapshots/`). Every snapshot was read as text and accepted
  deliberately; no blind accept.
- Sizes: 80x24 canonical, 160x48 shows more cord detail, 40x12 clips cleanly.
  Seeds 7, 42, 1701 give distinct but coherent colonies.
- FRONTIER sweep at t=19.8, seed 42, deep, 80x24: 0.40 (tight ring, dense
  web inside) -> 0.55 -> 0.70 -> 0.85 -> 1.00 (caps reach the side edges,
  still bounded). Adjacent values differ by a few cells; no jumps, no
  clipping, no renderer switch. Snapshot pair 0.68 vs 0.72 matches.

### Performance (release, 800x300, perf/knob_sweep.sh)

- Baseline about 3790 fps; worst knob COLONY=28 at 1811 fps, 0.53 ms/frame
  average, 0.60 ms p50, 1.67 ms p95.
- Hotspots at COLONY=28: soil 43.9 percent, field 34.1 percent, grow 0.8,
  ring 0.3, spores 0.1, cords 0.0. Soil is the per-cell hash scan; field is
  the coarse nutrient pass.

### Layer coverage (perf/layer_coverage.sh 400 120 3 moss fairy-ring)

| mode | layers | calls/frame | attributed | thin |
| --- | ---: | ---: | ---: | --- |
| fairy-ring | 7 | 8.0 | 72.8% | yes |

Attribution is below the 85 percent guideline because every painter is
cheap (0.53 ms/frame at 800x300), so the fixed harness cost per frame
(grid allocation and render dispatch outside mode layers) dominates the
residual. The 7 layers cover all cell-writing work the mode performs; the
timer-existence gates in cargo tests pass. Recorded as-is rather than
inflated.

### E2E (scripts/13_e2e.sh --headless)

- Passed: `pin-inputs`, `seed-search`.
- `workflow --mode fairy-ring`: 8 of 9 checks passed (startup, mode search
  and preview, modeless save, exact animation inputs, terminal-cell motion,
  knob-reaches-render, random-knob diversity, pause stops time); the resume
  gate failed.
- Pre-existing failures reproduced on the pristine tree (stash, rebuild,
  rerun, pop): the workflow resume gate (default mode) and the backpressure
  stall accumulation (`4_test_input_latency.py:169`, relay
  `terminal_wait_us` sum at or under 500 ms). Both are harness-level and
  mode-independent.
- Not run in the full-suite attempt because the suite stops at the first
  failing case: `bad-400x200`, `max-400x200`.
- Worktree note: builds land in the lane `CARGO_TARGET_DIR`, so a gitignored
  `target` symlink satisfies the suite's default binary paths; pass
  `--binary <lane-target>/release/ascii-renderer` when invoking.
