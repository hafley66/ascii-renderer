# Raw iTerm2 reproduction: delayed controls and visible animation

Reproduced on 2026-09-06 using the actual iTerm2 application, a visible raw
terminal session, and `cargo run --release -- 42 demo`. Select
`gem-aetherium-2`, press `a`, hold every declared knob at maximum, then press `q`.

No renderer or transport changes were made during this reproduction task.
The release binary SHA-256 stayed:
`6a690731f4b73ea0a0cd822bbf2bb85800f4da769023d3bb5bcc40c565f671d5`.

## Reproduced measurements

| Measurement | Screen recording enabled | Screen recording disabled |
| --- | ---: | ---: |
| Scripted UI q request to renderer receiving q | 4,000 ms | 4,058 ms |
| UI keystroke call already returned to renderer receiving q | 2,079 ms | 2,012 ms |
| Renderer receiving q to animation worker stopped | 1 ms | 2 ms |
| Completed animation frames recorded | 48 | 48 |

The request interval includes an AppleScript check of the selected iTerm2
window and the System Events keystroke call. It is not a timestamp from a
physical keyboard. The second row is a conservative lower bound on delay
after the UI keystroke call had already completed.

With recording disabled, no screenshot was taken before the quit event reached
the renderer. The first screenshot was taken after the worker stopped. This
pass still reproduced the multi-second input delay.

The recorded pass's screenshot at 6.7 seconds after the first q request still
shows the animation options panel and old animation content. The animation
worker had stopped at +4.001 seconds and the demo preview output completed at
+4.552 seconds. A later unrecorded-pass screenshot shows the settled demo with
the animation panel gone. The screenshot called `demo-returned.png` is named
by the harness's process-state assumption; its pixels still show the old
animation. Do not use that filename as evidence that the new UI was visible.

These runs reproduce multi-second input and visible-output delay. They do not
establish an exact ten-second physical-key-to-screen latency or a root cause.

## Environment and inputs

- Raw iTerm2 window; `TMUX` unset, controlling terminal `/dev/ttys012`.
- Terminal size 1718x348; animation render grid 1684x347.
- Requested size was 1718x358; iTerm2 clamped the height to 348.
- Default profile: Monaco 18 and AnonymicePowerline 18, separate non-ASCII font
  enabled. The test session received 17 Make Text Smaller menu actions.
- Size Changes Update Profile was unchecked. Zoom applied to the test session.
- Seed 42, native iterate, all 12 declared knobs at their maximum, randomization
  disabled. Every recorded animation frame was checked against the maxima.
- Config and logs were isolated in each evidence directory.
- GUI input went through System Events to iTerm2; no terminal byte drainer or
  tmux was inserted. Screen recording and screenshots captured the real display.

## Evidence

- [Short display recording](7_iterm_demo_1788718649884/quit-visible.mp4), an
  unannotated excerpt from seconds 18–35 of the full recording.
- [Full display recording](7_iterm_demo_1788718649884/visible.mov).
- [UI request timestamps](7_iterm_demo_1788718649884/ui-events.ndjson).
- [Renderer, input and relay records](7_iterm_demo_1788718649884/animation.ndjson).
- [Recorded-pass summary](7_iterm_demo_1788718649884/summary.json).
- [Screen still showing animation at +6.7 seconds](7_iterm_demo_1788718649884/demo-returned.png).
- [Recording-disabled summary](7_iterm_uncaptured_1788718894474/summary.json).
- [Recording-disabled UI timestamps](7_iterm_uncaptured_1788718894474/ui-events.ndjson).
- [Recording-disabled renderer trace](7_iterm_uncaptured_1788718894474/animation.ndjson).
- [After worker quit](7_iterm_uncaptured_1788718894474/after-worker-quit.png).
- [Settled demo](7_iterm_uncaptured_1788718894474/settled-demo.png).

The movie's creation-time metadata has second precision. Millisecond claims
above come from the UI and renderer logs, not an assumed movie clock origin.
Screenshot requests and completion timestamps bound capture time separately.

## Repeat without recording overhead

The setup creates a separate raw iTerm2 window and returns its window ID. It
requires the macOS Automation/Accessibility access used during this run. The
script guards against sending keys when the selected window or frontmost app
changes. It leaves its dedicated window open for inspection when complete.
The output directory must be new so stale records cannot satisfy its waits.

```bash
ascii_repro_window=$(osascript perf/results/7_prepare_iterm.applescript)
python3 perf/results/7_iterm_uncaptured_1788718894474/observe.py \
  --window-id "$ascii_repro_window" \
  --output "/tmp/ascii-iterm-repro-$(date +%s)"
```

For a recorded repeat, use the same setup with
`perf/results/7_iterm_demo_1788718649884/observe.py` instead. The observer uses
macOS `screencapture`; Python's standard library handles the input/timing log.
The wrapper holds a final shell `read` after exit. Send one character to return
the dedicated window to its shell, then close that window.

For a manual repeat, use a similarly zoomed raw iTerm2 window:

```bash
XDG_CONFIG_HOME="$PWD/perf/results/7_iterm_uncaptured_1788718894474/config" \
ASCII_TRACE_PATH="/tmp/ascii-iterm-manual-$(date +%s).ndjson" \
ASCII_TRACE_ALL=1 cargo run --release -- 42 demo
```

Press `/`, type `gem-aetherium-2`, press Return, press `a`, wait about 18 seconds,
then press `q` once and observe the return to demo. The supplied config has all
knobs maximal and randomization disabled.
