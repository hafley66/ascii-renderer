# Sustained terminal-consumer replay

The renderer's default slow log had no new records after epoch
1788712269285. Its last animation frame reported 50.812 ms total at 286x103.
That log contained no timing evidence for the subsequently reported GUI stall.

The host terminal path reads PTY chunks of up to 8192 bytes, batches them in
Rust, emits a WebView event, observes output, then calls xterm.write. The Rust
queue is bounded before event emission; there is no consumption acknowledgment
from xterm's callback to that queue.

An isolated WebKit test used the host terminal fixture and screenshot knob
values at 286x103. It fed the observer and xterm in 8192-byte UTF-8 chunks.
These are full-frame replay probes, not captures of the user's live IPC stream.

| Replay | Result |
|---|---|
| 60 frames, independent producer at 16 ms intervals | 2.608 s completion; observer 64 ms total; max parse latency 1430 ms; keyboard dispatch 39 ms |
| 600-frame sustained producer | 216 write-discard errors; not all frames completed within 45 s; keyboard dispatch 198 ms |
| Producer waits for xterm write completion for 10 s | 51/51 frames completed; one maximum pending frame; no discarded-write errors; max parse latency 244 ms; keyboard dispatch 150 ms |

The installed xterm WriteBuffer rejects writes when pending data exceeds
50,000,000 units (DISCARD_WATERMARK in src/common/input/WriteBuffer.ts).
The reported error was `write data discarded, use flow control to avoid losing data`.
This reproduces terminal queue overflow and output loss. The exact ten-second
GUI keyboard-dispatch delay remains unconfirmed.

The acknowledgment prototype still observed a 471 ms maximum main-thread
heartbeat gap. Backpressure prevents accumulating and dropping output; it does
not remove terminal parsing or painting cost. A production bridge repair must
bound in-flight output through consumption acknowledgments, handle closed tabs
and reloads, and retain an independent input path.

Renderer diagnostics now sample the first completed frame and once per second,
including input state, PID, frame index, interval duration, frame count, and byte
count, even when no render exceeds the slow threshold.
