# Direct Jev pixels

User steering, verbatim:

> sigh. okay. can we ask it to just render all the pixels or some shit i mean this was a pathetic experiment you cheated to get goal and completely gave up on using jev

Assistant action: request every pixel of a 32×32 jellyfish directly from Jev. The exact complete prompt and all 1,024 pixel questions are in request.json. Each returned choice becomes that pixel’s exact palette RGB value. The only display scaling is nearest-neighbor magnification. No procedural geometry, interpolation, image correction, or generated detail is applied.

## Result and validation

The initial 1,024-question request was rejected by the API with HTTP 400, `max_tokens_exceeded`. Its error response is retained. The same questions were then sent in four batches of 256, with identical shared state; exact batch requests and raw responses are preserved. All 1,024 answers returned. The aggregate response preserves each answer unchanged.

The result is a cyan mass with an amber interior, without resolved tentacles. No geometry was supplied by a drawing algorithm. Every PNG pixel and every foreground/background half-block in the terminal export was checked against Jev's returned palette choice. Preview scaling repeats each pixel in a 20×20 block. No image correction was applied.

A follow-up two-color skull experiment is preserved in `../direct-pixels-skull`. It returned all 576 colors in three batches. Its chosen white pixels form disconnected regions. Both experiments are retained as returned.

`scripts/27_jev_pixels.py render` replays the jellyfish; add `--folder art/jev/direct-pixels-skull` for the skull. Replay is offline. The `call` action resumes any completed batches and refuses to overwrite a completed aggregate response. `.grid` exports contain paired image rows as half-block cells for the existing serialized Grid format. This adds an offline experiment script, not a Rust mode.
