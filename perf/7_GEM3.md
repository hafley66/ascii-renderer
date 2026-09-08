# Gem Aetherium 3 and tracing defaults

`gem-aetherium-3` is appended to the demo registry. Its independent implementation
uses engraved orbital shells, nebula bands, lanterns, comets and harmonic tracers.
It renders analytically from the shared frame inputs and declares all 12 controls
through `Mode::params`, so normal presets and telemetry apply automatically.

At simultaneous maxima, animation draws at most 249 marks; two fixed-input
frames can differ in at most 498 cells. Stationary structure is reconstructed
each frame but omitted by the existing retained terminal diff. Changing knobs
can change that structure and exceed the animation-only cell budget.

Time complexity is O(W*H + R*min(3*(W+H),4096) + 400 + 249). Scratch geometry
holds at most 4096 triples of f32, approximately 48 KiB. Glyphs are ASCII.

## Measured output

Same headless demo harness, seed 42, 366×199 art within 400×200 terminal. Medians
below cover nine fixed-input frames after the initial draw. Each mode uses its
own maximum knobs. The scenes and compilation of tracing hooks differ; these
measurements do not isolate tracing overhead.

| Maximum-knob mode | Generate | Encode | Frame work | Output bytes |
| --- | ---: | ---: | ---: | ---: |
| Gem 2 | 1.391 ms | 3.902 ms | 9.695 ms | 228,869 |
| Gem 3 | 0.292 ms | 1.664 ms | 2.012 ms | 6,293 |

Gem 3 emits 97.25% fewer bytes in that comparison. In the separate 100-frame
max-knob playback run, steady median work is 1.864 ms, maximum 2.114 ms, and
maximum output is 4085 bytes/frame. The 100-frame random-jump run exercised 25
rolls, with steady median work 1.666 ms and maximum 2.113 ms; knob jumps peaked
at 56,412 bytes. Initial full redraws are excluded from these steady numbers.
Exact input records, hashes and stage totals are in `8_GEM3_RESULTS.json`.

## Verification

Two fixed-seed snapshots were visually inspected. Three Gem 3 tests cover
snapshots, determinism, motion, tiny grids, all-max cell limits and 100 frames
through the real ANSI encoder with a 20 KB steady payload gate.

Before tracing feature gating: full unit run had 433 passes, three pre-existing
failures and 15 ignored tests. The failures were the adjacent-RGB encoder
assertion, Gem 2 ANSI byte regression, and lifetree4 timing. All 189 integration
tests passed separately.

With hooks compiled out: actual-demo workflow, default-knob animation and
max-knob animation passed headlessly. Deliberate output backpressure remains a
failing shared-playback case. The subsequent full Rust test rebuild received
SIGTERM before executing; its artifact is `perf/results/61_no_hooks_suite`.
GUI painting is unmeasured. Probe limits stayed at 15 seconds and 256 MiB.

## Function tracing

Normal builds now compile out the 2549 declared function instrumentation
attributes and the manual main span. `--features function-trace` includes them;
`ASCII_FUNCTION_TRACE=<directory>` then enables compressed entry/exit recording.
The lossless logging queue can block producers when enabled and saturated.
Periodic first/every-tenth-frame records stay enabled independently.

`scripts/13_e2e.sh --function-trace` selects the feature explicitly. App and native
terminal helper builds run separately to prevent helper-only dependency features
from changing the normal application build. All builds retain the one-job,
background-priority 1 GiB watchdog.
