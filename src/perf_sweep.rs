//! Headless knob sweep: every registry knob at its max, fixed seconds each at
//! a large grid, fps table, then an in-process layer hotspot table for the worst.
use crate::_0_profile::{layer_capture_begin, layer_capture_end};
use crate::morph::IterateFrameRenderer;
use crate::registry::mode_spec;
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
    fn fps(&self) -> f64 {
        self.frames as f64 / self.wall.as_secs_f64().max(f64::EPSILON)
    }
    fn avg_ms(&self) -> f64 {
        self.wall.as_secs_f64() * 1_000.0 / self.frames.max(1) as f64
    }
}

fn env_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn set_knob(key: &str, value: Option<f32>) {
    let name = format!("ASCII_P_{key}");
    // The probe runs alone (--ignored, one thread); knobs are read through env.
    unsafe {
        match value {
            Some(v) => std::env::set_var(&name, v.to_string()),
            None => std::env::remove_var(&name),
        }
    }
}

fn run_for(label: &str, mode: &str, theme: &str, w: usize, h: usize, secs: f64, dt: f32, capture: bool) -> Option<RunStats> {
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
    Some(RunStats { label: label.to_string(), frames, wall, p50_ms: pick(0.5), p99_ms: pick(0.99), max_ms: pick(1.0) })
}

#[test]
#[ignore = "release-only knob sweep; run via perf/knob_sweep.sh"]
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
        println!("# knob sweep: {mode} does not render natively through iterate_grid; nothing measured");
        return;
    };
    let mut runs: Vec<(RunStats, f32)> = Vec::new();
    for p in spec.params {
        set_knob(p.key, Some(p.max));
        if let Some(r) = run_for(&format!("{}={}", p.key, p.max), &mode, &theme, w, h, secs, dt, false) {
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
            s.label, s.frames, s.fps(), s.avg_ms(), s.p50_ms, s.p99_ms, s.max_ms, base.fps() / s.fps().max(f64::EPSILON)
        );
    };
    row(&base);
    for (s, _) in &runs {
        row(s);
    }

    let worst_key = runs.first().map(|(s, _)| s.label.clone()).unwrap_or_else(|| "baseline".to_string());
    println!("\nworst: {worst_key}\n");
    if let Some(p) = spec.params.iter().find(|p| worst_key.starts_with(&format!("{}=", p.key))) {
        set_knob(p.key, Some(p.max));
    }
    let worst = run_for(&worst_key, &mode, &theme, w, h, secs, dt, true).expect("worst run rendered once already");
    let layers = layer_capture_end();
    for p in spec.params {
        set_knob(p.key, None);
    }
    let frame_ns = worst.wall.as_nanos() as f64 / worst.frames.max(1) as f64;
    println!("## hotspots at {worst_key}: {} frames, {:.1} fps\n", worst.frames, worst.fps());
    if layers.is_empty() {
        println!("no measure_layer timers fired for {mode}; wrap its painters in crate::_0_profile::measure_layer");
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

const NATIVE_MODES: &[&str] = &[
    "delta", "snakes", "fullmetal-eyes", "hypercube", "flux", "fireworks", "murmuration", "lanterns", "tide",
    "elevator", "ferris", "arboretum", "astrolabe", "sauron", "mahoraga-2", "mahoraga-3", "mahoraga-4",
    "mahoraga-5", "tree-of-life", "tree-of-life-2", "tree-of-life-3", "tree-of-life-4", "tree-of-life-5",
    "tree-of-life-6", "braid", "braid-2", "chladni", "pendulum-wave", "glm-apotheosis", "cosmograph",
    "illuminarium", "qwen-cathedral", "aetherforge", "gem-aetherium", "hyperloom", "fa6",
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
    "haiku-2-trees",
    "haiku-2-forest",
];

#[test]
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
    assert!(missing.is_empty(), "modes without layer timers: {}", missing.join(", "));
}
