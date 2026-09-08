# Gem 3 composition variation

The existing `gem-aetherium-3` now has 15 knobs. Three new controls are appended
so saved values for the previous 12 keys remain usable:

| Control | Effect |
| --- | --- |
| LAYOUT | 0 armillary, 1 binary observatories, 2 vertical tower, 3 spiral, 4 eclipse |
| ECCENTRIC | Instrument separation, stretch and seed-derived asymmetry |
| FILIGREE | Stationary radial lattice density |

Each layout defines one to four instrument sites. Engravings, rings, planets,
gears, tracers, and sweeping arms use those same sites, so changing layout
rearranges the complete composition. The layouts also use different cloud fields;
the spiral opens the engraved rings into spirals, and the eclipse opens them
into crescents. Every frame remains analytic in seed, dimensions, knobs and time.

The previous global moving-mark budget is shared across the sites: at most 638
marks, rather than multiplying the budget by the number of instruments. Added
stationary lattice work is bounded to 24 rays with 16 samples each. The existing
frame telemetry and preset mechanisms handle the three controls automatically.

## Verification

The focused pre-change baseline passed. Five revised tests pass, with the three
snapshots reviewed individually: initial frame, moving frame, and the five-layout
gallery. The gallery also checks determinism, seed variation, and more than 800
differing glyphs between every pair of layouts at 162x61 using default knobs.

The color-enabled real-encoder regression checks 100 frames at every layout's
maximum knobs at both 162x61 and 366x199: 1,000 frames in total. Each noninitial
frame stays under the existing 20,000-byte limit. The test removes NO_COLOR in
an isolated child when necessary, so the limit includes actual color escapes.
The 638-mark bound, tiny-grid behavior, and depth compositing checks also pass.
Registry generation is current. Raw evidence remains ignored under
`perf/results/79_gem3_layout_baseline` and `80_gem3_layout_tests`.

The full colored suite (`81_gem3_layout_suite`) has 437 unit passes, one existing
Gem 2 ANSI assertion failure, and 15 ignored tests. All 189 integration checks
pass.

## Release playback

The guarded release build succeeded. `target/release/ascii-renderer` includes
source commit `1f1a654`. Actual-demo headless workflow, default animation and
maximum-knob animation passed; the existing stalled-output backpressure case
still fails. GUI painting is unmeasured.

At 366x199 art inside a 400x200 terminal, the nine fixed-input E2E frames had:

| Inputs | Median frame work | Maximum frame work | Median bytes | Maximum bytes |
| --- | ---: | ---: | ---: | ---: |
| Defaults | 1.887 ms | 2.350 ms | 7,039 | 7,621 |
| All maxima, eclipse layout | 2.099 ms | 2.386 ms | 11,622 | 11,954 |

The colored direct playback test reached its 100-frame/25-roll checkpoint in
1.695 seconds. The complete trace includes two additional shutdown-period frames:
102 frames and 26 roll values. It contains all five LAYOUT values. Among frames
with unchanged knobs, median work was 2.414 ms and maximum was 30.776 ms.
Knob transitions had median work 2.724 ms, with one 62.573 ms maximum including
58.058 ms in presentation. Changing layout replaces stationary content as well,
so transition payloads exceeded the animation-only 20 KB gate, peaking at 79,136
bytes. The initial full repaint was 116,986 bytes.

Inputs, timings, color settings and the release binary hash are saved in
`13_GEM3_LAYOUT_RESULTS.json`. Raw artifacts are under `82_gem3_layout_e2e` and
`83_gem3_layout_random`. Runtime watchdogs stayed at 15 seconds and 256 MiB;
compilation used one background job and the existing 1 GiB guard. All probes
have finished.
