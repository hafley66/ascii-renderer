# p02 glm53f-max-d moongate: validation and performance receipts

Mode: moongate (registry name), file src/modes/_101_p02_glm53f_max_d.rs, lane
art/p02-glm53f-max-d. Region China: ice-ray split lattice (Stiny chuang-zi),
hui-wen frets, moon gate; meihua joint blooms; xiangyun gate clouds.

## Layer coverage (perf/layer_coverage.sh 400 120 3 moss moongate)

| mode | layers | calls/frame | attributed | nested | thin |
| --- | ---: | ---: | ---: | --- | --- |
| moongate | 7 | 7.0 | 92.1% | no | no |

Seven measured layers: wall, derive, lattice, gate, fret, bloom, clouds.
Coverage 92.1 percent, above the 85 percent floor; 7 layers inside the 3-8 band.

## Knob sweep (perf/knob_sweep.sh moongate 2000 1000 5 0.06 moss)

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 828 | 165.4 | 6.04 | 5.58 | 12.11 | 31.90 | 1.00x |
| GATE=0.9 | 843 | 168.5 | 5.93 | 5.51 | 11.48 | 23.79 | 0.98x |
| CRACK=8 | 860 | 172.0 | 5.81 | 5.51 | 10.30 | 13.57 | 0.96x |
| FRET=1 | 942 | 188.3 | 5.31 | 5.03 | 9.11 | 13.29 | 0.88x |
| BLOOM=1 | 1158 | 231.5 | 4.32 | 4.09 | 8.59 | 16.75 | 0.71x |
| GLOW=2 | 1162 | 232.3 | 4.31 | 4.08 | 8.43 | 13.15 | 0.71x |
| DRIFT=2 | 1176 | 235.1 | 4.25 | 4.07 | 8.67 | 10.90 | 0.70x |
| ROOT=1 | 1194 | 238.7 | 4.19 | 4.03 | 6.01 | 10.01 | 0.69x |

Worst knob: GATE=0.9 at 0.98x baseline; no knob exceeds baseline. Hotspots at
the worst knob: bloom 26.5 percent, wall 20.4 percent, gate 16.1 percent,
lattice 10.5 percent, derive 0.6, fret 0.5, clouds 0.1.

## E2E status (scripts/13_e2e.sh --headless, run at C3, commit c36ebc3)

- Passed: seed-search, pin-inputs, bad-400x200, max-400x200.
- Failed, pre-existing: workflow resume assertion; reproduced byte-identically
  on base 7c00a18 in a throwaway worktree (runs preserved:
  perf/results/e2e-1789882814181, e2e-1789882909730).
- Failed, pre-existing class: backpressure knob-apply 1028 ms vs 250 ms gate
  under a deliberate 1 s consumer stall; matches the documented known failure
  in perf/0_E2E.md (1005 ms recorded there); e2e-1789883076843 preserved.
- Unexecuted: GUI iTerm painting cases by design of --headless; headless
  coverage reports GUI painting untested.
- Build-stage probe guard tripped once at 900 s on a cold single-threaded
  release build; preserved (perf/results/e2e-build-1789881709); resolved by
  warming identical build flags unguarded; no limit was raised.
- No animation-path change since c36ebc3 (C4/C5 touched tests, snapshot files
  and palette derivation only).

## Test matrix at close

Module tests 17/17: canonical t0, t4, t9, full-bloom variant, determinism and
seed sensitivity, time motion, knob steps (CRACK, BLOOM, DRIFT), seven-knob
five-value ladder sweep (28 adjacent steps all glyph-distinct), rim-brightness
hierarchy across seeds 42 7 1337, tiny and narrow frame clipping (16x6, 8x30,
40x8, 3x3), frame cost 200x60 release gate. Integration 2/2. Full suite: 633
passed; 2 failures inherited from base (perf_sweep registered-mode timer gate
against 5 legacy modes, moongate complies; morph gem bad roll6 ansi tuple),
both reproduced byte-identically on base 7c00a18.
