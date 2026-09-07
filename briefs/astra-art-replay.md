# Astra art replay

Implement one new file-owned generated-registry art mode in this repository.
The task is an A/B replay: use the prior successful session directions below as
the input corpus, then make one fresh result that can be judged in the demo.

## Source directions

1. `make art`

   Follow-up after exploration: `this is fucking easily the fucking most
   varied when hitting random i ahve ever scene wow.`

2. `domain expansion: evolution`

   `evolve this repo like you're fighting mahagora in shibuya nad having a
   good time`

   Follow-up: `wow okay name this literally mahagora-2 in the code, copy iot
   and paste as maha 3`.

3. `please add 5 new modes, this is just literally a test of ur creativity
   dont break other stuff i guess, i'll explore in demo mode`

   Follow-up: `meadow === banger`; `AURORA is great`; the useful traits were
   contrast, graduated color and sizing, layering, and visual distinction
   under random exploration.

4. `heya, please add some sick as new things to this repo, do a new cool one
   shot of trees, make a sick ass new trunk/bole/stems/leaves/fruits/
   branching/engery/initial conditions tree drawer that can go from tiny to
   very large, i want the tree itself to have 10 properties of random knobbing
   in the algo, and then another 10 things about hwo the forest operates`

## Deliverable

Pick one subject. Do not reuse the names or copy the source modes. Produce a
composition with a readable large-scale structure, distinct layers, and broad
variation when its registered parameters are randomized. Make the movement a
deterministic pure function of time.

Read the repository `AGENTS.md` before changing code. Follow its generated
registry workflow: choose the next available `src/modes/_N_name.rs` number,
declare every visible tuning control in `Mode::params`, add deterministic
fixed-seed snapshots, run `scripts/0_generate_modes.sh`, and do not manually
edit generated `src/modes/mod.rs`.

Run the relevant tests, inspect static and moving terminal renders, commit the
mode, snapshot, and generated registry. Use this exact commit subject:

```text
feat: astra art replay
```

In the completion response, give the mode name, its parameter list, the static
and moving render commands used, test results, and commit hash.
