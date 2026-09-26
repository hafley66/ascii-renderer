# Brief: v2 of the taint isolation plan

You work in `$PWD`, a git worktree of ascii-renderer. It already holds your v1 plan at
`plans/2_taint_isolation.md`. Rewrite that file in place as v2 and commit it. No other file changes.
No code changes.

The problem statement is unchanged; read `briefs/ds41-taint-isolation-plan.md`.

## Review of v1: verified defects to fix

| # | Defect | Evidence |
| --- | --- | --- |
| 1 | v1 isolates `cargo test`, not the agent. The agent harness (omp, claude, codex) runs on the host with the whole repo, `.git` history, `art/`, `chat_log/`, `briefs/` readable. Taint is what the agent reads. | v1 section 4 and step 4 |
| 2 | "No mode imports another mode: 0 hits" is false. | `src/modes/_42_bower.rs:4` uses `crate::opus_1_trees`; `crate::morph` in `_40_moonwake`, `_41_vesper`, `_53_prismata`, `_54_nightglass`; `crate::gridio` in 3 modes. Run `grep -ho "crate::[a-z_0-9:]*" src/modes/_*.rs \| sort \| uniq -c` |
| 3 | The engine list misses symbols modes use. | `crate::opts::rand_knob` (6 uses), `crate::opts::effective_pvals` (2), `crate::gridio`, `crate::morph::IterateFrameRenderer` |
| 4 | `Param` line reference wrong. | `src/registry.rs:56`, not `:22-38`. Re-verify every line number you cite. |
| 5 | linkme drops entries from crates nothing references; the demo would need a `use` per mode crate, which is a roster again. | linkme docs |
| 6 | CI not updated. | `.github/workflows/0_ci.yml` |

## Direction for v2

Drop the workspace split and linkme. Keep all 55 modes where they are. Design instead:

1. **Export**: a script that builds a scratch directory from an allowlist: the engine files plus every
   symbol modes actually import (defect 2 and 3), a generated `src/modes/mod.rs` with exactly one entry
   (the new mode), a minimal `main.rs`/`lib.rs` that compiles, `Cargo.toml` + `Cargo.lock`, vendored
   deps for the `hafley-observe` git dependency (`Cargo.toml:28`). No `.git`, no `art/`, `briefs/`,
   `chat_log/`, `plans/`, `perf/`, `docs/`, reports, other modes, `CLAUDE.md` mode roster.
   State how the allowlist is kept correct as the engine grows (a check that fails loudly when the export
   does not compile).
2. **Run**: the agent harness itself runs inside a container that mounts only the scratch directory
   (read-write) and reaches the model API over the network. Name what else the harness brings with it
   (its own config, skills, memories, session history) and how each is excluded or allowed.
3. **Import**: a script that copies the new mode file, its snapshots, and its `art/prompts/<N>_<name>.md`
   back into the real repo at the next free `_N_` number, runs `scripts/0_generate_modes.sh`, and
   `cargo test --locked`. No snapshot is re-accepted blindly.

For each: type signatures or CLI signatures first, then the sequence of reads and writes, then failure
modes. Then numbered migration commits (conventional messages), each leaving `cargo test --locked` green.
Keep sections 1, 2, and 6 of v1 where still true, corrected.

## Done

`plans/2_taint_isolation.md` rewritten and committed with subject `docs(plans): taint isolation plan v2`.
