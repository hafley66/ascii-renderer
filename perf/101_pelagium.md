# Pelagium

Experiment: ART_INPUT_NUMBER=p01, ART_RUN_NAME=p01__crazy-astra-high.
Owned mode: `src/modes/_101_p01_crazy_astra_high.rs`; registry name `pelagium`.

## Signatures before implementation

```rust
fn solve(seed: u64, time: f32, p: &[f32; 5]) -> Mantle;
fn surface(m: &Mantle, u: f32, v: f32, p: &[f32; 5]) -> Vertex;
fn project(v: Vertex, width: usize, height: usize, time: f32) -> Vertex;
fn draw(frame: &mut ModeFrame<'_>, p: &[f32; 5]);
```

1. Lifetime: the existing CLI/native player allocates the grid, resets its RNG,
   and borrows inputs for one frame. Reconstruct the coupled system from seed and
   absolute time, project, shade, discard all frame scratch. No history dependency.
2. Storage: five resolved floats; a periodic 128-node mantle of energy and radial
   displacement; projected mesh vertices; a clipped depth buffer. No retained
   simulation, process environment writes, or files inside rendering.
3. Identity: seed-derived harmonic phases and amplitudes; ring sample index and
   filament anchor angle are stable identities. No time-dependent RNG consumption.
4. Reads/writes: initialize energy and displacement, perform 32 double-buffered
   coupled relaxation steps, read the converged mantle into mesh and filaments,
   depth-test fragments into the borrowed grid.
5. Sequence: periodic energy conduction, strain-dependent conductivity, elastic
   displacement driven by energy, pleated toroidal surface, tilted projection,
   membrane engraving and descending material streamlines.
6. Uniqueness: one canonical mode, one MODE static, five distinct parameter keys,
   exactly the generated dispatch; periodic endpoints refer to the same node.
7. Bounds: 128 nodes, 32 updates, at most 256x64 surface samples, 32,768 triangles,
   12 filaments x 128 segments and two sutures x 256 segments; no recursion,
   particles, stored frames, or branching. The endpoint-inclusive mesh has at
   most 257x65 = 16,705 vertices.
   Raster work is clipped to the visible grid; scratch O(W*H + 16,705).
   The depth buffer costs 4*W*H bytes and mesh vertices cost at most 467,740
   bytes, about 15.7 MiB total scratch at 2000x2000. Triangle work is the sum
   of clipped bounding-box areas, bounded by 32,768*W*H. Line subdivision
   has a hard 4,096-step ceiling per segment. No knob increases these bounds.
8. Projection: two columns per row; common scale fits the full vertical silhouette
   and horizontal crown, with an additional 1.25 lateral organism stretch.
   Empty and tiny grids return or clip through one write path.
9. Layers: dark background, opaque shaded mantle with depth occlusion, depth-tested
   ribs and filaments. Fine engraving follows the same surface coordinates.
10. Motion: forcing phases travel through fixed seed harmonics. Projection precesses
    slowly and energy pulses travel down the same filaments. Equal explicit inputs
    reproduce cells and colors, independent of playback order.
11. Continuous controls: APERTURE narrows tube thickness and changes energy forcing;
    TENSION changes elastic smoothing and vertical shell stretch; COUPLING changes
    strain/energy interaction and pleating; FLOW scales all physical time;
    TRAIL changes filament extent, energy drain and membrane loading. No renderer
    selection or integer artistic variants.
12. Feedback: energy drives radial displacement; displacement gradients resist
    energy transport. Filament loading drains the common mantle energy, whose
    resulting field sets filament curvature, length and emitted light.

Visual thesis: an abyssal crown suspended around an empty oculus, with nacreous
pleats carrying light into tapering filaments. Dark water occupies the aperture
and surrounding frame; the bright rim establishes the focal hierarchy.

Validation and visual observations will be recorded below after inspection.

## Rendering milestone

`cargo test rosette -- --nocapture`: 6 unit and 2 integration tests passed before
implementation. Foundation commit: `c98a998`.

The first colored review used seeds 42, 7, 913; times 0, 4, 11; 80x24, 160x56,
24x9; aperture 0, .25, .49, .50, .51, .75, 1. The undersized crown was enlarged,
filaments were attached to frontal material meridians to preserve the dark
oculus, and energy gradients replaced uniform filament torsion. Two surface
sutures clarify the lips. Filaments were then shortened to leave a bottom margin.
The color hierarchy is gold upper membrane, cyan sidewalls, dim blue-green tails.

The five controls share the same solver and projection. APERTURE moves the source
phase and tube radius; TENSION changes strain smoothing and vertical radius;
COUPLING changes conductance, radial strain, pleating and filament curvature;
FLOW advances the common clock; TRAIL changes energy drain and filament extent.
No artistic control is rounded to an integer or selects a different renderer.

## Snapshot and control validation

All eight snapshots were inspected as plain ASCII before acceptance:
`pelagium_canonical`, `pelagium_t4`, `pelagium_t11`, `pelagium_open`,
`pelagium_small`, `pelagium_aperture049`, `pelagium_aperture051`, `pelagium_max`.
Canonical seed is 42, palette `deep`; the maximum snapshot is at t=7.
The small snapshot is 24x9; remaining snapshots are 80x24.

`env -u NO_COLOR cargo test modes::_101`: 12 passed. Tests compare full Cells
(including colors), revisit the canonical frame after changing seeds/time/knobs,
exercise all five live controls, sweep each control through seven ordered
positions, verify reciprocal energy/strain effects, and check empty/tiny grids.

Final color renders are committed under `perf/previews/101_pelagium/`.
`0_render.py` reproduces the 14-frame review from the ordinary CLI, including the
160x56 view. The five primary APERTURE values are 0, .25, .50, .75, 1; .49 and
.51 bracket the midpoint. The opening expands while tube width contracts;
the pleated silhouette and the twelve anchored filaments retain identity.

## Existing validation failures

No shared renderer, playback, prior mode, or existing test was edited. The
following checks fail outside the new mode. Exact reproduction commands:

```sh
cargo test
env -u NO_COLOR cargo test
env -u NO_COLOR cargo test morph::iterate_frame_tests::gem_bad_roll6_ansi_regression -- --exact --nocapture
env -u NO_COLOR cargo test --test 3_relay_throughput
env -u NO_COLOR cargo test --test snapshot_modes
```

The shell exports `NO_COLOR=1`. The first run reports 523 passed, 2 failed,
18 ignored; the adjacent-RGB assertion fails because the output lacks color.
With NO_COLOR removed: 524 passed, 1 failed, 18 ignored. The remaining
`gem_bad_roll6_ansi_regression` failure reproduces by itself:

```text
actual   (14175131, 850398, 8769485, 2414016, 2860585, 0)
expected ( 9950827, 710305, 4748083, 2019066, 2976090, 0)
```

Tuple fields are bytes, controls, foreground bytes, glyph bytes, cursor bytes,
repeat bytes. The test renders the existing gem-aetherium-2 fixture through the
shared encoder; it does not render Pelagium.

Separate integration validation: generator, trace, and input replay tests pass.
The two existing Prismata PTY regressions fail: pause appears after 2.126073875 s
(2 s limit), and target seed is 44 after 5 s (expected 46, tolerated minimum 45).
These tests ran while the guarded, single-job release build was active.
The integration snapshot suite reports 197 passed, 2 failed: existing
`nightglass_seed_42` and `nightglass_rain_running_t9` snapshots differ in glow.
Their pending outputs were preserved under
`perf/results/pelagium-existing-regressions/`; existing snapshots were not accepted.

The touched-file diff against starting commit `57b9baf` confirms that all named
shared implementations and their tests are unchanged.

## Live E2E and performance stop

Executed the required live suite, with the lane's external Cargo target directory:

```sh
scripts/13_e2e.sh --mode pelagium \
  --binary /Users/chrishafley/.agent/lanes/art-crazy-astra-high/target/release/ascii-renderer \
  --directory perf/results/pelagium-e2e
```

The guarded release build passed (application 9m28s, terminal example 38.72s).
All 4 watchdog and 3 evidence-reader tests passed. The live suite exited 1:

| Case | Actual result |
| --- | --- |
| backpressure | Failed: knob application 1003.104208 ms, 250 ms limit. The telemetry assertion also found 0 terminal-wait microseconds, requiring >500,000. |
| workflow | Passed: actual iTerm pixels changed, live aperture reached the renderer in 12 ms, saved preset, random controls, pause/resume, resize, q return and terminal restoration (54 ms). |
| seed-search | Passed. |
| pin-inputs | Passed. |
| bad-400x200 | Failed: before/after captured art pixels were identical. Cleanup set observer stop; watchdog recorded `observer_requested_stop` and terminated the owned tree. |
| max-400x200 | Not run after the observer stop. |

The failing large case used seed 42, default parameters, terminal 400x200,
grid 366x199, theme auto. Its renderer generated 27 frames from t=0.000729 to
t=1.641229; the final frame reports 3,803 changed cells. Render time was
4.339 / 4.498 / 6.997 ms (min / median / max). The captured-pixel failure remains
unresolved; generated frames do not establish successful GUI painting.
The smaller workflow rendered 40 frames at 0.941 / 1.030 / 2.478 ms.
These are observed application rendering times, not a passed performance gate.

The guard stopped on the observer's cleanup request, not a resource threshold.
Peak sampled large-case owned RSS was 197,568 KiB; iTerm RSS was 458,336 KiB;
case artifacts were 1,240,222 bytes. No limits were raised and no further live
probes were launched after the stop.

Unexecuted checks: max-400x200; the backpressure suite's separate quit-latency
subcase; remaining large-case live controls/cadence/restoration assertions;
100 simultaneous-max 2000x2000 frames; random-knob 2000x2000 frames; the separate
2000x2000 stalled-consumer input-latency probe. The latter required commands are:

```sh
python3 scripts/3_test_animation.py "$CARGO_TARGET_DIR/release/ascii-renderer" --mode pelagium --max --size 2000x2000
python3 scripts/3_test_animation.py "$CARGO_TARGET_DIR/release/ascii-renderer" --mode pelagium --size 2000x2000
python3 scripts/4_test_input_latency.py "$CARGO_TARGET_DIR/release/ascii-renderer" --mode pelagium --size 2000x2000 --stall 10 --max-ms 250
```

Each needs the existing external watchdog before launch. They are recorded as
unexecuted, not as passing or as replaced by the smaller workflow.

Full evidence remains in `perf/results/pelagium-e2e/` (PNG captures, exact inputs,
NDJSON, guard records, profile) and `perf/results/pelagium-existing-regressions/`.
The committed `perf/fixtures/101_pelagium_validation.json` preserves commands,
effective inputs, binary identity, failures and unexecuted cases for the parent.
The aperture .49/.51 color-review frames differ in 51 of 1,920 glyph positions.

Status: blocked. The artwork, generated integration, snapshots, and visual
identity are implemented; shared test failures and the guarded live failure
prevent the requested full validation result.

Boop-Ask: Should the parent expand scope to resolve the shared validation
failures and authorize rerunning the remaining guarded checks?
