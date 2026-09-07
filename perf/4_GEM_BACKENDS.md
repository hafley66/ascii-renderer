# Gem 2 rendering setups

Work in progress: the adapter implementation passed `cargo check`. Release builds were stopped after the machine became overloaded. The focused adapter test and three-way terminal comparison have not run. No performance result is available for these adapters.

The `gem-lab` command calls the existing Gem 2 renderer directly, with explicit seed, palette, time and knobs. It bypasses demo playback and its process relay. Three choices own their own presentation state:

| Argument | Presentation |
| --- | --- |
| `ratatui` | Ratatui retained buffers and Termion backend, buffered stdout |
| `console` | console_engine screen storage, diff and output |
| `termwiz` | Termwiz Surface composition and BufferedTerminal |

Termwiz's [BufferedTerminal API](https://docs.rs/termwiz/0.23.3/termwiz/terminal/buffered/struct.BufferedTerminal.html) performs surface change optimization and terminal rendering on flush.

Command syntax after a successful build:

```text
ascii-renderer gem-lab BACKEND FRAMES LOG [INPUTS.json|max] [WIDTH HEIGHT] [hold]
```

The default fixture is `perf/fixtures/12_gem_aetherium_2_bad_roll6.json`. Dimensions default to terminal width minus 34 and height minus one, matching the demo art area. All three paths use the demo's indexed-color normalization. `max` selects every declared knob maximum. Time advances deterministically by 1/60 per frame. `q` or Escape exits. A resize stops the comparison so dimensions cannot silently diverge.

The command limits frames to 300 and terminal area to 80,000 cells. Those bounds cannot interrupt a blocked write. Automated runs must use the external watchdog.

`scripts/19_gem_backends.py --directory perf/results/gem-backends-RUN` runs the three backends through the native PTY parser at 400×200, with 366×199 art and 12 frames each. Every run is externally guarded at 15 seconds, 256 MiB owned RSS and 32 MiB artifacts. It stops on failure, checks final colored terminal cells against the scene, and compares all three results. Python controls the experiment; native Rust consumes the terminal byte stream.

Per-frame timings separate scene generation, shared color normalization, library data conversion, and presentation. Presentation includes library diff, encoding and terminal writes. These are headless terminal measurements; GUI painting requires a separate guarded observation.

Before resuming compilation, put builds under external resource limits too. `-j 1` alone did not prevent the reported machine overload. Do not repeat the unguarded release build.
