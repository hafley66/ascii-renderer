# Jev exploration

## Prompt provenance

The earlier conversation and first API prompt are retained in `../0_prompts.md` and `../request.json`.

Subsequent user steering, verbatim and in order:

1. `think bigger darling`
2. `listen i have no way to undersatand how to use this or care, but i wnt you to do whack shit and fully explore it and try every thing you can and save the progress and show me every picture along the way`

Assistant concept proposed between those messages: a forest that dreams itself into a cathedral; roots become columns, branches become vaults, and moonlight becomes stained glass. The concrete proposed experiment was “a drowned monastery remembering that it was once a forest”.

Every exact Jev state, question, rubric and composition prompt used below is retained in the numbered experiment's `request.json`. Responses and usage receipts are stored next to requests. API keys are excluded. The `07_director/spec.json` contains the finite numeric control choices and their mapping to native renderer parameters.

## Checkpoint 1

- The original forest distributions were replayed as mean colors, seeded categorical samples, and entropy contours.
- `04_jellyfish`: 576 binary silhouette questions. Explicit landmark coordinates produced a broad bell/body region; fine tentacles were not resolved.
- `05_moth`: 576 palette questions. Explicit landmarks produced color zones; wing boundaries and bilateral symmetry were not reliably resolved.
- `06_drowned`: 320 material questions. The probabilities divide the image into sky, central architecture, side growth and lower water. They do not supply detailed geometry.
- `07_director`: 36 questions choose finite native renderer controls, a palette and transformation. Captures retain the executable hash, exact command and environment. This uses existing `qwen-cathedral`, `cosmograph` and `kolam` modes with seed 4269 and time 5.
- Images 10–12 combine the directed cathedral with probability-tinted procedural roots and reflected water at three growth stages. The geometry comes from local rendering code; Jev provides recorded decisions and probability fields.

Every generated picture is retained as PNG, ANSI, plain text, serialized Grid and a metadata record. `scripts/22_jev_explore.py` handles API jobs and probability experiments. `scripts/23_jev_compositions.py` handles offline native compositions. No existing rendering mode has been edited.

## Checkpoint 2

`13_organisms` asks 18 Score questions for complexity, symmetry, luminescence, turbulence, ornament and warmth across three procedural concepts. The initial renderer consumes complexity and ornament to set vein, rib, filament and particle counts. Other scores are retained for subsequent variants. The moth uses palette probabilities from `05_moth`; the medusa uses occupancy probabilities from `04_jellyfish`; particle currents use material probabilities from `06_drowned`.

Images `13_moth_p0`, `14_medusa_p0`, and `15_engine_p0` are the first organism renderings. Exact replay was verified for all three. The first moth has a quadrant-dependent glyph selection artifact, retained as part of the progression. Shapes in this branch are explicit procedural geometry, with recorded Jev values controlling selected attributes.

## Checkpoint 3

- Revised moth glyph selection, applied symmetry to wing colors, and used scored luminescence/warmth/turbulence in the organism renderer. Original pictures remain intact as `*_p0`; revised pictures are `*_v2_p0`.
- Produced monochrome etchings of the moth and medusa.
- Sampled the native control distributions at temperatures 2 and 5 with fixed seeds 71 and 193. Six alternative compositions retain sampled decisions, native commands, exact knobs and raw source grids. No additional Jev calls were needed.
- Exported a 12-frame periodic medusa sequence and a 1.8-second looping GIF. Every full-resolution frame and corresponding Grid/ANSI export is retained under `motion/`. The GIF uses one palette for all frames. This is an offline image sequence, not live-terminal playback.
- Generated `index.md`, a machine-readable manifest and four contact sheets.

### Validation and coverage

`26_verify_jev_gallery.py` passed for 46 pictures: 34 stills and 12 animation frames. Checked 604,800 visible terminal cells against saved Grids, including glyph and truecolor foreground/background via pyte with ordinary tty ONLCR conversion. All 1,526 new API answers match requested coordinates/keys and types, probabilities satisfy range/sum checks, PNGs decode, Grid hashes match and all 12 motion frames differ. Same-input organism replay is deterministic; the medusa closes at phase 2π with zero quantized cell mismatches.

The guarded `scripts/13_e2e.sh --headless` run failed overall. Workflow, seed-search, bad-400x200 and max-400x200 passed. Backpressure failed its assertion that recorded terminal wait exceeds 500 ms. The case recorded input application at 1004.392833 ms and `applied_while_output_blocked=false`. The exact run and inputs remain under `perf/results/e2e-1789948566809`. Limits were unchanged, and no repeat stress run was launched. GUI painting, the function-trace variant and the pin-inputs case were not executed. `terminal-e2e.json` records this scope. No existing Rust mode or terminal playback code was changed by the art exploration.

### API usage

Five new API calls, all to the official TypeSafe endpoint, returned 1,526 answers. Reported usage totals 141,027 input tokens and 80,024 output tokens. The earlier forest response was reused without a new call. No dollar cost is inferred. Requests, responses and receipts are preserved in their numbered directories; the key is excluded.

### Replay

```bash
# Existing recorded probability experiments, no network:
python3 scripts/22_jev_explore.py recover
python3 scripts/22_jev_explore.py render 04_jellyfish
python3 scripts/22_jev_explore.py render 05_moth
python3 scripts/22_jev_explore.py render 06_drowned

# Recorded native compositions and revised organisms, no network:
python3 scripts/23_jev_compositions.py
python3 scripts/24_jev_organisms.py moth
python3 scripts/24_jev_organisms.py medusa
python3 scripts/24_jev_organisms.py engine
python3 scripts/25_jev_gallery.py sample
python3 scripts/25_jev_gallery.py animate
python3 scripts/25_jev_gallery.py index

# Requires Pillow and pyte (both in the repository's E2E venv):
python3 scripts/26_verify_jev_gallery.py
```

The original pre-revision organism code is retained at Git checkpoint `963a95e`. New API calls are only made by the explicit `22_jev_explore.py call JOB` action, which refuses to overwrite an existing response. Saved exports can be viewed with `cat PATH.ansi`; their cell dimensions are recorded in metadata, so the terminal must be large enough to prevent wrapping.
