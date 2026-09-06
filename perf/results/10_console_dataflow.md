# Rust console output: dataflow audit and renderer comparison

Date: 2026-09-06. This investigation used source reads and existing traces.
No renderer, GUI probe, benchmark, or compiler stress was launched. No renderer
implementation changes accompany this report.

## Current path and ownership

```text
supervisor: terminal events -> bounded event queue -> worker stdin
worker: events -> parameter state -> Grid -> AnsiFrameEncoder -> String
worker: String -> write_frame -> 32 KiB pipe writes
supervisor: pipe read -> retained byte chunk -> nonblocking terminal writes
iTerm2: terminal tokens -> screen state -> main-thread text drawing
```

`AnsiFrameEncoder::encode(&mut self, grid: &Grid, force_full: bool,
output: &mut String) -> FrameEncodeStats` owns previous cells and dirty-run
scratch for one animation session. It advances its previous-cell buffer during
encoding, before output succeeds. `write_frame` returns whether it completed;
an interruption invalidates the encoder, making the next frame a full repaint.

`pump` owns the child process, input forwarding, and one pending output chunk.
Its interface carries bytes, with no frame ID or commit boundary. Input is
checked independently, but the terminal app must first deliver the input.
Successful writes and `POLLOUT` establish stream progress, not completed painting.

The recorded steady case had 43 frames, median 184,423 changed cells and 18,292
dirty runs per frame. Only the first frame was a full repaint. The existing diff
is therefore already active during the measured slowdown.

## N+1 sites found

| Site | Observed shape | Effect and proposed change |
| --- | --- | --- |
| `src/_1_playback.rs:227` | Write, failed write, readiness poll, retry | 64,348 terminal writes/sec, 91.38% EAGAIN in the prior instrumented interval. After false readiness, impose an actual retry deadline while continuing input checks. Measure retry count separately from throughput. |
| `src/morph.rs:1235` and `src/_1_playback.rs:352` | A frame becomes an unframed stream | No synchronized presentation bracket. Use the existing Crossterm begin/end commands at the complete presentation boundary, including error/cancellation cleanup. |
| `src/_1_playback.rs:358` and `src/morph.rs:1318` | Any forwarded event aborts output, then invalidates all previous cells | Even panel selection, ignored keys, and clamped knob changes can restart a full frame. Apply pending events as a batch and classify whether final visual state changed before cancelling output. |
| `src/gridio.rs:331` | Structural cell inequality drives dirty marking | Two plain spaces with equal backgrounds but different foreground colors are visually equal in this Cell model, yet trigger output. Compare visible state for blank cells, with wide-glyph invalidation preserved. Frequency in the recorded art has not been counted. |
| `src/gridio.rs:506` | Foreground changes are emitted for spaces | Omit invisible foreground transitions until a visible glyph requires them. Track emitted attributes, not merely requested cell attributes. |
| `src/morph.rs:1280`, `src/registry.rs:1341` | Panel output is appended after the art diff | The unchanged divider is emitted once per row, 347 times per frame at the repro dimensions. The loop also creates one temporary formatted String per divider row. Put the panel in retained composition or cache its output state; use `write!` into the existing String for formatting. |
| `src/morph.rs:1268` | Whole status row rewritten every frame | Time text changes, but unchanged padding is also resent. Include status in the final-screen diff. |
| `src/opts.rs:369` -> `src/opts.rs:262` | Each knob key serializes the entire options map and creates/replaces a file | Apply a received input batch first, then persist once when its final state changed. Preserve atomic replacement and saving on exit. |
| `src/gridio.rs:545` | Every dirty run gets absolute cursor positioning | Track cursor position and choose relative/column/absolute commands by encoded cost, as the compared presenter does. The 18,292-run median makes this measurable, but it does not remove glyph drawing work. |

The input-cancellation finding concerns interaction while bytes are in flight.
It does not explain full repaint amplification in the steady, no-input trace,
which contained only one full repaint.

## Rust renderers inspected

### Ratatui

[Terminal](https://docs.rs/ratatui/latest/ratatui/struct.Terminal.html) composes
widgets into a frame buffer, diffs against the previous buffer, and sends changed
cells to its backend. That allows artwork, panel, and status to share one diff.
The [Crossterm backend source](https://github.com/ratatui/ratatui/blob/main/ratatui-crossterm/src/lib.rs)
tracks color/modifier state and the previous glyph's position/width to avoid
redundant cursor moves. It accepts a generic writer. A caller must choose
buffering appropriate to that writer; queueing commands is not a promise of
one kernel write or one terminal repaint.

Integration boundary: adapt the existing Grid plus UI into a Ratatui Buffer.
Its complete terminal lifecycle includes cursor and backend flush handling.
No performance result for this art workload was measured.

### Termwiz / WezTerm

[Surface](https://docs.rs/termwiz/latest/termwiz/surface/struct.Surface.html)
retains screen state and a sequence-numbered change log. It can provide an
optimized sequence of changes and prune consumed history.
[BufferedTerminal::flush](https://github.com/wezterm/wezterm/blob/main/termwiz/src/terminal/buffered.rs)
invalidates its sequence before rendering, commits the returned sequence on
success, and then prunes old changes. On error, the sequence remains invalid
so the next render repairs the display.

[TerminfoRenderer](https://github.com/wezterm/wezterm/blob/main/termwiz/src/render/terminfo.rs)
accumulates pending attributes and emits them when needed for text. Its changes
include region-clearing operations. The inspected code explicitly notes a
background-color-erase limitation in its clear-to-end-of-line path, which matters
for this application's colored backgrounds.

Integration boundary: Grid -> Surface/Change -> terminal renderer. This directly
matches the application's cell-oriented output. A comparison should use those
library APIs and bound change-log retention; no backend speedup is established.

### FrankenTUI `ftui-render`

The inspected [Presenter source](https://github.com/Dicklesworthstone/frankentui/blob/main/crates/ftui-render/src/presenter.rs)
groups diff runs, retains scratch storage, uses a 64 KiB BufWriter, tracks cursor
and style, chooses shorter cursor commands, and compares style-delta cost with
reset/reapply cost. It brackets presentation with synchronized updates when its
capability policy allows, attempts cleanup after emission failure, and flushes.
These are concrete output techniques visible in source, independent of the
project's performance claims.

Its [crate manifest](https://github.com/Dicklesworthstone/frankentui/blob/main/crates/ftui-render/Cargo.toml)
declares a standalone render crate and the license identifier
`LicenseRef-MIT-OpenAI-Anthropic-Rider`. No code from that crate was copied here.

### Codex's Ratatui-based terminal

[The custom terminal source](https://github.com/openai/codex/blob/main/codex-rs/tui/src/custom_terminal.rs)
finds uniform blank row tails and represents them with a clear-to-end command.
Its logic accounts for background differences, modifiers, forced updates, and
wide glyphs crossing the clear boundary. Those conditions matter when adapting
the technique. Art occupies a subrectangle beside the options panel, so clearing
to the physical line end there would also erase the panel. Compose the full row
first or use a bounded erase operation.

## Cost model per effect

| Effect | Fixed / repeated cost | Variable cost | Batch boundary | Latency policy |
| --- | --- | --- | --- | --- |
| Terminal writes | Measured 791.5 ms inside 968,823 writes over 15.056 s | Accepted bytes, parser work | Existing 32 KiB pending block; no measured optimal chunk size | Retry deadline after failed readiness; continue input polling |
| Terminal painting | Existing samples show 86.1% of main-thread observations under text drawing | Visible cells and attributed-string runs | One complete synchronized presentation | Capability-gated, bounded, cleanup on every exit |
| Knob updates / persistence | Whole-map formatting, temporary file, replacement per key | Map size | Already-pending input batch | Apply keys in order, preserve net state, save once |
| UI formatting | Per-row temporary String and repeated ANSI | Panel height and changed UI content | Retained panel/status state | Re-emit only damaged/changed content |
| Geometry | About 5.8 ms median in prior profile | Mode computation and cell count | Existing frame renderer | No added batching proposed |

Synchronized output asks the terminal to defer visible updates while processing
the frame. It does not compress the ANSI stream or acknowledge painting.
Terminals can time out synchronization, so a bracket is not an unconditional
atomicity guarantee for arbitrarily long frames. Crossterm **0.26.1 already in
this repository** has `BeginSynchronizedUpdate` and `EndSynchronizedUpdate`.
The serialized bracket pair is 16 bytes total.

Our supervisor can kill a worker mid-frame. A worker-only drop guard cannot
close a bracket after SIGKILL. The supervisor must attempt end-sync during
cleanup and a future frame must repair interrupted state. Its nonblocking
cleanup writes also need bounded partial-write handling. Emitting begin/end
around each 32 KiB chunk would create many presentation boundaries per frame.

## Partition and implementation order

1. Use the existing Crossterm synchronization API at the full-frame boundary,
   including supervisor cleanup. Measure its effect on terminal paint work.
2. Retain a fully composed screen, including panel/status. Evaluate Termwiz's
   Surface/renderer boundary and Ratatui's composed Buffer against the same
   deterministic frames; record bytes, controls, visible-cell equality, and
   allocations. Keep the candidate adapter outside the mode implementations.
3. Coalesce pending input before rendering/persistence, avoid cancellation for
   events that leave relevant state unchanged, and enforce an output retry
   deadline after EAGAIN. Keep quit priority in the supervisor.
4. Add blank-cell equivalence, bounded blank-region clearing, and cursor/style
   cost selection through the chosen presentation path.

Expected savings: redundant terminal updates, readiness retries, temporary UI
formatting allocations, and repeated saves. Actual improvement remains to be
measured. Larger byte queues add retained data without making iTerm2 consume
it faster. More geometry worker threads do not address the measured presentation
stall. The existing reusable String already batches ANSI construction.

## Frame state and policy

Retain at most one active presentation and one newest unencoded target state.
Do not queue a history of full grids. A new target replaces only pending work;
already-transmitted ANSI remains part of terminal state. A delta is valid only
against its actual base, so arbitrary encoded frames/chunks cannot be dropped.
Commit retained presentation state after complete stream emission; use explicit
invalidation after partial emission. This is stream commitment, not paint ACK.

| Effect | Window | In flight | Maximum pending | Leading |
| --- | --- | --- | --- | --- |
| Input state reduction | Drain currently available bounded input | 1 reducer | Existing bounded controls | Yes |
| Presentation | Complete available target; no extra batching delay | 1 frame | 1 newest unencoded state | Yes |
| Persistence | End of current input batch | 1 atomic replacement | 1 latest map | Apply UI immediately |
| Retry after false readiness | Candidate 1 ms deadline, not yet measured | 1 pending chunk | Existing bounded byte storage | Resume on eligible attempt |

For B = width * height * size_of(Cell), two retained cell buffers cost about 2B,
plus one bounded encoded frame and backend metadata. Adding a pending full Grid
raises this to roughly 3B. Library cell sizes and allocations must be measured
before claiming these layouts fit the 256 MiB probe budget. A changed-cell
buffer can also approach a full screen, so it needs a byte cap, not only a
frame-count cap.

The current native clock advances by 0.06 per loop in `src/morph.rs:1173`.
Output congestion therefore slows animation progression as well as presentation.
Any later switch to elapsed-time animation needs to preserve explicit replay
timestamps and pause/step behavior; it should be validated separately.

## Verification without another stress run

First compare encodings in memory on small fixed grids: RGB glyphs, invisible
foreground-only changes on spaces, colored blank runs, wide-glyph replacement,
and an unchanged options panel. Use a terminal parser library to compare final
visible cells, rather than checking ANSI strings alone. Mock partial writes,
EAGAIN/false readiness, cancellation, and begin/end cleanup. Count actual writer
calls as well as bytes, since a library flush can contain multiple writes.

Live validation remains subject to `scripts/5_probe_guard.py` and the existing
time/RSS/artifact thresholds. There is no measured library winner yet.
