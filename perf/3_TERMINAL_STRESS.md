# Terminal stress animation

## Independent reroll

`examples/2_fresh_animation.rs` is a separate animation executable. Its path is a parametric colored wave written directly into Ratatui's buffer, then Ratatui's diff and Crossterm backend write to a 64 KiB buffered stdout. It does not import the application's renderer, registry, playback supervisor, or frame relay.

```sh
cargo run --release --example 2_fresh_animation -- 300 /tmp/fresh-animation.ndjson
```

Press `q` or Escape to exit. The example accepts at most 300 frames and 80,000 terminal cells. A blocked terminal write can extend its runtime; automated probes must still use the external watchdog.

The guarded native 400×200, 12-frame run measured 8.107 ms median and 18.257 ms maximum draw time after the first frame. This is a fresh-pattern baseline, with different content and byte volume from Gem 2. It does not establish equivalent visual performance. [Recorded result](results/38_fresh_animation/summary.json).

## Demo load generator

Run `cargo run --release -- 42 demo`, select `terminal-stress` at the end of the mode list, and press `a` to animate. The mode uses the existing knob controls, input recording, and seed handling.

Density, churn, color count, background coverage, frequency, speed, glyph set, and pattern control terminal output load. The renderer performs one pass over the grid with a fixed sine lookup table. The maximum preset changes colored Unicode cells across the full art area.

Run the guarded real-demo comparison:

```sh
scripts/13_e2e.sh --headless --case bad-400x200
scripts/13_e2e.sh --headless --mode terminal-stress --case max-400x200
```

The harness uses portable-pty and vt100 in Rust for terminal bytes and parsing. Python orchestrates controls and assertions. Colors are explicitly enabled and verified in captured cells. Probes retain the external watchdog, 15-second wall limit, 256 MiB owned-process limit, and 32 MiB artifact limit.

Measured with a 400×200 terminal and 366×199 art grid, seed 42:

| Mode/configuration | Median frame interval | Median bytes/frame |
| --- | ---: | ---: |
| gem-aetherium-2, historical bad-roll6 | 16.25 ms | 159,373 |
| terminal-stress, every knob at maximum | 61.28 ms | 1,749,898 |

Each interval sample covers steady frames 2–10. Both runs passed the functional E2E checks. Maximum stress exceeds a 60 FPS budget. These measurements include native PTY consumption and terminal parsing; GUI terminal painting remains unmeasured. Exact inputs, binary hash, stage timings, and checks are in [the comparison record](results/37_native_comparison.json).

The isolated dense-color Termion lab measured 12.24 ms median with the native consumer, compared with the earlier 323.42 ms Python/pyte consumer. All four native lab patterns produced matching final terminal cells between the two library backends.
