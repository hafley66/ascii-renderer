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

## Row-conditioned drawing

`--strategy rows` sends one row at a time and includes all previously completed rows in the next request. Each previous row is encoded as one character per pixel, using the `palette_codes` dictionary included in the state. The API still returns one named Choice answer for every requested pixel.

```bash
python3 scripts/27_jev_pixels.py init --folder art/jev/NEW-200-RUN --prompt 'Your image description' --width 200 --height 200 --strategy rows --scale 1
python3 scripts/27_jev_pixels.py call --folder art/jev/NEW-200-RUN
```

`--scale 1` saves an actual 200×200 PNG. The state explicitly describes the input fields, history encoding, and output contract. Every 20 completed rows, `image.png` is refreshed with only those completed rows; the image record in `io.jsonl` marks whether it is complete. Full completion replaces that preview with the entire image. Requests that exceed the API token limit are recorded and split into smaller batches with the same completed-row history. Rate-limit retries are bounded and recorded. Completed pixel answers are reused when resuming.

The 200×200 run exceeded the context limit at row 158 even with one question. Its log records a switch to lossless run-length history (`history_encoding` event with value `runs`), then a return to 200 questions per request. A row such as `0:100 5:100` decodes to 100 black pixels followed by 100 cyan pixels. The complete earlier pixel history is retained. The caller honors this encoding event on resume; this run's change was explicit, rather than an automatic retry policy.
