# shamsa design note (checkpoint 1)

Iran, Safavid dome interiors (Isfahan). Pattern tradition: girih tessellation
(radial rosette construction with interlaced strapwork) on a muqarnas-vaulted
dome, with arabesque fronds growing through the lattice. Mode name: `shamsa`.

## Type signatures

```rust
pub(super) struct Shamsa;
impl Mode for Shamsa { fn render(&self, frame: &mut ModeFrame<'_>) }

struct Look { /* resolved per-frame geometry, palette, node rings, lantern */ }
struct NodeGlow { x: f32, y: f32, b: f32 }          // 21 fixed screen anchors
#[derive(Clone, Copy, Default)]
struct Sample { u, theta, n, light, strap, sdir: f32,
                over: u8, tier: u8, tier_cell: u8, owner: u8, rho: f32 }

fn u_of_r(n: f32, depth: f32) -> f32    // flat disc -> hemisphere projection
fn n_of_u(u: f32, depth: f32) -> f32    // inverse projection for node placement
fn sample_field(field: &mut [Sample], w, h, look)
fn paint_vault(grid, field, w, h, look) // muqarnas tiers, dome shading, wall
fn paint_straps(grid, field, w, h, look)
fn paint_boss(grid, w, h, look)
fn paint_glow(grid, w, h, look)
fn draw(frame: &mut ModeFrame<'_>, p: &[f32; 10])
```

## Design points

1. Timeline: parametric animation. Every frame is recomputed from
   `(seed, t, params)`; no state survives frames. t=0 is the static composition.
2. Stored state: two thread-local scratch buffers (field samples, later the
   vine ink field), sized to the grid. No globals, no env.
3. Identity: splitmix hash over `(seed, layer, index, slot)`; `frame.rng`
   stream untouched. Nodes, tiers, niches, vines have stable hash identities.
4. Reads/writes: one field pass reads Look and writes Sample per cell; paint
   passes read field + write grid, strictly layered (see 9).
5. Rewrite sequence: cell -> (u, theta) on the dome -> nearest of 3 candidate
   rosette frames (center, ring A, ring B) -> analytic girih star distance
   (spokes blended into kinked parallel chords) -> min with tier bands and
   radial connectors -> strap mask -> over/under parity -> light -> paint.
6. Uniqueness: each cell claims at most one owner rosette (min normalized
   distance); straps claim the cell only inside their half-width.
7. Bounds: per cell O(1) with 3 candidate evals; vine segments (checkpoint 3)
   capped at 240 branches x 44 segments; glow halos capped at 21 nodes x box.
8. Clipping: everything derived from (cx, cy, R) with knob aspect; all writes
   index-checked by row/col iteration; tested at 30x12 and smaller.
9. Layers: field, vault, straps, boss, glow (vines slot before glow). Later
   layers only add or darken; strap gaps restore vault by skipping writes.
10. Animation invariants: spin and lantern azimuth are the only global angle
    sources; all jitter from stable hashes; t=0 byte-identical to static.
11. Parameter vector (continuous, one system): FOLD star order, DEPTH dome
    projection, SPIN rad/s, FLOW lantern sweep rad/s, LANTERN height, VINE
    growth budget, STRAP width, BLOOM glow, HUE palette rotation, ASPECT.
12. Feedback: vines (checkpoint 3) steer toward light and away from an ink
    field they deposit; ink then stains the vault shading; straps occlude the
    light field that gates both strap brightness and vine steering.

## Checkpoint 3: arabesque vines and motion

13. Growth: fronds root at ring nodes (ring B gated by the rb detail factor),
    each root spawning 1-3 fronds, budget gated by VINE (0 = bare dome).
    Hard bounds: 240 fronds, 44 segments each.
14. Steering: three candidates (straight, +/-0.45 rad plus an oscillating
    curl) scored by lantern light minus a squared ink penalty; scores below
    the viability floor end the frond. Squared (sublinear-near-zero) penalty
    lets a frond run its own fresh path while dense ink repels.
15. Feedback: each step deposits ink (core 0.55, ring 0.18, cap 1.5) into a
    per-frame thread-local field cleared each draw, so later fronds inherit
    earlier occupancy deterministically; no state crosses frames.
16. Motion: fronds paint a root-to-tip pulse (exp wave over rem_euclid of
    time and arc position); glow dust breathes with a 0.78-1.00 factor at
    1.7 rad/s; both are pure functions of t, so t=0 stays byte-stable.
17. Palette: vines lerp vine_a (dim verdigris-gold, hue+off) to vine_b
    (lit) by the pulse; glyphs ( ) ' , are vine-exclusive so presence is
    testable.

## Checkpoint 4: snapshots and refinement

18. Refinements: wall speckle blocks tightened from 7x3 to 5x2 with a single
    glyph pick (calmer negative space); straps fade below u=0.96 so the dome
    edge reads as one continuous silhouette instead of strap fragments.
19. Snapshot set (all visually inspected before acceptance): canonical t0,
    t6, t14 (lantern sweeps right then down-left), 160x50 (ring B eyes,
    denser interlace, tier seams), 30x12 clip, VINE 0 and 1 variants,
    LANTERN 0.40/0.45 nearby pair (crown block shifts, continuity held).
20. Integration entry appended at the end of tests/snapshot_modes.rs; the
    CLI render with theme moss matches the module canonical byte for byte.

## Checkpoint 5: performance and E2E receipt

Layer coverage (perf/layer_coverage.sh 400 120 3 moss shamsa, release):
6 layers (field, vault, straps, vines, boss, glow), 6.0 calls/frame,
98.9% of the render call attributed; 0 thin, 0 nested, 0 without timers.

Knob sweep (perf/knob_sweep.sh shamsa 2000 1000 2, 2M cells, release):
baseline 5.9 fps / 168.32 ms avg; every knob at max stays within noise
(0.91x-1.00x: FLOW 163.95 ms worst, VINE=1 160.81 ms, STRAP=0.3 162.59 ms);
ASPECT=4 halves the frame (0.48x) because the dome covers fewer columns.
Hotspots at the worst knob: field 95.4% (per-cell girih projection, by
design), vault 2.5%, straps 1.1%, boss 0.2%, vines 0.1%, glow 0.0%.
Frame budget: 200x60 release frame_cost asserts avg < 6 ms and passes.

Continuity: LANTERN swept 0.10/0.20/0.30/0.40/0.50 at 80x24; the lit crown
migrates smoothly with no jumps, matching the accepted lantern40/45
nearby-pair snapshots.

E2E (scripts/13_e2e.sh, probe guard 900 s / 1024 MiB / 32 MiB artifacts,
safety suites passed on every run; binary sha256 ddf193b1..., commit
54b27c8): six cases, each executed, all artifacts preserved under
perf/results/e2e-*/; no limits were raised.
- workflow: passed. seed-search: passed.
- pin-inputs: passed (first attempt tripped on foreground focus loss,
  foreground=instant; retry with identical limits passed).
- backpressure: failed twice, same assertion (4_test_input_latency.py:
  sum terminal_wait_us > 500_000). Shared demo relay, fixture modes only
  (gem-aetherium-2/party); no shamsa code in the path. Guard completed
  with no breaker; both runs preserved.
- bad-400x200: failed, terminal RPC key:animate exceeded 750 ms (mode
  party, relay terminal_wait_us=0). Reproduced identically with a binary
  built at cp1 commit 6f1f801 before the strapwork and vine work, so the
  trip predates this lane's changes; environmental (tmux-nested iTerm,
  taskpolicy nice 19).
- max-400x200: failed, one focus trip (foreground=Google Chrome), then the
  same key:animate 750 ms trip as bad-400x200.
Unexecuted cases: none; every suite case was attempted, failures preserved
with exact inputs (report.json, command.json, guard.ndjson, rpc/terminal
logs) for upstream triage.
