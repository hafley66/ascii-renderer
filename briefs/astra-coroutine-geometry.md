# Astra coroutine: open geometric art

## Preamble

`make art`

Build one original, animated ASCII renderer with a high-complexity composition. Treat the grid as a complete image at 80x24 and 2000x1000. Use deterministic seed-driven structure, at least six live parameters, multiple depth layers, and visible motion at distinct time values. The render should have an internal geometric rhythm: repeated forms with controlled variation, clear negative space, and a coherent focal structure.

Do not imitate an existing mode or a named work. Choose the subject and implementation from the constraints above.

## Lease contract

Work only in `src/modes/_D_next.rs`. Do not assign a numeric prefix. Do not edit `src/modes/mod.rs`, snapshots, integration tests, performance scripts, dependencies, playback, or other modes. The primary checkout leases the next number, generates registration, creates and reviews snapshots, profiles, installs, and commits after delivery.

Include inline deterministic tests in `_D_next.rs` for a seed at two times and parameter clamping. Render small grids without panicking.

Commit the draft with this exact subject:

`feat: astra coroutine geometry draft`

Report the mode name, controls, test command and commit hash. A review message will arrive after the first pass. Use it to make a second committed iteration rather than ending work.
