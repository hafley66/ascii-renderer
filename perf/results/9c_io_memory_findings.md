# Repeated calls, allocations, and memory growth

All dedicated renderer/profiler processes were stopped. No further renderer
stress was run after the user requested time and memory circuit breakers.

## Foreground run: measured I/O

Same binary and all-max inputs as the preceding profile. The probe interposes
libc `read`, `write`, and `poll` in newly launched renderer processes only.
These are API-boundary call/result counters, not kernel syscall stack traces.
The counter logger does not record terminal payloads. It excludes its own log
writes and samples cumulative counters once per second; concurrent counters
can differ slightly at snapshot boundaries. File descriptors can be reused;
the table below uses an interval inside the animation with stable descriptors.

From epoch 1788720517212 through 1788720532268, a 15.056-second interval:

| Supervisor terminal writes | Measurement |
| --- | ---: |
| Calls | 968,823 |
| Calls per second | 64,348 |
| EAGAIN results | 885,346, or 91.38% |
| Successful calls | 83,477 |
| Partial successful writes | 80,853 |
| Bytes accepted per successful call, average | 1,023.8 |
| Bytes accepted per second | 5,676,494 |
| Time inside write calls, total | 791.5 ms |

The successful worker-to-supervisor pipe writes averaged 32,569.6 bytes. The
terminal accepted much smaller fragments. The current supervisor retries a
write after `poll` reports writable, with a 25-microsecond backoff after eight
consecutive failures. Its readiness polling repeatedly reports writable during
these failed writes. This accounts for repeated OS calls in our output relay.
The write-call duration itself averages about 52.6 ms per elapsed second in
this interval, so those calls do not explain the entire multi-second UI stall.

Counters distinguish retried requested bytes from accepted bytes. Summed
requested lengths include the same pending data repeatedly and are not an
allocation or transferred-byte measurement.

## Memory

Renderer worker memory maps, early and late in the foreground run:

| Metric | Early | Late |
| --- | ---: | ---: |
| Physical footprint | 22.3 MiB | 25.6 MiB |
| Live malloc allocations | 840 | 840 |
| Live allocated heap | 19.6 MiB | 19.6 MiB |
| Resident allocator pages | 20.1 MiB | 23.3 MiB |

The maps show stable live heap at those two points with increased resident
allocator pages. They do not count cumulative allocation/free churn between
snapshots. Mapping can pause a process; the separate longer run disabled it.

iTerm2 RSS rose from 201.1 MiB at the foreground probe's start to a measured peak
of 1,171.6 MiB. After closing that dedicated window, a separate RSS read was
478.4 MiB. iTerm2 is a shared process, so these are whole-process size counters;
the specific owning allocations have not been identified.

The subsequent animation ran for 60 seconds but its q automation failed the
foreground guard. Its iTerm2 RSS ranged from 409.3 to 1,201.1 MiB, ending at
754.4 MiB. The renderer worker ranged from 24.4 to 28.7 MiB. Foreground continuity
was not verified, so this run cannot establish a foreground memory plateau.
It was stopped and its dedicated window closed. An earlier attempt aborted
before animation after losing focus and is excluded from these measurements.

## Repeated work found in the sampled iTerm2 source

The [3.6.3 attributed-string builder](https://github.com/gnachman/iTerm2/blob/v3.6.3/sources/iTermAttributedStringBuilder.m#L327)
starts and accumulates timers inside its character loop. Attribute changes can
finish the current segment, allocate another builder, and construct an attribute
dictionary. The code therefore repeats allocation work per segment, with segment
boundaries affected by character attributes. Existing stack samples put 42.8%
of main-thread observations inside attributed-string construction.

The [timer implementation](https://github.com/gnachman/iTerm2/blob/v3.6.3/sources/iTermPreciseTimer.m#L131)
acquires an Objective-C synchronized lock before testing whether timers are
enabled. When enabled, wrapper calls enter further synchronized timer functions.
The existing samples measured 13.0% of main-thread observations under these
timer paths. This cost is included within drawing and must not be added to it.

[String conversion](https://github.com/gnachman/iTerm2/blob/v3.6.3/sources/ScreenChar.m#L472)
looks up the bidi preference on each conversion invocation. The sampled terminal
mutation path includes this preference lookup under string conversion. This
establishes a repeated lookup site; no exact invocation count was collected.

These findings identify relay retry amplification and terminal per-character /
per-segment work. They do not establish a quadratic rendering algorithm or a
specific iTerm2 memory leak. The earlier drawing stacks remain the evidence for
where the foreground thread spends time.

## Circuit breakers added after the run

`scripts/5_probe_guard.py` now wraps the I/O probe before the renderer starts.
It runs independently of the GUI observer, checks every 100 ms, and stops only
the launched process tree. It never signals the watched iTerm2 process.

Defaults: 15 seconds total wall time; 256 MiB owned RSS; 768 MiB watched RSS;
128 MiB watched RSS growth; 32 MiB artifacts; at least 2 GiB free disk.
Observer-owner exit and monitoring failures also stop the probe. Cleanup uses
OS signals and restores terminal attributes without waiting for a GUI response.
The observer checks the breaker before sending further keys.

These are polled trip thresholds with possible overshoot, not instantaneous
memory quotas. Existing historical repro scripts predate this guard and must
not be run directly for additional stress. The repository instructions now
require a watchdog for live probes and prohibit extending stress or raising
limits without user direction.

Validation used three low-load tests: threshold/ownership logic; termination
of a sleeping child and grandchild after one second; and a one-MiB RSS threshold
against a small Python child. All passed. No GUI stress was run to validate the
new guard.

## Artifacts

- [Derived I/O rates and memory timeline](9_iterm_io_1/io-summary.json)
- [Supervisor counters](9_iterm_io_1/io-91825.ndjson)
- [Worker counters](9_iterm_io_1/io-92166.ndjson)
- [Early worker memory map](9_iterm_io_1/vmmap-92166-early.txt)
- [Late worker memory map](9_iterm_io_1/vmmap-92166-late.txt)
- [Foreground-run RSS measurements](9_iterm_io_1/memory.ndjson)
- [Long-run RSS measurements, foreground continuity unverified](9_iterm_io_60s_retry/memory.ndjson)
- [Prior sampled call stacks](8b_raw_iterm_profile.md)

Automatic approval review rejected attaching syscall tracing to the shared
iTerm2 process because it could expose or disrupt unrelated terminal sessions.
No such attachment was performed. Noninteractive sudo also required a password.
The renderer-only libc counters and read-only RSS counters were the permitted
alternative; kernel tracing remains unavailable in this investigation.
