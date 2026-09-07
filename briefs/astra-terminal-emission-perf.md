# Task: reduce terminal emission cost and preserve trace inputs

Work in this lane's worktree. Other contributors are active. Do not revert
unrelated changes. Own only the output/trace path files needed for this task:
`src/render.rs`, `src/gridio.rs`, `src/_0_profile.rs`, `src/cli.rs`, related
tests, and `docs/0_render_trace_and_presets.md` if the trace schema changes.

Observed installed trace for `gem-aetherium-2` at a 320x103 grid:

```json
{"dur_us":131081,"render_us":355,"emit_us":130726}
```

The mode renderer is sub-millisecond. The terminal ANSI serialization/write
path consumes the remaining time. Optimize `emit_grid` / `render_grid` for
ordinary one-shot terminal output. Retain the rendered visual output and color
semantics. Do not add a mode or change Gem Aetherium 2.

Requirements:

1. Determine and remove allocation/formatting/write overhead in the hot cell
   loop. Build output in a bounded buffer and make the write boundary explicit.
   Preserve Unicode width handling and foreground/background state behavior.
2. Add deterministic tests that compare an optimized ANSI encoding with the
   current required output semantics. Do not use `toBeDefined` equivalents.
3. Add a focused release-only performance probe for a 320x103 colored grid,
   reporting encoding and write timing separately if practical.
4. Trace both dimensions that affect a one-shot render: actual terminal size
   returned by `crossterm::terminal::size()` and the resolved grid size. The
   existing `grid.w`/`grid.h` records resolved dimensions. Add a separate
   nullable terminal-size field and identify the fallback case if size lookup
   fails. Trace events remain NDJSON and compatibility fields remain present.
5. Update trace documentation for the new terminal-size field.
6. Run `scripts/0_generate_modes.sh --check` and focused tests. Commit a single
   commit with subject exactly: `perf: reduce terminal ANSI emission overhead`.

Report changed files, test commands/results, and measured timings in the final
message.
