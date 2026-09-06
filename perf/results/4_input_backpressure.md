# Input latency under blocked terminal output

Reproducer: `scripts/4_test_input_latency.py`. Baseline: `038e902`.
All knobs at declared maxima, seed 42, Gem Aetherium 2. Consumer reads pause;
the configured limit is 10 seconds. Input is injected into the PTY.

| Test | Baseline ms | Patched ms |
|---|---:|---:|
| 286x103 knob application | 10006.225 | 2.719 |
| 2000x2000 knob application | unmeasured | 2.664 |
| 286x103 q terminal-mode restoration | prompt, verified with temporary stage markers | 2.614 |
| 2000x2000 q terminal-mode restoration | unmeasured | 2.604 |
| 286x103 q process exit | 10003.318 | 10003.817 |

The baseline knob wait ended when terminal reads resumed. The patched worker
abandons an unfinished frame when a control arrives, applies the control, and
invalidates its encoder for a complete replacement frame.

Quit was already received by the supervisor promptly. Temporary stage markers
confirmed worker termination and terminal restoration completed before output
resumed. Process exit still waited for PTY consumption. The test reports quit
restoration and process-exit latency separately to avoid treating terminal
teardown as time spent handling the key.

A separate tmux-server baseline applied the knob in 9.302 ms despite the paused
outer reader. Its terminal restoration took 2.596 ms; client process exit waited
10003.965 ms. That path buffers/consumes output differently from a direct PTY.

The reported setup uses a WebView terminal through tmux, with a 320x104 pane
and 286x103 render area. PTY injection bypasses WebView keyboard dispatch and
painting. These measurements reproduce renderer backpressure and terminal
teardown; they do not establish the cause of a ten-second GUI keyboard/display
stall.

An isolated headless WebKit replay used the host application's terminal fixture
and 60 full ANSI frames (17,057,005 bytes) with the screenshot's knob values.
A WebView timer pacing writes at 16 ms intervals completed in 14,726 ms, with
411 ms maximum heartbeat gaps, 402 ms maximum write-callback latency, and
428 ms keyboard dispatch. An independent producer queuing the same frames at
16 ms intervals completed in 2,476 ms, with 550 ms heartbeat gaps, 1,292 ms
write-callback latency, and 32 ms keyboard dispatch. Batching changes these
results. Neither run reproduced a ten-second keyboard-dispatch delay. These
probes exclude the native IPC bridge and the owner's other open terminal tabs.

Final validation: 422 unit tests, 3 integration tests, and 186 mode snapshots
passed. The 2000x2000 simultaneous-max PTY run completed 100 frames in 36.828 s,
with median generation 29.631 ms, encoding 66.715 ms, presentation 252.309 ms,
and total 349.686 ms. Random jumps and switching between native and morph
strategies passed at 320x103. No terminal-consumer speedup is implied by the
control-latency repair.
