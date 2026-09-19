# heliotrope-flash41

One coupled system: a vine grown by 3D space colonization toward the dawn sun,
lit by that sun through its own accumulated wood, thickened in proportion to the
light it carries, and thinned where the shade it casts leaves it starved. A
turning camera then reveals the depth of the structure the light shaped.

## Type signatures

```rust
struct V3 { x: f32, y: f32, z: f32 }               // world space, y up, ground at y=0
struct Field { data: Vec<f32>, nx, ny, nz }        // shared wood density, BX x BY x BX box
struct Node { pos, dir: V3, parent: u32, light, sugar: f32, leaves: u16,
              thick, along, avg: f32, gen: u8, tip: bool }
struct Stand { nodes: Vec<Node>, tips: Vec<u32>, attr: Vec<V3>, alive: Vec<bool>,
               attr_light: Vec<f32>, next: Vec<u32>, leaf: Vec<bool> }
struct Knobs { lean, crown, fork, crowd, light, prune, leaf, sap, sun, cam, haze, aspect: f32 }
struct Look { k: Knobs, seed, w, h, ppu, cx, cy, az, tilt, sun, sun_seed: V3,
              sun_px, sun_py, sun_r, horizon, time, palette roles ... }
struct Stamp { depth, ax, ay, bx, by, w, light, surv: f32, gen: u8, kind: u8 }
```

```rust
fn grow(seed: u64, k: &Knobs, sun: V3, b: &Budget, field: &mut Field, st: &mut Stand);
fn light_pass(st: &mut Stand, field: &Field, sun: V3, gain: f32);
fn transport(st: &mut Stand);                       // sugar up the lineage, thickness from sugar
fn deposit(st: &Stand, field: &mut Field, optical: bool);
fn solve(l: &Look, field: &mut Field, st: &mut Stand);
fn project(l: &Look, p: V3) -> (f32, f32, f32);     // column, row, camera depth
fn build_stamps(l: &Look, st: &Stand, sc: &mut Scratch);
fn paint_sky / paint_sun / build_shadow / paint_soil / paint_stand / paint_sap / paint_bloom
```

## 1 Timeline and lifetime

Parametric: every frame is a pure function of (seed, params, t). No retained
state; the runtime recreates the grid and RNG per time value, so a stand is grown
and solved inside one frame.

## 2 Stored state

Scratch only, per thread: the density field, the stand, the stamp list, the shadow
buffer, the bloom list. Nothing outlives a frame, nothing is static mutable.

## 3 Identity and RNG

Splitmix64 over (seed, layer, index, slot). No rng stream is consumed. A branch,
leaf, mote or tuft keeps the same identity for every t.

## 4 Read and write order

grow: deposit wood -> light march reads wood -> attractor weights -> next growth
step. Per frame: grow, then light_pass, transport, deposit(optical), light_pass,
transport. Two rounds of light -> mass -> opacity -> light.

## 5 Branching and projection

Attractor pull + phototropic pull + occupancy gradient repulsion + keyed jitter.
Forks spawn when grown length passes a light-scaled spacing; the fork plane rolls
by the golden angle so limbs spread in 3D. Orthographic projection with weak
perspective, camera azimuth turning with t.

## 6 Uniqueness

Seed sets the crown cloud, the sun azimuth, the light azimuth and the camera
offset; the stand is a deterministic consequence.

## 7 Bounds

steps <= 150, tips <= base 26, nodes <= base 900, attractors <= base 150, field
26x22x26, march <= 30 steps, stamps <= nodes + leaves, bloom <= 384 entries,
motes <= 64. All scale with sqrt(area), hard capped.

## 8 Clipping

Every plot goes through a bounds check; ppu is capped by width so a narrow
terminal shrinks the whole scene instead of cropping it.

## 9 Layer order

sky, sun, cast shadow, soil, stand (z-sorted wood and leaves), sap, bloom.

## 10 Animation invariants

Structure depends on the seed only; t drives the sun azimuth and elevation, the
camera azimuth, sap phase and leaf shimmer. t=0 renders the static frame.

## 11 Parameters

lean, crown, fork, crowd, light, prune, leaf, sap, sun, cam, haze, aspect. All
continuous, all fed into the same system: crowd is the occupancy feedback gain,
light is the shading gain, prune is the shade-thinning compensation point.

## 12 Feedback

Growth writes density; density shades later growth through both the attractor
weights and the direction gradient (the vine steers out of its own occupied
space). Light drives leaf sugar; sugar sets branch thickness; thickness sets
opacity; opacity re-shades the leaves. Subtrees whose average light falls under
the compensation point thin and pale out, so the crown hollows where it shades
itself.

## Performance receipt

Layer coverage, 400x120, three reps, theme moss:

| mode | layers | calls/frame | attributed | nested | thin |
| --- | ---: | ---: | ---: | --- | --- |
| heliotrope-flash41 | 8 | 8.0 | 92.5% | no | no |

Frame cost, 200x60 release, in-process (`frame_cost` test): avg 0.238 ms,
worst 0.664 ms over 60 frames.

Knob sweep, 2000x1000, 2 s per run, dt 0.06, under `scripts/5_probe_guard.py`
(max 300 s, 2048 MiB owned, 32 MiB artifacts). Raw output in
`perf/results/heliotrope-flash41.md`.

| knob at max | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 258.6 | 3.87 | 3.84 | 4.17 | 4.33 | 1.00x |
| CROWD=1 (worst) | 247.7 | 4.04 | 4.06 | 4.28 | 4.45 | 1.04x |
| LEAN=2 | 253.9 | 3.94 | 3.96 | 4.14 | 4.27 | 1.02x |
| CROWN=1.4 | 254.0 | 3.94 | 3.91 | 4.17 | 4.31 | 1.02x |
| CAM=1 | 254.5 | 3.93 | 3.96 | 4.14 | 4.28 | 1.02x |
| LEAF=1.5 | 254.8 | 3.92 | 3.95 | 4.12 | 4.18 | 1.02x |
| FORK=1.2 | 254.9 | 3.92 | 3.95 | 4.20 | 9.63 | 1.01x |
| PRUNE=1 | 256.6 | 3.90 | 3.93 | 4.13 | 4.24 | 1.01x |
| SUN=1 | 257.5 | 3.88 | 3.93 | 4.12 | 4.21 | 1.00x |
| LIGHT=3 | 258.8 | 3.86 | 3.84 | 4.12 | 4.29 | 1.00x |
| SAP=3 | 259.8 | 3.85 | 3.83 | 4.11 | 4.27 | 1.00x |
| HAZE=1.2 | 260.0 | 3.85 | 3.81 | 4.10 | 4.22 | 0.99x |
| ASPECT=3 | 303.2 | 3.30 | 3.31 | 3.52 | 4.52 | 0.85x |

Hotspots at the worst knob: sky 49.1%, sun 11.2%, soil 7.1%, grow 6.2%,
stand 1.3%, cast 0.1%, bloom 0.1%, sap 0.0%.

First guarded attempt tripped the breaker on owned RSS (2,105,680 KiB against
the 2,097,152 KiB limit) because the run also compiled the release test targets
inside the guarded tree. Limits were not raised: the sweep was pre-built with
`cargo test --release --no-run` and re-run under the same guard, which passed.
The measured workload is the renderer, not rustc.

The sky pass was the whole frame before tuning: 3.63 ms of 5.54 ms at
2000x1000 (64%). It now fills each row from one precomputed colour and only
walks the sun's halo rectangle for the radial falloff, with the squared
distance replacing a per-cell sqrt: 1.98 ms of 4.04 ms (49%), 258 fps baseline.

## Continuity and emergence observations

Five ordered LEAN values (0.1, 0.5, 0.9, 1.3, 1.7) at 80x24 keep one system:
the crown swings sunward, the trunk shortens under it, and the outer limbs
stretch until the canopy saturates at the reach cap. Two values around the
midpoint (0.95 and 1.05) move 3.8% of the coarse ink blocks; the full sweep
moves 15.0%; a different seed moves 35.0%, so a control deforms the stand while
seeds replace it. Cell-level churn is high near any value (about 5% at a 0.01
step) because growth amplifies a direction change into a different set of
attractor assignments; the coarse structure is what stays.

Emergence: growth writes the density field, the field's gradient bends later
shoots out of written space, and the sun marched through the same field decides
which attractors are still worth reaching for, so a shaded attractor loses its
claim. After growth, light sets leaf sugar, sugar sets limb thickness, thickness
sets optical mass, and the second light pass re-shades the leaves through the
wood the first pass made thick. Subtrees whose average light falls under the
compensation point keep their place but lose ink, so the crown hollows and
pales on the side the sun is not on. The skeleton itself is time invariant
(`skeleton_is_time_invariant`): only the light, the flow and the camera turn.

## Validation receipt

- `cargo build` clean; `cargo test` on this worktree reports 525 passed, 1 failed
  (`morph::iterate_frame_tests::gem_bad_roll6_ansi_regression`) and the
  integration target reports 199 passed, 2 failed (`nightglass_seed_42`,
  `nightglass_rain_running_t9`). All three fail identically with this mode
  stashed, so they are pre-existing in this worktree and unrelated.
- Heliotrope's own tests: 7 snapshots, skeleton time invariance, nearby-lean
  continuity, seed sensitivity, time continuity, extreme parameters at 24x8 and
  2x2, and the release frame cost, all passing. No `.snap.new` left anywhere in
  `src/snapshots`, `src/modes/snapshots` or `tests/snapshots` from this work.
- Layer coverage 400x120 x3: 8 layers, 8.0 calls/frame, 92.5% attributed, not
  nested, not thin.
- Knob sweep 2000x1000 under the probe guard: baseline 258.6 fps, worst knob
  CROWD=1 at 247.7 fps (1.04x). See the performance section for the full table.

### Terminal E2E

`scripts/13_e2e.sh --headless` reaches the real demo over a PTY. Result:
`workflow` failed at its ninth check, `resume: no matching application evidence
within 2s`; `seed-search`, `bad-400x200`, `max-400x200` and `backpressure` are
`not_run` because the suite stops at the first failure. `pin-inputs` is not part
of the headless case list.

That failure is not this mode: the identical failure reproduces with the
heliotrope-flash41 registration removed from the generated registry (`resume` still
times out at the same check) and with the default mode at the base commit.
The checks that do run all pass, and running the same case against this mode
(`--case workflow --mode heliotrope-flash41`) validates the live path end to end:

| check | status | evidence |
| --- | --- | --- |
| actual-demo-startup | passed | 160x40 PTY |
| mode-search-and-preview | passed | heliotrope-flash41 found and previewed |
| modeless-save | passed | preset written |
| exact-animation-inputs | passed | frame 1, 23010 bytes, 4914 cells changed |
| terminal-cell-motion | passed | cell hash changes between frames |
| knob-reaches-render | passed | 2 ms from keypress to frame, all 12 knobs present |
| random-knob-diversity | passed | roll changes knobs |
| pause-stops-animation-time | passed | time frozen at 3.8e-05 |
| resume | failed | pre-existing timeout, reproduced without this mode |

Two harness notes, neither of which changed a limit:
the first `13_e2e.sh` attempt tripped its own watchdog in the release build
step (owned RSS 1,062,320 KiB against 1,048,576 KiB) because rustc ran inside
the guarded window; pre-building the binary and example let the same guard pass
and reach the cases. `12_test_e2e.py` also resolves the helper example at
`ROOT/target/release/examples`, while this lane sets `CARGO_TARGET_DIR` to a
lane directory, so a `target` symlink to the lane target dir was needed for the
suite to find the freshly built binaries.
