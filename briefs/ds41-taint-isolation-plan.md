# Brief: plan a refactor so an art run cannot be tainted by earlier art

You work in `$PWD`, a git worktree of ascii-renderer. Write ONE file and commit it:
`plans/2_taint_isolation.md`. Do not change any other file. No code changes.

## The problem

The user asks agents to make new art modes. The user's words (art/prompts/105_suture.md):

> try not to look at latest ones in psat 3 days ... try not to be too influence buy the previous ones
> thats okay only ur stuff matters skip not ur stuff
> dont want u tainted

"Tainted" means: during a run the agent reads earlier art (other modes' source in `src/`, outputs in
`art/`, prompts in `art/prompts/`, `briefs/`, `lanes/`, `chat_log/`, reports like `LANE_REPORT.md`) and
its output imitates them. Today nothing stops that: the whole repo is visible to every run.

## What the plan must contain

Read the repo first: `Cargo.toml`, `src/` layout (how a mode is registered and dispatched), `tests/`
(insta snapshots), `art/`, `briefs/`, `lanes/`, `scripts/`, `AGENTS.md`, `CLAUDE.md`. Cite paths and
line numbers for every claim about current structure.

1. Current structure: table of top-level dirs, what each holds, and whether an art run needs to read it.
2. The minimal "engine" an art run needs (grid, color, themes, the mode trait/registration, CLI entry)
   vs the "gallery" it must not see (other modes, their outputs, prompts, reports). Name the exact files.
3. Target layout. Type signatures of the mode interface a new mode implements, then how a mode is
   registered without editing a shared file that lists every other mode (a list of all modes is itself
   a taint source).
4. How a run is isolated. Compare at least: git sparse-checkout of a worktree, a separate crate/workspace
   member per mode, a Docker container with only the engine mounted read-only. For each: what the agent can
   still read, how tests/snapshots run, cost. Pick one and say why with evidence from the repo.
5. Migration steps in order, each one a commit with its conventional message, each leaving `cargo test`
   green. Say which snapshot tests move and how, and that no snapshot is re-accepted blindly.
6. What stays unisolated and why.

Style: tables and numbered steps, no prose paragraphs of opinion, no dates, file paths with line numbers.

## Done

`plans/2_taint_isolation.md` exists and is committed with subject `docs(plans): taint isolation refactor plan`.
