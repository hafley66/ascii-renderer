# opus-5-5-prothesis

The laying-out of the dead: one shrouded body on a bier, a lamp at its head, mourners
with their hands to their heads, and a name. Registered trait mode,
`src/modes/_134_opus_5_5_prothesis.rs`. The prompt sequence and the measured reading of
the corpus it inverts are in `art/prompts/134_opus_5_5_prothesis.md`.

- [What it draws](#what-it-draws)
- [Knobs](#knobs)
- [Timeline](#timeline)
- [Knob sweep, 2000x1000](#knob-sweep-2000x1000)
- [Layer coverage](#layer-coverage)

## What it draws

Plain 7-bit ASCII in neutral grays on the terminal's own background. The palette and
theme are ignored on purpose, so there is no hue. The scene sits low in an otherwise empty grid:
ink covers 7 percent of 80x24 and 3 percent of 200x60.

The shroud is an authored height profile, 31 control points from the pillow to the toes,
interpolated with monotone cubic tangents so it never overshoots. It is drawn the way a
hand draws ASCII line. Every glyph attaches left and right at quarter-row heights
(`_` 0-0, `.` 0-2, `-` 2-2, `'` 2-4, `/` 0-4, and mirrored), and a Viterbi pass over a
band of attach levels picks the connected glyph run closest to the curve. Turns and
direction reversals cost extra, so the line settles into `__..--''` idioms instead of
flickering. Steep stacks at the two ends become `(` and `)`. The body never moves,
so the line is traced once per size and cached in a thread-local.

The seed chooses one of 28 names from Attic grave stelai. It does not move or reshape
anything else.

## Knobs

| key | label | range | default |
| --- | --- | --- | ---: |
| `VIGIL` | seconds until the room is dark | 10 to 3600 | 273 |
| `MOURNERS` | mourners at the bier | 0 to 7 | 3 |
| `STATURE` | body length, share of width | 0.25 to 0.8 | 0.56 |
| `BIER` | bier line, share of height | 0.45 to 0.9 | 0.74 |
| `NIGHT` | darkness the lamp leaves behind | 0 to 1 | 0.6 |

Positional CLI args follow that order:
`cargo run -- <seed> opus-5-5-prothesis <theme> [vigil] [mourners] [stature] [bier] [night]`

## Timeline

`VIGIL` is in wall seconds of the animation player, whose clock runs at 3.6 units per
second (`morph.rs` adds 0.06 per 1/60 s tick). The default of 273 s is 4'33".

| fraction of the vigil | what happens |
| --- | --- |
| 0 | three mourners lament (`/o\`), the lamp burns; only the flame glyph changes |
| 0.10 to 0.80 | each mourner lowers their arms, then leaves, at a seeded moment |
| 0.88 | the flame gutters to `.` |
| 0.92 | the lamp goes out |
| 0.88 to 1.00 | the light drains; the shroud keeps the most of it |
| after 1.00 | nothing changes again; every later frame is identical, colours included |

Tests that hold this: `time_only_takes_away` (the inked cell count never rises across the
whole vigil), `it_never_loops` (frames from the end out to 10,000 vigils are identical),
`only_the_flame_moves_while_they_stand`.

## Knob sweep, 2000x1000

`perf/knob_sweep.sh opus-5-5-prothesis 2000 1000 2`, theme moss, dt 0.06, release.
Raw output: `perf/results/opus-5-5-prothesis.md`. The script's own `sed` drops the table
because libtest prints `test ... ` on the same line as `# knob sweep`; the raw output was
captured from the same command with that prefix stripped.

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 2180 | 1089.8 | 0.92 | 0.91 | 1.30 | 2.13 | 1.00x |
| STATURE=0.8 | 2081 | 1040.0 | 0.96 | 0.95 | 1.29 | 4.63 | 1.05x |
| MOURNERS=7 | 2137 | 1068.5 | 0.94 | 0.92 | 1.15 | 7.18 | 1.02x |
| BIER=0.9 | 2161 | 1080.3 | 0.93 | 0.93 | 1.04 | 1.63 | 1.01x |
| VIGIL=3600 | 2172 | 1085.9 | 0.92 | 0.91 | 1.11 | 4.95 | 1.00x |
| NIGHT=1 | 2205 | 1102.5 | 0.91 | 0.90 | 1.01 | 1.52 | 0.99x |

Hotspots at `STATURE=0.8`, 1092.5 fps:

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| body | 1.0 | 3.8 | 344.0 | 0.4% |
| bier | 1.0 | 2.1 | 10.7 | 0.2% |
| mourners | 1.0 | 0.4 | 4.1 | 0.0% |
| lamp | 1.0 | 0.3 | 5.5 | 0.0% |
| knobs | 1.0 | 0.2 | 3.4 | 0.0% |
| inscription | 1.0 | 0.0 | 0.5 | 0.0% |

The `body` max is the first frame, which traces and caches the line. In-module
`frame_cost` at 200x60, release: 0.013 ms average.

## Layer coverage

Six timers (`knobs`, `mourners`, `body`, `bier`, `lamp`, `inscription`) cover every
statement of the mode's render except the layout arithmetic. `perf/layer_coverage.sh`
still marks the mode thin, because it divides by the whole `IterateFrameRenderer::render`
call. That call blank-fills the entire grid before the mode runs, and that fill
is nearly all of the frame here:

| grid | attributed |
| --- | ---: |
| 80x24 | 42.1% |
| 400x120 | 7.0% |
| 2000x1000 | 0.7% |

`perf/layer_coverage.sh <w> <h> 20 moss opus-5-5-prothesis`, release. The share falls as the
grid grows because the blank fill scales with the grid and the drawing does not.

The mode writes a few hundred cells. The fixed costs around it are larger than its own
work: the blank fill, the registry lookup, the RNG reseed, and the timer bookkeeping. The
only way to reach 85 percent would be to redo that fill inside a timer, so the mode does
not. The registry gate `every_registered_mode_that_renders_in_process_has_layer_timers`
passes for it.
