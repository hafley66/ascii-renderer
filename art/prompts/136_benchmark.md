# 136 benchmark prompt sequence

Verbatim user prompts, in the order they were given.

## 1. User prompt

```
make art

Create one new original animated procedural ASCII art mode. Choose its subject and name.
Work only in /app, your isolated Git worktree. Every participant starts from the same
source snapshot and uses index 136: src/modes/_136_<name>.rs. Do not inspect other
repositories, submissions, model evaluations, or host services. Do not change the task,
its tests, the baseline commit, or existing art modes.

Read AGENTS.md and .claude/skills/add-mode/SKILL.md. Follow the generated registry
workflow, expose visible controls using Mode::params, include fixed-seed snapshots,
and regenerate with scripts/0_generate_modes.sh. Preserve this exact prompt in
art/prompts/136_benchmark.md before your first completed implementation.

Inspect deterministic frames at seed 42, 80x24, times 0 and 6. Save previews and
render commands in art/benchmark-136/. Run the new mode tests and relevant registry
checks. Review each new snapshot before accepting it. No fake I/O tests. Do not
change existing snapshots to make tests pass. A headless terminal check is authorized;
use the repository's existing external watchdog for any live playback probe. Report
unavailable E2E prerequisites explicitly; do not claim GUI validation in this container.

Commit the implementation, snapshots, prompt, and preview artifacts with subject
"feat: add art mode 136". Finish with mode name, controls, commands, test outcomes,
limitations, and commit hash. Save that receipt to art/benchmark-136/receipt.md too.
```
