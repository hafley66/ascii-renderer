# Astra: Gem Aetherium 2 performance pass

## Scope

Own only `src/modes/_50_gem_aetherium_2.rs`.

Improve the performance of the existing canonical mode `gem-aetherium-2` in place. Do not create, rename, alias, or register any third Gem Aetherium mode. Do not modify `_34_gemini_astrolabe.rs`.

## Required outcome

- Preserve `GemAetherium2Mode`, `gem-aetherium-2`, its 12 existing parameter keys, deterministic seed/time behavior, and visual subject.
- Profile the renderer at a large headless resolution. Use the existing layer timers and add or refine stable layer boundaries if they do not identify the expensive phases.
- Remove repeated work and allocations from per-frame and inner loops where measurements support it. Keep behavior bounded by grid dimensions and declared parameter maxima.
- Keep the renderer deterministic and test tiny grids plus maximum controls.
- Run focused mode tests, `cargo test`, and a headless performance probe. Do not write a huge grid to a terminal.

## Delivery

Commit only the modified existing source file with subject:

```text
perf: optimize Gem Aetherium 2 renderer
```
