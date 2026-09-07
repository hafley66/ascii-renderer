# Art replay

```text
make art
```

Treat that as the full creative direction. Add one new file-owned
generated-registry mode in this repository. Choose its name and subject.

Read `AGENTS.md` first. Follow the generated registry workflow: choose the
next available numbered `src/modes/_N_name.rs` file, expose visible controls
through `Mode::params`, add fixed-seed snapshots, run
`scripts/0_generate_modes.sh`, and do not manually edit generated
`src/modes/mod.rs`.

Inspect static and moving deterministic renders, accept only snapshots for the
new mode, run relevant tests, and commit all artifacts with this exact subject:

```text
feat: astra make art
```

In the completion response, provide the mode name, controls, render commands,
test results, and commit hash.
