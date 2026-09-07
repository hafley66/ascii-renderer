# What the animation asks the PTY and terminal to do

Actual measured run: 400x200 terminal, 366x199 art grid, gem-aetherium-2 seed42,
fixed roll-6 knobs. Eight art frames were reconstructed with the production
renderer and encoder from the exact recorded times, knobs and palette in
16_pacing/after-direct/animation.ndjson. Export ran under the existing watchdog
into files; no additional terminal stress was launched. The bounded test exporter
now accepts up to 80000 cells to cover the user's requested terminal dimensions.

## One delta, frame 2 at animation time 0.12

| Requested operation | Count |
| --- | ---: |
| Changed grid cells from production telemetry | 12852 of 72834 |
| Printable Unicode scalar values emitted | 16656 |
| Foreground-color commands | 5885 |
| Absolute cursor moves | 199 |
| Horizontal cursor moves | 4563 |
| SGR resets | 1 |
| Distinct foreground colors | 31 |
| Printable spans between commands | 7561 |
| Single-character spans | 5160 |
| Median printable span length | 1 character |

The art stream is 127209 bytes. Foreground commands consume 61437 bytes,
cursor moves 21027, final reset 4, and printed text 44741. Thus 64.83% of the
art payload is control syntax. The logged complete frame is 127225 bytes;
the remaining 16 bytes are the synchronized-output brackets. The panel and
status contribute no other bytes for this frame.

Actual stream prefix, with ESC rendered visibly:

```text
ESC[1;58H ESC[38;5;113m │     ESC[37C ESC[38;5;187m ˙ ESC[19C
```

Spaces separating command descriptions above are editorial except the five
spaces after the vertical line. Raw payloads are retained for exact inspection.
Interpretation: move to row1/column58, select color113, print a line and five
spaces, move right37, select color187, print a dot, move right19.

Frequent printed characters include ▒ (4075), ┊ (3580), spaces (2445), ░ (1128),
▓ (947), ─ (885), ┆ (803), │ (753), and ⚙ (518). Counts describe this frame only.
Printable scalar counts differ from display-cell counts for double-width glyphs.

## Across eight frames

Frame1 is a full repaint: 72832 printable scalars, 7835 foreground commands,
199 absolute cursor moves, 270393 art bytes, and 6139 additional UI/sync bytes.
Frames2–8 are retained deltas: 5735–6089 foreground changes and 4459–4762 total
cursor moves per frame. All199 art rows receive an absolute positioning command.
No background-setting CSI or per-delta full clear occurs in the reconstructed
art payload. These frames end with an SGR reset. UI/sync overhead is 16 bytes
for frames2–7 and 34 bytes for frame8.

`collect_dirty_runs` absorbs unchanged gaps when their encoded bytes cost no
more than an absolute cursor command. `encode_runs` then uses shorter horizontal
commands where possible. The gap cost decision still prices the absolute form.
`encode_span` changes foreground whenever the next visible glyph requires a
different indexed color and appends the glyph. This produces interleaved
short text spans, color commands and cursor commands. Complete-frame String
assembly does not eliminate those operations in the terminal parser/drawer.

## The actual relay boundary

In the actual live run's 595454 us final relay interval:

| Counter | Value |
| --- | ---: |
| Bytes accepted by terminal writes | 1378310 |
| Write attempts | 6389 |
| Would-block results | 5042 (78.92%) |
| Successful writes | 1347 |
| Mean accepted bytes per successful write | 1023.24 |
| Maximum accepted bytes per write | 1024 |
| Worker-pipe reads | 23 |
| Maximum worker-pipe read | 65536 bytes |
| Time inside terminal write calls | 4155 us |
| Waiting for terminal output capacity | 576114 us |

The relay interval includes startup/cleanup and a partially delivered next
frame; it is not the sum of the eight reconstructed art payloads. It provides
measured write boundaries, not kernel syscall traces. The relay passes its
remaining byte slice to write, accepts partial results and retries after
readiness/backoff. It does not intentionally issue one syscall per character.

Worker frame String → worker pipe → up-to-64KiB relay chunks → PTY accepts
about1KiB at a time → terminal interprets thousands of interleaved ANSI and
printable-text operations → terminal draws text. Existing iTerm samples identify
attributed-string construction/drawing as terminal-side work; this decoding
counts requested operations and does not measure each command's execution cost.

## Evidence

[Raw frames, inputs and command counts](17_pty_work/).
`scripts/9_decode_ansi_work.py` tokenizes every byte of every frame, verifies
complete token coverage and byte-accounting equality, and reports each frame.
The ignored production exporter passed under its watchdog; Python compilation
and byte-accounting checks passed. Production rendering behavior was unchanged.
