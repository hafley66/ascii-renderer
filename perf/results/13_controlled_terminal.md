# Controlled terminal processing comparison

Captured 2026-09-07 in a dedicated raw iTerm2 3.6.3 session, 426x135.
Grid 392x134; gem-aetherium-2, seed 42, recorded roll 6. Exact palette,
knobs and f32 times come from PID 54254 frames 165 through 177 in
[inputs.ndjson](13_controlled_terminal/inputs.ndjson).

One verified test executable exported frames through the production renderer
and encoder at cc2715a. The comparison reused those exact bytes. The literal
arm expands REP into the preceding glyph repeated N times; it preserves cursor
commands, style transitions and frame partition. The REP arm uses the exported
bytes. Executable, input and harness hashes are in
[metadata.json](13_controlled_terminal/metadata.json).

The harness writes each complete frame through a nonblocking real tty, then
requests a cursor position report (DSR). Timings end when the report is received.
This measures terminal processing acknowledgement, including PTY and parser
backpressure. It does not measure completed painting, physical keyboard latency,
Rust relay overhead or geometry generation. Different binary builds and different
animation phases are excluded from this comparison.

The Unicode REP probe produced the expected cursor (1,13) after one shade glyph
and 11 repeats. Corresponding frame cursor reports matched. No screenshot-based
cell comparison was performed.

## Measurements

The first run completed 13 literal frames and 10 REP frames before the unchanged
128 MiB iTerm2 growth breaker stopped it after 1.73 seconds. Matched delta indices
1 through 9: literal acknowledgement total 497,918 us, REP 611,765 us (+22.9%).
Bytes fell 17.7%. The incomplete counterbalanced run is supporting evidence only.

The reduced run completed literal, REP, REP, literal (ABBA), each with the same
warmup frame and two deltas. The initial full repaint is excluded from this table.

| Four delta observations per arm | Literal | REP | REP change |
| --- | ---: | ---: | ---: |
| Bytes | 656,892 | 543,730 | -17.2% |
| Write calls | 1,283 | 1,064 | -17.1% |
| Blocked writes | 639 | 530 | -17.1% |
| Time writing, us | 109,565 | 117,702 | +7.4% |
| Time through acknowledgement, us | 157,417 | 191,138 | +21.4% |

This short controlled sample provides no evidence of a terminal latency benefit
from REP. It does not establish a general performance law for other terminals.
REP was removed from the default encoder. The previous indexed-color behavior
remains. The roll-6 byte regression returns to 7,061,339 bytes across 60 frames.

## Evidence and repeat

- [Completed ABBA measurements](13_controlled_terminal/run3-measurements.ndjson)
- [Completed ABBA watchdog](13_controlled_terminal/run3-guard.ndjson)
- [Interrupted larger run](13_controlled_terminal/run2-measurements.ndjson)
- [Larger run watchdog](13_controlled_terminal/run2-guard.ndjson)
- Three original REP frame payloads are retained beside these logs.
- Harness: `scripts/6_terminal_ab.py DIRECTORY`.

Copy the three retained `frame-*.ansi` files into a fresh directory. In a dedicated
426x135 terminal, run the harness through `scripts/5_probe_guard.py` with the
shared iTerm2 PID as `--watch-pid`. Default 15-second, 256 MiB owned, 768 MiB
watched, 128 MiB growth and 32 MiB artifact ceilings apply. The harness creates
`measurements.ndjson` exclusively and restores tty settings on normal/error exit.
The guard restores the tty if it kills the child. Never interpret guard exit
124/125 as a completed comparison.

## Corrections to earlier conclusions

The previous synthetic-byte reductions established byte savings only. The latest
user log did not contain a verified build identifier. Cached SCIP placed
`write_sgr` in an older source layout and therefore did not confirm the current
hot path. Source reads and live measurements are the evidence used here.
