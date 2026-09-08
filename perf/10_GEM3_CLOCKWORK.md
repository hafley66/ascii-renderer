# Gem 3 clockwork revision

Implements the armillary direction from `9_GEMINI_CREATIVE_REVIEW.md` in the
existing `gem-aetherium-3` mode. Gem 1 and Gem 2 retain their implementations.

The stationary engraved dial now surrounds two to four tumbling 3D gimbals,
a diameter-spanning rotating alidade, three counter-rotating gears, and planets
with paired moons. The RINGS knob controls the engraving count and gimbal count;
TILT controls ring/planet inclination; HARMONIC and PETALS alter ring contours.
Existing seed, speed, trail, comet, nebula, spread and tracer controls still apply.

A pitched/yawed circle supplies each ring's XYZ coordinates. Perspective maps
these to terminal coordinates. All moving marks are sorted from far to near;
nearer marks overwrite farther marks at crossings. Equal-depth marks retain
submission order. Background engravings are intentionally behind all moving ink.
Planet glyph offsets use terminal cells. Other geometry scales to the art grid.
All output glyphs remain ASCII and occupy one terminal column.

Maximum moving submissions per frame:

| Geometry | Marks |
| --- | ---: |
| Gimbals | 192 |
| Sighting arm | 129 |
| Gear teeth and spokes | 72 |
| Planets, tails and moons | 156 |
| Comets | 64 |
| Tracers | 24 |
| Core | 1 |
| Total | 638 |

The mark vector reserves 638 entries. Loop bounds limit submissions to 638;
an assertion checks the reserved limit. Sorting uses bounded additional scratch.
Static circle scratch remains capped at 4,096 triples. Work is
O(W*H + R*min(3*(W+H),4096) + M*log(M)); no simulation persists between frames.
Two fixed-input frames can change at most twice the submitted mark count.
Knob changes can also replace stationary geometry and exceed that motion bound.

The existing 20,000-byte real-encoder gate remains in the 100-frame all-maximum
regression. This is an output-size gate, not a claim about terminal painting FPS.
The older measurements in `7_GEM3.md` and `8_GEM3_RESULTS.json` describe the prior
249-mark implementation and are retained for comparison.

## Validation

The pre-change focused baseline passed all three Gem 3 tests. The revised mode
passes four focused tests, including the new depth-crossing check. Both 80x30
snapshots were reviewed, accepted individually, and rerun without force-pass.
The encoder test exercises 100 maximum-knob frames at each of 162x61 and 366x199.

The initial unit run completed with 435 passes, two failures, and 15 ignored tests.
The failures are the pre-existing shared adjacent-RGB encoder assertion and
Gem 2's ANSI regression assertion. All 189 integration tests pass after updating
the generator's stale one-line snapshot to reflect opt-in function tracing.
Artifacts: `62_gem3_creative_baseline`, `64_gem3_creative_verified`,
`65_gem3_creative_suite`, and `67_gem3_creative_integration` under `perf/results`.

## Color and scheduling controls

The shell exported `NO_COLOR=1`. The first focused payload checks and direct
100-frame probes inherited it, while the native E2E helper explicitly removed
it. Initial colored E2E output reached about 24 KB per maximum-knob frame.
The 1,170-mark draft therefore exceeded the intended colored payload budget.
The final composition reduces sample density while retaining all mechanisms.

The Gem 3 payload test now re-executes only itself with `NO_COLOR` removed when
necessary. This avoids mutating process-wide environment during parallel tests
and guarantees that the 20 KB gate includes color escapes. The direct animation
probe also removes `NO_COLOR` for its app child, matching the E2E harness.

Initial direct runs `69_gem3_clockwork_max` and `70_gem3_clockwork_random` also
used background task priority, unlike normal demo playback. Their wall times
were 7.628 and 6.455 seconds for 100 frames. These measurements are retained
as mismatched probe configurations, not a controlled performance comparison.
Final direct probes use normal runtime priority; compilation keeps background
priority. Runtime guards remain 15 seconds, 256 MiB and 32 MiB artifacts.

The final colored unit run (`75_gem3_color_suite`) has 436 passes, one failure,
and 15 ignored tests. The remaining failure is Gem 2's pre-existing ANSI byte
assertion. The adjacent-RGB assertion passes with color enabled. All four Gem 3
tests pass, including normal equality checks against the reviewed final snapshots.

## Released source and remaining validation

Final source is committed and pushed as `4fda48e`. The installed release binary
still contains the earlier 1,170-mark clockwork revision. The final 638-mark
revision passed its colored real-encoder test and the unit run above, but its
release playback has not been measured.

Two final release builds received rustc SIGTERM below watchdog limits:
`e2e-build-1788833391` and `e2e-build-1788833502`. The third build,
`e2e-build-1788833655`, was stopped at the user's request. Its guard recorded
`signal_2` and termination of its owned processes. No further probe is running.

The earlier clockwork release (`68_gem3_clockwork_e2e`) passed workflow, default
animation and maximum-knob animation; the shared stalled-output backpressure
case failed. Those results describe the earlier revision. Measurements in
`11_GEM3_CLOCKWORK_RESULTS.json` are explicitly labeled historical and do not
claim performance for the final 638-mark version. Raw logs remain ignored.
