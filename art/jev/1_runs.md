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

## Integer-indexed RGBA face

`face-integer-rgba/` uses the image prompt `a face`, a 100×100 canvas, and fixed integer pixel indices `index = y*100+x`. Every question embeds its integer index, x/y coordinates, `type: rgba`, and channel name. Four Noul probabilities become `round(255*p)` RGBA values per pixel. This is the application's RGBA contract; the API's actual answer type remains Noul. There is no palette, composition prescription, prior-row context, or local image correction.

`image.png` preserves all four channels. The `ansi` record in `io.jsonl` contains truecolor background escapes with alpha composited over black, one space per cell. ANSI itself has no alpha channel. Replay uses `python3 scripts/29_jev_rgba.py --offline`.

### Explicit channel choices

`face-integer-rgba-choices/` repeats the same 100×100 face request with four Choice answers per fixed integer pixel index. Each channel selects one of 16 explicit byte values: `0, 17, 34, …, 255`. The selected value is written directly into the RGBA PNG; returned probabilities do not determine pixel values. This provides 16 levels per channel. Requests contain 100 channel questions each. Input describes the index, coordinates, RGBA type and channel; the image description remains `a face`.

Replay: `python3 scripts/29_jev_rgba.py --choices --offline`. The ANSI string in the log composites alpha over black.
