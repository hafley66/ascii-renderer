# Astra: chaos theory first-frame pizzazz

Own only `src/modes/_49_astra_chaos_theory.rs`.

The current `astra-chaos-theory` renderer has useful technical detail but its appealing composition emerges too late. Revise it so every static render at `ASCII_T=0` is immediately dense, readable, and visually satisfying at 80x24. Treat time as movement and variation of an already-complete scene, never as a warm-up phase that must accumulate before the picture appears.

Retain the mode name and all 24 controls. Preserve deterministic inputs and bounded work. Strengthen first-frame composition using visible attractor structure, contrast, deliberate foreground and background layers, a clear focal region, and structured trails or orbit bands. Avoid relying on random speckle as the main visual content. Ensure representative nondefault controls still produce distinct compositions and keep large-grid render cost practical.

Update the file's inline snapshots and tests for the revised render. Do not edit registry generation, integration tests, snapshot files outside this source file, performance reports, dependencies, playback, or any other mode. Run focused tests and `cargo test`.

Commit with this exact subject:

`feat: refine Astra chaos theory first frame`

Report the first-frame composition changes, test command, and commit hash.
