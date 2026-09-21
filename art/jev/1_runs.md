# Direct-pixel run layout

Each direct-pixel experiment folder contains exactly:

```text
io.jsonl    chronological prompt sequence, plan, HTTP requests/responses and render records
image.png   exact Jev-chosen pixel colors, magnified with nearest-neighbor scaling
```

`io.jsonl` records one JSON object per line. Request bodies contain the full scene state, palette and individual question keys for each pixel. Response records retain the returned answers, probabilities and usage. Authentication headers and API keys are excluded. Requests are saved before sending; failures remain in the sequence. Completed batches are reused on resume. Completed runs replay without additional API calls.

The `plan` record contains the full pixel-question map, batch size, original canvas dimensions and display scale. Rendering reads successful responses from the same log and writes only `image.png`.

```bash
python3 scripts/27_jev_pixels.py init --folder art/jev/NEW-RUN --prompt 'Your image description' --width 32 --height 32
python3 scripts/27_jev_pixels.py call --folder art/jev/NEW-RUN
python3 scripts/27_jev_pixels.py render --folder art/jev/NEW-RUN
```

`init` saves the prompt and plan. `call` makes paid API requests for missing pixels and renders the result. `render` is offline.

Existing direct jellyfish and skull runs have been consolidated into this layout with identical image bytes and unchanged API answer objects. Their original multi-file records remain at Git checkpoint `f74d2f0`. The earlier procedural exploration is historical and remains in its original layout.
