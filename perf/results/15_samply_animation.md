# Actual demo animation: samply and memory evidence

The requested command ran in a dedicated raw iTerm2 session at 400x200:

```sh
samply record --rate 1000 --duration 15 --save-only \
  --output perf/results/ascii-functions.json.gz \
  -- target/release/ascii-renderer 42 demo
```

Automation used the app's mode picker to select gem-aetherium-2, then `a` to
animate. The options pane reserved 34 columns, yielding a 366x199 art grid.
Seed 42 and exact roll-6 knob values were loaded through an isolated config;
randomization was disabled. Actual palette and every completed frame's inputs
are recorded in animation.ndjson. This covers early animation times 0.06–0.48,
not the earlier retained replay's 9.90-second phase.

The external guard stopped the workload after 2.806 seconds. Shared iTerm RSS
rose from 222480 KiB to 498624 KiB, crossing the user-approved 256 MiB growth
threshold by 14000 KiB between checks. Owned-process RSS was 29408 KiB at that
check. These are whole-process RSS counters, not heap allocation ownership.
The configured sampler duration was 15 seconds; the capture was interrupted.
After killing workload descendants, the watchdog gave samply at most two seconds
to save. Samply saved successfully and exited 0. This is a breaker-terminated
run, not a completed 15-second run. No limits were increased further.

## Eight completed animation frames

| Median stage | Time |
| --- | ---: |
| Geometry | 1.109 ms |
| ANSI encoding | 2.160 ms |
| Output emission including backpressure | 37.469 ms |
| Measured frame work | 40.710 ms |
| Frame interval including pacing | 60.082 ms |

The measured interval corresponds to about 16.6 frames/s. It measures generated
frame cadence, not physical terminal paint completion. Median output was 126417
bytes per frame. The initial full frame is included in these medians.

## Every recorded thread symbolicated

The saved profile contains 29 threads and 279 distinct native addresses. The
local samply symbol server resolved these against the matching binary and system
libraries. `scripts/7_analyze_samply.py` walks every recorded stack in every
thread and saves inclusive, leaf, CPU-delta-at-leaf and full-path tables.

Worker PID 72645 main thread: 343 sample rows, 497 total sample weight:

| Path | Sample weight |
| --- | ---: |
| morph_worker_session → write_frame → poll | 327 |
| morph_worker_session → recv_timeout → semaphore_timedwait_trap | 138 |

Weights include coalesced samples. They are not CPU percentages. CPU deltas are
attributed to the observed endpoint only; they do not reconstruct execution
between samples. The supervisor profile includes both idle demo selection and
animation. Its animation path includes 450 weighted observations in
supervise → sleep → nanosleep; do not divide by whole-profile duration to infer
an animation-only percentage.

Source inspection confirms `morph.rs` unconditionally calls
`receiver.recv_timeout(Duration::from_millis(16))` after presentation when no
input events are pending. That adds pacing time after output backpressure.
`_1_playback.rs` also backs off repeated terminal EAGAIN responses. These paths
explain the app-side waits in the sampled stacks.

## Terminal-side evidence and heap limits

The existing real-animation samples in 8_iterm_profile_1 establish the consuming
path at the earlier, larger terminal dimensions:
PTYTextView drawRect → iTermTextDrawingHelper → attributed-string construction
and drawing. They place 4264/4952 iTerm main-thread observations under drawing,
with 2117 building attributed strings and 2110 drawing them. These nested wall
observations are not new 400x200 CPU measurements.

The newly collected `iterm-vm-summary.txt` records aggregate memory after this
run: 294118 live allocations and 180.9 MiB allocated across allocator zones;
175.6 MiB allocator resident pages. IOSurface had 83.1 MiB resident. Those
categories must not be added to RSS without accounting for shared/reusable
mappings. This summary covers the shared terminal process and does not identify
which allocations belong to this session or establish a leak.

Earlier worker heap snapshots in 9_iterm_io_1 showed 840 live allocations and
19.6 MiB allocated both early and late. No allocation-stack heap recording was
collected in this run. Existing renderer libc counters measured terminal writes
accepting about 1024 bytes and repeated EAGAIN. The new samply recording confirms
output waiting; it does not itself collect syscall counts or terminal CPU stacks.

The identified limits are output consumption/backpressure and post-output
pacing. No production performance change is claimed from this recording.

## Artifacts and validation

All artifacts are in [15_samply_animation](15_samply_animation/): profile,
symbol request/response, all-thread stack summary, frame/event log, watchdog,
aggregate memory map, input config, binary/profile hashes, and automation.
Earlier failed attempts are retained separately. All three low-load watchdog tests, Python compilation, and profile
shape/address/thread-accounting assertions passed. The profiler-save branch was
exercised by two real breaker terminations. No Rust source changed.
