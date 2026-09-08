# Gem 2: fog, rays and ring runes disabled individually

Controlled inputs: seed 42, recorded bad roll 6 from
`fixtures/12_gem_aetherium_2_bad_roll6.json`. Each case holds the same remaining
knobs, palette, and 61 native animation time values. The initial frame is excluded;
statistics use the next 60 frames, t=0.12 through t=3.66. Art grids are 162x61 and
366x199, inside 196x62 and 400x200 terminals. The fixture's original larger
terminal dimensions are replaced equally in every case.

The existing release binary, real Unix PTY supervisor and ANSI encoder were used.
The consumer drains the PTY; GUI painting is not measured. NO_COLOR is removed
for the app child. No renderer changes were made for this experiment.

## Colored output, median bytes per frame

| Disabled controls | 162x61 bytes | Reduction | 366x199 bytes | Reduction |
| --- | ---: | ---: | ---: | ---: |
| None | 55,084 | 0% | 165,667 | 0% |
| Aetherial cosmic dust fog | 51,127 | 7.2% | 147,022 | 11.3% |
| Mystic chronos ray bursts | 51,717 | 6.1% | 129,880 | 21.6% |
| Arcane glyph ring density | 54,753 | 0.6% | 165,459 | 0.1% |
| Fog + ray bursts | 47,386 | 14.0% | 113,514 | 31.5% |
| Fog + ray bursts + ring glyphs | 47,109 | 14.5% | 113,371 | 31.6% |

Fog and rays contribute measurable output, particularly at the larger size.
Disabling ring runes barely changes the payload. That control replaces rune
characters with ordinary strokes at the same ring samples; it does not remove
rings. Zodiac and planet symbols remain in these tests, so this result does not
isolate every floating-symbol layer.

With all three disabled, 113 KB/frame remains at the larger size. Using the median
payload at 60 updates per second would require approximately 6.8 MB/s. The earlier
user-run relay measurement was about 0.95 MB/s. Removing these layers alone does
not establish that Gem 2 can sustain 60 terminal updates per second.

## Timings and variability

Large-grid median times in milliseconds, using the final baseline repeat:

| Disabled | Generate | Encode | Worker output | Total work |
| --- | ---: | ---: | ---: | ---: |
| None | 1.042 | 2.831 | 1.584 | 5.487 |
| Fog | 0.708 | 2.408 | 1.600 | 4.684 |
| Rays | 0.663 | 2.568 | 0.097 | 3.446 |
| Ring runes | 1.093 | 2.845 | 1.713 | 5.833 |
| Fog + rays | 0.306 | 2.082 | 0.076 | 2.466 |
| Fog + rays + ring runes | 0.317 | 2.139 | 0.097 | 2.554 |

The initial verified large-grid baseline had 47.925 ms median total work and
34.997 ms worker-output time. Its repeat had **identical byte counts and changed
cell counts at every compared time**, but much shorter waits. Scheduling and
output conditions varied during this matrix. Byte reductions are reproducible;
the timing differences must not be presented as an isolated end-to-end speedup.

## Reproduction and evidence

`scripts/3_test_animation.py --fixture FILE` now holds the fixture seed, theme and
knobs fixed. It checks the recorded knobs against their exact Rust f32 values,
checks the seed and dimensions, and disables random input injection. The original
maximum and random probe modes remain available.

Saved candidate inputs:

- `fixtures/13_gem2_without_fog_and_rays.json`
- `fixtures/14_gem2_without_fog_rays_and_runes.json`

Example bounded reproduction, using a fresh trace path:

```bash
mkdir -p perf/results/manual-gem2-no-fog-rays
python3 scripts/5_probe_guard.py \
  --state perf/results/manual-gem2-no-fog-rays/guard.ndjson \
  --artifact-dir perf/results/manual-gem2-no-fog-rays \
  --max-seconds 15 --max-owned-mib 256 --max-artifact-mib 32 -- \
  python3 scripts/3_test_animation.py target/release/ascii-renderer \
  --mode gem-aetherium-2 \
  --fixture perf/fixtures/13_gem2_without_fog_and_rays.json \
  --size 366x199 --frames 61 --timeout 12 \
  --trace perf/results/manual-gem2-no-fog-rays/frames.ndjson
```

All 13 verified cases completed under their own 15-second, 256 MiB, 32 MiB guard.
The initial fixture check failed only because Python decimal values were compared
to serialized Rust f32 values; that check was corrected and the baseline rerun.
Raw logs are ignored under `perf/results/86_gem2_layer_ablation`. Exact measured
inputs, timing summaries, watchdog memory peaks and the binary hash are committed
in `15_GEM2_LAYER_ABLATION_RESULTS.json`. All probe processes have finished.
