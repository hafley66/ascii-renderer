# Raw iTerm2 slowdown: sampled call stacks

Run: 2026-09-06, iTerm2 3.6.3, macOS 14.6.1. Same release executable as the
[previous GUI repro](7_raw_iterm_repro.md), SHA256
`6a690731f4b73ea0a0cd822bbf2bb85800f4da769023d3bb5bcc40c565f671d5`.
Command: `cargo run --release -- 42 demo`, seed 42, gem-aetherium-2, all twelve
knobs at their declared maxima, randomization off. Raw iTerm2, terminal 1718x348,
animation grid 1684x347. No recording or screenshots before q reached the app.

## Sampled attribution

Four processes were sampled simultaneously with `/usr/bin/sample PID DURATION 1
-mayDie -file PATH`: a six-second steady-animation interval and an eight-second
interval spanning the q request and its delayed receipt. The animation worker
exited during the latter interval, so its sample ended early. All eight sample
commands and the GUI observer exited successfully.

Counts below are inclusive stack observations for the named thread or queue.
They measure sampled wall time, include waits, and are not percentages of total
machine CPU. Nested categories overlap. The analyzer counts each observation
once per category even when matching functions nest or recur.

| Thread / path | Steady animation | Across q |
| --- | ---: | ---: |
| iTerm2 main: `PTYTextView drawRect:inView:` | 4264/4952 (86.1%) | 5670/6621 (85.6%) |
| iTerm2 main: build attributed strings | 2117/4952 (42.8%) | 2849/6621 (43.0%) |
| iTerm2 main: draw attributed strings | 2110/4952 (42.6%) | 2764/6621 (41.7%) |
| iTerm2 main: nested `iTermPreciseTimer*` instrumentation | 645/4952 (13.0%) | 874/6621 (13.2%) |
| iTerm2 mutation queue: `terminalAppendString:` | 4089/4864 (84.1%) | 4271/5103 (83.7%) |
| Renderer worker: `write_frame` | 4929/5345 (92.2%) | 4851/5225 (92.8%) |

The measured main-thread drawing path is:

```text
AppKit run-loop observer
  CA::Transaction::commit
    NSViewBackingLayer display
      iTermLegacyView drawRect:
        SessionView legacyView:drawRect:
          PTYTextView drawRect:inView:
            iTermTextDrawingHelper drawCharactersForDisplayLine:...
              constructAndDrawRunsForLine:...
                iTermAttributedStringBuilder attributedStringsForLine:...
                drawMultipartAttributedString:...
                  drawFastPathString:...
```

The legacy drawing path is directly observed in the stacks. These samples do
not establish why that backend was selected. Attributed-string construction
includes Objective-C messaging, attribute dictionaries, allocation, and timer
instrumentation. The nested timers account for part of the drawing cost; their
counts are already included above. No iTerm preferences were changed to test
their individual contribution.

The terminal mutation queue is executing
`TokenExecutorImpl.execute` -> `VT100Terminal executeToken:` ->
`terminalAppendString:` -> `appendStringAtCursor:`. This is an additional
terminal-side workload concurrent with the main-thread drawing.

On the renderer worker, 4912/5345 steady samples are specifically in
`morph_worker_session` -> `write_frame` -> `poll`, waiting for output-pipe
capacity. On the supervisor, 4301/5318 steady observations fall under the
retry sleep reached after the terminal rejects output writes, and another
549 are in `poll`. The sampled iTermServer process was in `select` throughout
its steady sample; its participation in this dedicated session's data path
was not independently established.

## Frame and input measurements

The 43 completed animation frames logged:

| Stage | Median | Maximum |
| --- | ---: | ---: |
| Geometry generation | 5.844 ms | 8.818 ms |
| ANSI encoding | 11.865 ms | 24.020 ms |
| Worker output emission, including backpressure | 466.082 ms | 516.470 ms |
| Complete measured frame | 482.939 ms | 538.478 ms |
| Encoded bytes | 2,827,341 | 3,816,533 |

`presentation_us` is the worker's output-write duration. It does not measure
completed terminal painting. Median columns do not add because each is computed
independently. Total encoded output for completed frames was 122,448,747 bytes.

| q event | Unix epoch ms |
| --- | ---: |
| GUI automation request began | 1788719762794 |
| GUI automation call returned | 1788719763689 |
| Renderer supervisor received q | 1788719766114 |
| Worker stopped | 1788719766115 |

This reproduces **3.320 seconds** from automation request to app receipt,
including **2.425 seconds after the key-posting call had already returned**.
The worker stop operation took **578 microseconds**. The request begins before
the automation window check, so it is not a physical keyboard timestamp.
The maximum recorded supervisor input-poll gap during animation was 26.925 ms.

The samples attribute the sustained foreground stall to terminal text drawing
on iTerm2's event-handling thread, alongside terminal token execution and
renderer output backpressure. Aggregated samples do not divide the exact q
delay into individual event-queue or paint intervals. Existing uncaptured runs
also reproduced multi-second delays; sampling overhead can affect this run's
absolute timings.

## Evidence and repeat command

- [Steady iTerm2 stacks](8_iterm_profile_1/steady_iterm.sample.txt)
- [iTerm2 stacks spanning q](8_iterm_profile_1/quit_iterm.sample.txt)
- [Steady renderer worker stacks](8_iterm_profile_1/steady_worker.sample.txt)
- [Steady supervisor stacks](8_iterm_profile_1/steady_supervisor.sample.txt)
- [All derived thread counts and measurements](8_iterm_profile_1/summary.json)
- [Frame and event trace](8_iterm_profile_1/animation.ndjson)
- [GUI request timestamps](8_iterm_profile_1/ui-events.ndjson)
- [Sampler commands and timestamps](8_iterm_profile_1/samples.ndjson)
- [Binary, process IDs, dimensions](8_iterm_profile_1/metadata.json)

```bash
profile_window=$(osascript perf/results/7_prepare_iterm.applescript)
python3 perf/results/8_profile_iterm.py --window-id "$profile_window" \
  --output "perf/results/8_iterm_profile_$(date +%s)"
# Run the analyzer with that output directory:
python3 perf/results/8a_analyze_iterm.py perf/results/8_iterm_profile_1
```

The setup creates a dedicated window and uses session-local zoom. Keep that
window focused while the observer runs. The observer sends q to stop animation,
then q to exit demo. The wrapper then waits for one character; release it before
closing that dedicated window. The recorded profiling window was closed after
completion. No renderer implementation changes were made for this profile.
