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
