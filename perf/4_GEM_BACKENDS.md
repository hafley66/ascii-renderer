# Gem 2 rendering setups

The three library paths passed native PTY comparisons on 2026-09-07: the recorded fixture at 162×61 and 366×199, and every knob at maximum at 366×199. Each case renders 12 deterministic frames, validates the final colored terminal cells against the scene, and requires identical scenes and captures across libraries. These measurements exclude GUI painting.

| Art grid / inputs | Ratatui | console_engine | Termwiz |
| --- | ---: | ---: | ---: |
| 162×61 recorded | 1.723 ms | 1.566 ms | 3.356 ms |
| 366×199 recorded | 6.609 ms | 5.161 ms | 12.478 ms |
| 366×199 all max | 9.453 ms | 7.144 ms | 15.577 ms |

Values are median total frame work over frames 1–11, excluding initial draw and frame pacing. Art area increases 7.37× between the recorded cases; measured work increases 3.84×, 3.30× and 3.72× respectively. Short runs characterize these inputs, not sustained GUI frame rate. Summaries, hashes and stage timings are preserved in `perf/5_GEM_BACKENDS_RESULTS.json`; raw artifacts remain ignored under `perf/results/46_gem_max_fixed`, `47_gem_small_fixed` and `48_gem_large_fixed`.

The max test initially found 15 missing cells in Ratatui and console_engine, and four in Termwiz. Zodiac symbols occupy two terminal columns, while the source grid stored another character in each trailing column. The adapter now reserves those columns before feeding library buffers, so moving wide glyphs cannot leave falsely unchanged hidden cells. The rerun passed all three backends. Gem 2's source renderer remains unchanged.

The `gem-lab` command calls the existing Gem 2 renderer directly, with explicit seed, palette, time and knobs. It bypasses demo playback and its process relay. Three choices own their own presentation state:

| Argument | Presentation |
| --- | --- |
| `ratatui` | Ratatui retained buffers and Termion backend, buffered stdout |
| `console` | console_engine screen storage, diff and output |
| `termwiz` | Termwiz Surface composition and BufferedTerminal |

Termwiz's [BufferedTerminal API](https://docs.rs/termwiz/0.23.3/termwiz/terminal/buffered/struct.BufferedTerminal.html) performs surface change optimization and terminal rendering on flush.

Build the isolated executable with `scripts/20_build_gem.sh`. The build uses one background job with the authorized 1 GiB watchdog ceiling. It compiles the existing Gem 2 source directly and retains `ascii-renderer` as the default Cargo executable. Command syntax:

```text
target/release/gem-render-lab gem-lab BACKEND FRAMES LOG [INPUTS.json|max] [WIDTH HEIGHT] [hold]
```

The default fixture is `perf/fixtures/12_gem_aetherium_2_bad_roll6.json`. Dimensions default to terminal width minus 34 and height minus one, matching the demo art area. All three paths use the demo's indexed-color normalization. `max` selects every declared knob maximum. Time advances deterministically by 1/60 per frame. `q` or Escape exits. A resize stops the comparison so dimensions cannot silently diverge.

The command limits frames to 300 and terminal area to 80,000 cells. Those bounds cannot interrupt a blocked write. Automated runs must use the external watchdog.

`scripts/19_gem_backends.py --directory perf/results/gem-backends-RUN` runs the three backends through the native PTY parser at 400×200, with 366×199 art and 12 frames each. Every run is externally guarded at 15 seconds, 256 MiB owned RSS and 32 MiB artifacts. It stops on failure, checks final colored terminal cells against the scene, and compares all three results. Python controls the experiment; native Rust consumes the terminal byte stream.

Use `--cols 196 --rows 62 --width 162 --height 61` for the screenshot dimensions, `--fixture max` for all maximum knobs, or `--backends console` to isolate one library. Always use a new result directory.

Per-frame timings separate scene generation, shared color normalization, library data conversion, and presentation. Presentation includes library diff, encoding and terminal writes. These are headless terminal measurements; GUI painting requires a separate guarded observation.

The existing demo still uses its playback relay. In the recorded 162×61 user run, worker frame 451 spent 479 µs generating and 676 µs encoding, while its 28 µs presentation field measured only worker-to-pipe output. The supervisor separately logged 933,781 µs waiting on terminal output in a one-second interval, 4,865 blocked writes out of 5,487 attempts and 636,928 bytes transferred. The direct-backend results isolate another output path and consumer; they do not establish that the interactive demo slowdown is fixed.
