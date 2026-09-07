# Gap-cost candidate: output matched, terminal processing slowed

The candidate made dirty-gap pricing use the same horizontal cursor command as
emission. It preserved visible output but increased cursor-command count.
The complete candidate diff is retained in 18_gap_cost/candidate.patch and was
removed from production after measurement. The existing pacing fix remains.

## Output and actual terminal measurements

Eight exact recorded frames were generated through the changed production
encoder. An independent pyte 0.8.2 terminal emulator compared all 80000 cells
after each frame, including glyphs, visible foreground/background and style,
plus cursor position. All 640000 cell comparisons and eight cursor states
matched. Foreground colors on blank cells are ignored because they are invisible.
This is emulator equivalence, not a screenshot comparison of iTerm painting.
See [pyte's screen/stream API](https://pyte.readthedocs.io/en/latest/api.html).

Frame2: printable characters fell 16656 → 14193; cursor commands increased
4762 → 5519; foreground commands stayed 5885. Art bytes fell127209 → 124871.
The decoder's candidate summary uses baseline_logged_minus_candidate_art_bytes
for the residual against the old live log; that residual includes byte savings
and must not be interpreted as candidate UI overhead.

Two raw iTerm2 runs at400x200 replayed the same full frame and two deltas in
before/after/after/before order. Both encodings were bracketed with synchronized
output. Each run completed under the unchanged15s watchdog,256MiB owned RSS,
user-approved256MiB terminal growth and768MiB absolute terminal RSS ceilings.

Mean per delta, four observations per arm per run:

| Run | Baseline through DSR | Candidate through DSR | Change |
| --- | ---: | ---: | ---: |
| 1 | 37.645 ms | 42.306 ms | +12.4% |
| 2 | 36.863 ms | 43.229 ms | +17.3% |

Mean payload including sync fell125998.5 →123586 bytes (-1.9%). Mean write
intervals were20.587 →20.757ms and19.898 →20.634ms. Reduced byte count did not
provide a terminal-processing latency benefit. Extra cursor operations and
shorter text spans are the measured output differences; this test does not time
individual terminal commands. DSR measures terminal stream acknowledgement,
not physical paint completion or application input latency.

The full Rust suite passed after removal:432 unit tests,3 integration tests,
186 snapshots,15 ignored. The candidate's failing byte-count regression was
never accepted. A candidate regression test and implementation remain in the
saved patch for investigation. The release binary was never replaced with the
candidate. The dedicated terminal window was closed.

## How this code produces an animation

`types.rs` defines `Cell { ch, fg, bg }` and `Grid = Vec<Vec<Cell>>`: a rectangular
array of characters and colors. A grid cell is one terminal position, with
special handling for characters that occupy two columns.

`opts.rs` runs the demo picker and knob UI. Selecting an animation hands the
mode name, seed and options to the playback machinery.

`morph.rs::IterateFrameRenderer` owns reusable grid memory, a palette and random
number generator. Each frame clears that grid and resets the RNG to the seed.
The mode then computes a fresh picture for the current time and knob values.
The time advances by0.06 per playing iteration. Reused grid memory and a fresh
picture calculation coexist; this mode does not carry a persistent particle
simulation from the previous frame.

`modes/_50_gem_aetherium_2.rs` fills the picture in layers: background/nebula,
rays, stars, gears, armillary rings, limb, orrery, comets and core. Knobs feed
those formulas. Changing time changes positions, rotations and pulse values.
These functions write cells into RAM.

`gridio.rs::AnsiFrameEncoder` keeps the previous picture. It compares cells,
accounts for terminal-width glyphs and forms horizontal groups of changed cells.
It sometimes repaints unchanged gaps to avoid another cursor command. Colors
are mapped into the terminal's indexed palette. The encoder emits an ANSI
String containing move-cursor commands, color changes and UTF-8 characters.
The first frame, or a sufficiently large change, uses a full repaint.

`morph.rs` adds changed UI text and synchronization brackets. It calls
`_1_playback.rs::write_frame` with that buffer. A worker process computes frames;
a supervisor process reads terminal input, forwards ordinary controls, handles
quit, and relays worker output to the terminal. Bounded queues and nonblocking
writes keep pending data bounded, but writes still wait when the consumer is full.

The PTY transports bytes. iTerm interprets the ANSI commands, updates its own
screen cells, and draws font glyphs. A String containing an entire frame can
still contain thousands of commands and one-character spans. Whole-frame byte
assembly does not make the terminal's work one drawing operation.

The measured code currently spends roughly1–3ms building/encoding this scene
and substantially longer delivering it through terminal backpressure. Earlier
iTerm samples locate its rendering work in attributed-string building/drawing.
The attempted byte-cost correction traded printed characters for more cursor
commands and was slower in both controlled terminal comparisons.

[Raw evidence](18_gap_cost/). Repeat with `6_terminal_ab.py --experiment gap
--size 400x200` under the guard, using a fresh directory containing the first
three baseline frame files and corresponding candidate files under `after/`.
Use `10_check_ansi_equivalence.py BEFORE AFTER OUTPUT.json` with pyte0.8.2 to
repeat the independent visible-cell comparison.
