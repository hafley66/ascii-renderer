# Azulejo architectural tessellation

Mode: `azulejo`, owned by `src/modes/_43_azulejo.rs`. The numeric generator registers it after Bower. Native animation and demo knobs use the existing registry path.

The geometric starting point is the regular octagon/square tessellation. Each octagon contains arm-shaped tesserae and shoulder pieces; diamond joints contain smaller enamel diamonds. Five local inlay families combine eight-point stars, four-point crosses, alternating braids, twelve-point rosettes and stepped hollow centers. Continuous shear preserves shared octagon/joint coordinates. Pointed arch bays, narrow repeating border marks and bonded masonry frame the tiled panels.

This is a procedural interpretation, not a reconstruction of a particular building or historical tile plan. The Met's [star-shaped tile](https://www.metmuseum.org/art/collection/search/444459) describes puzzle-like star/cross wall revetments; its [geometric-pattern essay](https://www.metmuseum.org/fr/essays/geometric-patterns-in-islamic-art) describes repeated square, star and multisided units. These informed the distinction between repeated structural units and the ornament within them. The surrounding brick courses are a contemporary compositional reference to everyday masonry craft.

## Inputs and lifetime

`draw_azulejo(frame: &mut ModeFrame<'_>, knobs: &[f32; 11])` borrows one runtime-owned grid. The runtime allocates/reuses its frame grid and supplies seed, palette, arguments, live parameters and time. No mode cache or retained simulation exists. Every frame builds material ramps and one row-span vector per bay, then rasterizes the complete surface. Construction, shape, seed, size, theme and time require no cache invalidation.

Read order is positional CLI argument, explicit native parameter vector, then `param_f32` environment/default. Defaults match the local `Param` declarations. Nonfinite values use defaults; finite values clamp at the boundary. Nonfinite time uses zero. Finite time wraps at 3600 seconds before speed multiplication. No external effects or per-frame RNG history enter the artwork.

| Knob | Visible change |
|---|---|
| BAYS | Seed-selected arrangement or 1–4 framed bays |
| SCALE | 3–12 tile courses across the facade |
| PATTERN | Seed selection or five inlay families |
| ARCH | Rectangular through deeply pointed surrounds |
| INSET | Star valleys, ribbon width and hollow centers |
| FLOW | Signed course travel |
| TURN | Signed inlay rotation |
| BREATH | Periodic folding of inlay valleys |
| RELIEF | Ceramic grain, bevels and glaze highlights |
| SPEED | Shared clock; zero freezes all motion |
| BOND | Continuous shared-lattice shear |

Seed changes bay count, selected family, inlay orientation, material arrangement and piece tones. Rerolls use the demo's `rand_knob(seed XOR roll * 0x9E3779B97F4A7C15, param)` with the declared step snapping.

## Reviewed artifacts

- [100×36 default](../previews/0_azulejo/1_default100.png)
- [160×60 default](../previews/0_azulejo/2_default160.png)
- [2000×1000 cell-color downsample](../previews/0_azulejo/3_large2000.png)
- [12 exact demo rerolls at t=0 and t=9](../previews/0_azulejo/4_gallery.png)
- [Exact vectors and dimensions](../previews/0_azulejo/5_manifest.txt)

The large preview renders the full 2000×1000 grid, maps actual foreground/background cell colors and resizes that image to 1000×1000 using a 1:2 character aspect. It does not rerender at reduced geometry or density. Terminal previews use Menlo and the actual width-one glyphs. Dark interstices, shared diamond joints, arch boundaries and seed-selected structures were reviewed at both scales.

Reproduce exports with `AZULEJO_PREVIEW_DIR=/tmp/azulejo-review cargo test azulejo_export_gallery -- --ignored`, then `python3 perf/previews/0_azulejo/0_render.py /tmp/azulejo-review` (Pillow and macOS Menlo required).

## Validation and limits

Baseline: 362 unit tests and 181 integration tests passed, with 7 ignored. New tests cover fixed-seed static/nonzero-time snapshots; complete colored-grid repeatability after intervening frames; seed/theme/parameter effects; frozen animation; empty and tiny dimensions; width-one glyphs; finite/nonfinite boundary values; 20 seeds × 4 exact UI rerolls at two times. The pictured gallery is the first 12 rerolls of seed 42 in the deep theme, not exhaustive combination coverage.

Geometry density is identical in demo and benchmark. At dense settings on small terminals, multiple tessera edges share character cells, reducing individual inlay readability. The architecture and moving repeated field remain visible. Measurements cover native grid rendering, excluding terminal encoding/output. No animation infrastructure, existing modes or dependencies were changed.

Final debug validation: 365 unit tests and 181 integration tests passed, with 9 ignored (the two added ignored tests export previews and run isolated timing). `scripts/0_generate_modes.sh --check` passed. Static and t=7 plain snapshots were read and the corresponding colored geometry reviewed before accepting those two files. Concave shoulder shapes at high inset use sorted scanline intersection pairs.

## Performance

Commands: `perf/knob_sweep.sh azulejo 2000 1000 5 0.06 moss` and `cargo test --release azulejo_cold_and_random_cost -- --ignored --nocapture --test-threads=1`. The benchmark slot was coordinated with the other agent; no art-side build or preview batch overlapped these measurements.

The [complete first sweep](azulejo.md) measured default average 8.77 ms, p50 5.34 ms, p99 63.87 ms and maximum 81.27 ms. Early runs contained long tails, including BAYS=4 maximum 135.19 ms. Their cause was not established. Those measurements are preserved. Later sweep rows mostly had single-digit millisecond maxima.

The [follow-up cold and exact-reroll probe](1_azulejo_cold_random.txt) used 100 timed frames per case, after separately recording constructor and first-render costs:

| Case | Average ms | p50 ms | p99 ms | Maximum ms |
|---|---:|---:|---:|---:|
| Default | 5.269 | 5.205 | 5.825 | 5.879 |
| Every knob at its maximum | 7.184 | 7.113 | 7.914 | 8.019 |
| Slowest of the 12 pictured demo rerolls, roll 3 | 9.811 | 9.779 | 10.757 | 10.970 |

Case 0 is default, case 1 is all-max, and cases 2–13 correspond to pictured rerolls 0–11. First default constructor: 4.349 ms; first render: 12.748 ms; combined: 17.097 ms. Subsequent constructors ranged from 0.912 to 1.683 ms. There is no mode cache, so every sampled frame reconstructs its complete geometry and materials. No warm geometry is omitted from the steady-state render timings.

The 16.7 ms rendering target was met by the follow-up default/all-max/12-reroll averages and tails. It was exceeded by initial-sweep tail measurements and by the combined first constructor plus first render. This is sampled performance on this machine, not a bound on all knob combinations or operating-system scheduling.

At BAYS=4 the layer probe attributed 67.1% of frame time to tesserae, 11.5% to surrounds and 7.6% to masonry. Tesserae averaged 1159.2 microseconds per call, with four calls per frame. Geometry storage is one `height`-long clipping-span vector per current bay plus fixed material and vertex arrays; no retained per-tile allocations or per-cell trigonometry are used.

The pictured 12 rerolls contain BAYS selections 0–4, PATTERN selections 0–5, and SCALE values from 3.5 to 10.75. Zero uses the documented seed selection. The focused release tests also passed: 3 passed, 2 ignored.
