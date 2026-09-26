# Brief: make one new art mode without looking at the others

You work in `$PWD`, a git worktree of ascii-renderer. Make ONE new standalone mode, your own idea.

## The user's words, verbatim, in order

1. > how can i use it to go make art in a worktree taint and asll but asked not to look too hard at others
2. > try not to look at latest ones in psat 3 days, try to keep it efficient, have fun be avant garde but try not to be too influence buy the previous ones
3. > thats okay only ur stuff matters skip not ur stuff
4. > dont want u tainted

## Read ONLY these

| Path | For |
| --- | --- |
| `.claude/skills/add-mode/SKILL.md` | the file skeleton and wiring rules. It names two reference modes; do NOT open them. |
| `src/registry.rs` lines 1-140 | `Param`, `param!`, `AnimKind`, `ModeFrame`, `Mode` |
| `src/types.rs`, `src/color.rs`, `src/pp.rs` | grid, color, drawing helpers |
| `src/opts.rs` `param_f32`, `rand_knob` only | knob reads |
| `src/_0_profile.rs` `measure_layer` only | layer timers (required by the registry gate) |
| `AGENTS.md` "Ground rules" | repo rules |

## Do NOT read

Any other file under `src/modes/` except the one you create and the generated `mod.rs` diff; any
`snapshots/` directory except your own; `art/`, `briefs/` (other than this one), `chat_log/`, `plans/`,
`perf/`, `docs/`, `examples/`, `lanes/`, `*_REPORT.md`, `PERF.md`, the `## Modes` list in `CLAUDE.md`
(append your name to it without studying it), and git history (`git log -p`, `git show` of other commits).
If a compile error points into another mode, fix your file, not by reading theirs.

## Make

1. Pick the next unused number N: `ls src/modes | grep -o '^_[0-9]*' | tr -d _ | sort -n | tail -1`, plus one.
2. `src/modes/_N_<name>.rs`, following the skill skeleton: 3-8 knobs through `Mode::params`, every painter
   wrapped in `measure_layer`, a fixed-seed snapshot test at 80x24.
3. `scripts/0_generate_modes.sh`; append `<name>` to the end of the `CLAUDE.md` modes list.
4. `art/prompts/<N>_<name>.md`: the four user messages above, verbatim, in order, then this brief's path.
   Commit it before your first long build/test loop finishes (AGENTS.md prompt-sequence rule).
5. `cargo test --locked` green. Look at your own snapshot output before accepting it (`mv *.snap.new *.snap`
   only after you have read the art in it).
6. Commit: `feat(modes): <name> - <one line about the piece>`.

## Done

At least one commit adding `src/modes/_N_<name>.rs`, its snapshot, and `art/prompts/<N>_<name>.md`,
with `cargo test --locked` passing. Report the mode name, N, the knob list, and the command to view it:
`cargo run -- 1 <name>`.
