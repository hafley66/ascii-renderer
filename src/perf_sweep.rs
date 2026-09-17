//! Headless knob sweep: every registry knob at its max, fixed seconds each at
//! a large grid, fps table, then an in-process layer hotspot table for the worst.
use crate::_0_profile::{layer_capture_begin, layer_capture_end};
use crate::morph::IterateFrameRenderer;
use crate::registry::{mode_spec, registered_modes};
use std::hint::black_box;
use std::time::{Duration, Instant};

struct RunStats {
    label: String,
    frames: u64,
    wall: Duration,
    p50_ms: f64,
    p99_ms: f64,
    max_ms: f64,
}

impl RunStats {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn fps(&self) -> f64 {
        self.frames as f64 / self.wall.as_secs_f64().max(f64::EPSILON)
    }
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn avg_ms(&self) -> f64 {
        self.wall.as_secs_f64() * 1_000.0 / self.frames.max(1) as f64
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn env_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn set_knob(key: &'static str, value: Option<f32>) {
    crate::opts::LIVE_PARAMS.with(|values| {
        values.borrow_mut().insert(key, value);
    });
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn run_for(
    label: &str,
    mode: &str,
    theme: &str,
    w: usize,
    h: usize,
    secs: f64,
    dt: f32,
    capture: bool,
) -> Option<RunStats> {
    let mut r = IterateFrameRenderer::new(mode, 42, theme, w, h)?;
    r.render(0.0, None)?;
    if capture {
        layer_capture_begin();
    }
    let mut samples: Vec<u128> = Vec::with_capacity(4096);
    let mut t = 0.0f32;
    let started = Instant::now();
    let mut frames = 0u64;
    while started.elapsed().as_secs_f64() < secs {
        let f0 = Instant::now();
        black_box(r.render(t, None));
        samples.push(f0.elapsed().as_nanos());
        frames += 1;
        t += dt;
    }
    let wall = started.elapsed();
    samples.sort_unstable();
    let pick = |q: f64| samples[((samples.len() - 1) as f64 * q) as usize] as f64 / 1e6;
    Some(RunStats {
        label: label.to_string(),
        frames,
        wall,
        p50_ms: pick(0.5),
        p99_ms: pick(0.99),
        max_ms: pick(1.0),
    })
}

#[test]
#[ignore = "release-only knob sweep; run via perf/knob_sweep.sh"]
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn perf_knob_sweep() {
    let mode: String = env_or("ASCII_PERF_MODE", "chladni".to_string());
    let theme: String = env_or("ASCII_PERF_THEME", "moss".to_string());
    let w: usize = env_or("ASCII_PERF_WIDTH", 2000);
    let h: usize = env_or("ASCII_PERF_HEIGHT", 1000);
    let secs: f64 = env_or("ASCII_PERF_SECS", 5.0);
    let dt: f32 = env_or("ASCII_PERF_DT", 0.06);
    let spec = mode_spec(&mode);
    for p in spec.params {
        set_knob(p.key, None);
    }

    let Some(base) = run_for("baseline", &mode, &theme, w, h, secs, dt, false) else {
        println!(
            "# knob sweep: {mode} does not render natively through iterate_grid; nothing measured"
        );
        return;
    };
    let mut runs: Vec<(RunStats, f32)> = Vec::new();
    for p in spec.params {
        set_knob(p.key, Some(p.max));
        if let Some(r) = run_for(
            &format!("{}={}", p.key, p.max),
            &mode,
            &theme,
            w,
            h,
            secs,
            dt,
            false,
        ) {
            runs.push((r, p.max));
        }
        set_knob(p.key, None);
    }
    runs.sort_by(|a, b| a.0.fps().partial_cmp(&b.0.fps()).unwrap());

    println!("# knob sweep: {mode} {w}x{h}, {secs}s per run, dt {dt}, theme {theme}\n");
    println!("| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |");
    println!("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
    let row = |s: &RunStats| {
        println!(
            "| {} | {} | {:.1} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2}x |",
            s.label,
            s.frames,
            s.fps(),
            s.avg_ms(),
            s.p50_ms,
            s.p99_ms,
            s.max_ms,
            base.fps() / s.fps().max(f64::EPSILON)
        );
    };
    row(&base);
    for (s, _) in &runs {
        row(s);
    }

    let worst_key = runs
        .first()
        .map(|(s, _)| s.label.clone())
        .unwrap_or_else(|| "baseline".to_string());
    println!("\nworst: {worst_key}\n");
    if let Some(p) = spec
        .params
        .iter()
        .find(|p| worst_key.starts_with(&format!("{}=", p.key)))
    {
        set_knob(p.key, Some(p.max));
    }
    let worst = run_for(&worst_key, &mode, &theme, w, h, secs, dt, true)
        .expect("worst run rendered once already");
    let layers = layer_capture_end();
    for p in spec.params {
        set_knob(p.key, None);
    }
    let frame_ns = worst.wall.as_nanos() as f64 / worst.frames.max(1) as f64;
    println!(
        "## hotspots at {worst_key}: {} frames, {:.1} fps\n",
        worst.frames,
        worst.fps()
    );
    if layers.is_empty() {
        println!(
            "no measure_layer timers fired for {mode}; wrap its painters in crate::_0_profile::measure_layer"
        );
        return;
    }
    let mut layers = layers;
    layers.sort_by(|a, b| b.total_ns.cmp(&a.total_ns));
    println!("| layer | calls/frame | avg us | max us | share of frame |");
    println!("| --- | ---: | ---: | ---: | ---: |");
    for l in &layers {
        let per_frame = l.total_ns as f64 / worst.frames.max(1) as f64;
        println!(
            "| {} | {:.1} | {:.1} | {:.1} | {:.1}% |",
            l.layer,
            l.calls as f64 / worst.frames.max(1) as f64,
            l.total_ns as f64 / l.calls.max(1) as f64 / 1e3,
            l.max_ns as f64 / 1e3,
            per_frame / frame_ns * 100.0
        );
    }
}

/// Report-only companion to the two timer gates: per mode, how many layer timers
/// fired and what share of the render call they attribute. The `add-mode` skill asks
/// for 3 to 8 layers covering at least 85 percent of the frame and nothing enforces
/// it, which is how eight registered modes shipped with no timers at all. This prints
/// the roster so a thin mode is visible at review instead of at the next perf
/// question. It asserts nothing on purpose: a coverage floor cannot be a hard gate
/// until layers stop nesting (`gem-aetherium-2` attributes 177 percent because
/// `background` wraps `nebula` and `rays`), and a report that fails the suite would
/// have to be waived instead of read.
#[test]
#[ignore = "release-only layer coverage report; run via perf/layer_coverage.sh"]
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn layer_coverage_report() {
    let w: usize = env_or("ASCII_LAYER_WIDTH", 400);
    let h: usize = env_or("ASCII_LAYER_HEIGHT", 120);
    let theme: String = env_or("ASCII_LAYER_THEME", "moss".to_string());
    let reps: usize = env_or("ASCII_LAYER_REPS", 3);
    let filter: String = env_or("ASCII_LAYER_FILTER", String::new());

    println!(
        "\n# layer coverage: {w}x{h}, theme {theme}, seed 42, dt 0.06, {reps} reps, release\n"
    );
    println!("| mode | layers | calls/frame | attributed | nested | thin |");
    println!("| --- | ---: | ---: | ---: | --- | --- |");
    let mut thin: Vec<(String, f64)> = Vec::new();
    let mut nested: Vec<(String, f64)> = Vec::new();
    let mut untraced: Vec<String> = Vec::new();
    let mut named: Vec<String> = Vec::new();
    let mut reported = 0usize;
    for mode in NATIVE_MODES {
        if !filter.is_empty() && !mode.contains(filter.as_str()) {
            continue;
        }
        let Some(mut r) = IterateFrameRenderer::new(mode, 42, &theme, w, h) else {
            continue;
        };
        reported += 1;
        r.render(0.0, None);
        layer_capture_begin();
        let mut render_ns = 0u128;
        for rep in 0..reps {
            let started = Instant::now();
            let rendered = r.render(0.5 + rep as f32 * 0.06, None);
            render_ns += started.elapsed().as_nanos();
            if rendered.is_none() {
                break;
            }
        }
        let mut layers = layer_capture_end();
        if layers.is_empty() {
            untraced.push(mode.to_string());
            println!("| {mode} | 0 | - | - | - | yes |");
            continue;
        }
        layers.sort_by(|a, b| b.total_ns.cmp(&a.total_ns));
        let calls: u128 = layers.iter().map(|l| l.calls as u128).sum();
        let attributed_ns: u128 = layers.iter().map(|l| l.total_ns).sum();
        let share = attributed_ns as f64 / render_ns.max(1) as f64 * 100.0;
        let is_nested = share > 102.0;
        let is_thin = share < 85.0;
        if is_nested {
            nested.push((mode.to_string(), share));
        }
        if is_thin || is_nested {
            named.push(format!(
                "  - {mode} ({share:.1}%): {}",
                layers
                    .iter()
                    .map(|l| l.layer)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if is_thin {
            thin.push((mode.to_string(), share));
        }
        println!(
            "| {mode} | {} | {:.1} | {share:.1}% | {} | {} |",
            layers.len(),
            calls as f64 / reps.max(1) as f64,
            if is_nested { "yes" } else { "no" },
            if is_thin { "yes" } else { "no" },
        );
    }
    println!();
    if !named.is_empty() {
        println!("modes worth reading (thin or nested):");
        for line in &named {
            println!("{line}");
        }
        println!();
    }
    println!(
        "{reported} modes reported: {} thin under 85 percent, {} nested over 100 percent, {} with no timers at all{}",
        thin.len(),
        nested.len(),
        untraced.len(),
        if untraced.is_empty() {
            String::new()
        } else {
            format!(": {}", untraced.join(", "))
        }
    );
}

/// The ruler for `perf/16_LARGE_RENDER_PLAN.md`: whole-mode render, blank grid
/// build, and the three encoder stages at one grid size, so every later change
/// is measurable in the same units instead of through a throwaway bench.
#[test]
#[ignore = "release-only split probe; run via perf/split_probe.sh"]
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn perf_split_probe() {
    use crate::gridio::AnsiFrameEncoder;
    use crate::types::Cell;

    let mode: String = env_or("ASCII_SPLIT_MODE", "prismata".to_string());
    let theme: String = env_or("ASCII_SPLIT_THEME", "moss".to_string());
    let w: usize = env_or("ASCII_SPLIT_WIDTH", 366);
    let h: usize = env_or("ASCII_SPLIT_HEIGHT", 199);
    let reps: usize = env_or("ASCII_SPLIT_REPS", 9);
    let dt: f32 = env_or("ASCII_SPLIT_DT", 0.06);
    let Some(mut renderer) = IterateFrameRenderer::new(&mode, 42, &theme, w, h) else {
        println!(
            "# split probe: {mode} does not render natively through iterate_grid; nothing measured"
        );
        return;
    };

    let mut render_us: Vec<f64> = Vec::with_capacity(reps);
    let mut t = 0.0f32;
    for _ in 0..reps {
        let started = Instant::now();
        black_box(renderer.render(t, None).expect("native renderer returns a grid"));
        render_us.push(started.elapsed().as_secs_f64() * 1e6);
        t += dt;
    }
    let first = renderer
        .render(0.0, None)
        .expect("native renderer returns a grid")
        .clone();
    let second = renderer
        .render(dt, None)
        .expect("native renderer returns a grid")
        .clone();

    let mut blank_us: Vec<f64> = Vec::with_capacity(reps);
    for _ in 0..reps {
        let started = Instant::now();
        let mut grid = vec![vec![Cell::blank(); w]; h];
        for row in &mut grid {
            row.fill(Cell::blank());
        }
        black_box(grid[0][0]);
        blank_us.push(started.elapsed().as_secs_f64() * 1e6);
    }

    let mut full = Vec::with_capacity(reps);
    let mut delta = Vec::with_capacity(reps);
    let mut identical = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut encoder = AnsiFrameEncoder::new();
        let mut output = Vec::new();
        full.push(encoder.encode(&first, true, &mut output));
        output.clear();
        delta.push(encoder.encode(&second, false, &mut output));
        output.clear();
        identical.push(encoder.encode(&second, false, &mut output));
        black_box(output.len());
    }

    let median = |mut values: Vec<f64>| -> f64 {
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        values[values.len() / 2]
    };
    let min = |values: &[f64]| -> f64 { values.iter().copied().fold(f64::INFINITY, f64::min) };
    let us = |d: Duration| d.as_secs_f64() * 1e6;

    println!(
        "\n# split probe: {mode} {w}x{h} ({} cells), theme {theme}, seed 42, dt {dt}, {reps} reps, release\n",
        w * h
    );
    println!("| stage | median us | min us | convert us | emit us | bytes | cells changed | cells skipped | cells invisible | runs |");
    println!("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
    println!(
        "| render (whole mode) | {:.1} | {:.1} | - | - | - | - | - | - | - |",
        median(render_us.clone()),
        min(&render_us)
    );
    println!(
        "| blank grid build + row fill | {:.1} | {:.1} | - | - | - | - | - | - | - |",
        median(blank_us.clone()),
        min(&blank_us)
    );
    let row = |label: &str, s: &[crate::gridio::FrameEncodeStats]| {
        let total: Vec<f64> = s.iter().map(|x| us(x.convert + x.emit)).collect();
        let convert: Vec<f64> = s.iter().map(|x| us(x.convert)).collect();
        let emit: Vec<f64> = s.iter().map(|x| us(x.emit)).collect();
        let bytes = median(s.iter().map(|x| x.bytes as f64).collect());
        let changed = median(s.iter().map(|x| x.changed_cells as f64).collect());
        let skipped = median(s.iter().map(|x| x.skipped as f64).collect());
        let invisible = median(s.iter().map(|x| x.invisible as f64).collect());
        let runs = median(s.iter().map(|x| x.runs as f64).collect());
        println!(
            "| {label} | {:.1} | {:.1} | {:.1} | {:.1} | {bytes:.0} | {changed:.0} | {skipped:.0} | {invisible:.0} | {runs:.0} |",
            median(total.clone()),
            min(&total),
            median(convert.clone()),
            median(emit.clone()),
        );
    };
    row("encode full repaint", &full);
    row("encode delta over one dt step", &delta);
    row("encode identical frame", &identical);
}

/// Every mode `IterateFrameRenderer` can build in process: the legacy native modes,
/// which are not in the registry, plus the registered ones. The registry-wide gate
/// beside the timer gate covers the registered side independently, which is how the
/// thirteen that were missing here were found.
const NATIVE_MODES: &[&str] = &[
    "delta",
    "snakes",
    "fullmetal-eyes",
    "hypercube",
    "flux",
    "fireworks",
    "murmuration",
    "lanterns",
    "tide",
    "elevator",
    "ferris",
    "arboretum",
    "astrolabe",
    "sauron",
    "mahoraga-2",
    "mahoraga-3",
    "mahoraga-4",
    "mahoraga-5",
    "tree-of-life",
    "tree-of-life-2",
    "tree-of-life-3",
    "tree-of-life-4",
    "tree-of-life-5",
    "tree-of-life-6",
    "braid",
    "braid-2",
    "chladni",
    "pendulum-wave",
    "glm-apotheosis",
    "cosmograph",
    "illuminarium",
    "qwen-cathedral",
    "aetherforge",
    "gem-aetherium",
    "hyperloom",
    "singularity",
    "thunderhead",
    "mandelbox",
    "fa6",
    "polytope",
    "poincare",
    "opus-1-quasicrystal",
    "opus-2-quasicrystal",
    "sonnet-1-spirograph",
    "sonnet-2-clifford",
    "haiku-1-torus",
    "haiku-2-ripple",
    "fable-1-trees",
    "fable-1-forest",
    "fable-2-trees",
    "fable-2-forest",
    "opus-1-trees",
    "opus-1-forest",
    "opus-2-trees",
    "opus-2-forest",
    "haiku-1-trees",
    "haiku-1-forest",
    "haiku-2-trees",
    "haiku-2-forest",
    "sonnet-2-trees",
    "sonnet-2-forest",
    "sonnet-1-trees",
    "sonnet-1-forest",
    "opus-5-dover",
    "prismata",
    "nightglass",
    "astra-chaos-theory",
    "astra-jurassic-park",
    "astra-opus-1-chronofold",
    "azulejo",
    "bower",
    "chimera-shadow-garden",
    "gem-aetherium-2",
    "gem-aetherium-3",
    "moonwake",
    "terminal-stress",
    "tideglass",
    "vesper",
    "volute",
];

#[test]
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn every_native_mode_has_layer_timers() {
    let mut missing = Vec::new();
    for mode in NATIVE_MODES {
        let Some(mut r) = IterateFrameRenderer::new(mode, 42, "moss", 120, 40) else {
            missing.push(format!("{mode} (not native)"));
            continue;
        };
        layer_capture_begin();
        let ok = r.render(0.5, None).is_some();
        let layers = layer_capture_end();
        if !ok || layers.is_empty() {
            missing.push(mode.to_string());
        }
    }
    assert!(
        missing.is_empty(),
        "modes without layer timers: {}",
        missing.join(", ")
    );
}

/// NATIVE_MODES is only the sweep's subset; the registry is the roster. A mode
/// that renders in-process must fire layer timers or its hotspots are invisible,
/// which is how eight registered modes shipped with no timers at all.
#[test]
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn every_registered_mode_that_renders_in_process_has_layer_timers() {
    let mut missing = Vec::new();
    let mut covered = 0usize;
    for (name, _) in registered_modes().iter() {
        let Some(mut r) = IterateFrameRenderer::new(name, 42, "moss", 120, 40) else {
            continue;
        };
        covered += 1;
        layer_capture_begin();
        let ok = r.render(0.5, None).is_some();
        let layers = layer_capture_end();
        if !ok || layers.is_empty() {
            missing.push(name.to_string());
        }
    }
    assert!(
        covered > 0,
        "no registered mode rendered in process; the gate proved nothing"
    );
    assert!(
        missing.is_empty(),
        "{covered} in-process registered modes checked, without layer timers: {}",
        missing.join(", ")
    );
}
