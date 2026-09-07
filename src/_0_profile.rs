use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const FRAME_TARGET: &str = "ascii_renderer::profile";
const LAYER_TARGET: &str = "ascii_renderer::profile::layer";
const DEFAULT_REPORT_EVERY: u64 = 120;

static SETTINGS: OnceLock<ProfileSettings> = OnceLock::new();
static TRACE_SETTINGS: OnceLock<TraceSettings> = OnceLock::new();
static TRACE_WRITERS: OnceLock<Mutex<TraceWriters>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProfileSettings {
    enabled: bool,
    layers: bool,
    report_every: u64,
}

impl ProfileSettings {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn from_env() -> Self {
        Self::parse(
            std::env::var("ASCII_PROFILE").ok().as_deref(),
            std::env::var("ASCII_PROFILE_LAYERS").ok().as_deref(),
            std::env::var("ASCII_PROFILE_EVERY").ok().as_deref(),
        )
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn parse(enabled: Option<&str>, layers: Option<&str>, report_every: Option<&str>) -> Self {
        Self {
            enabled: enabled.is_some_and(env_flag),
            layers: layers.is_some_and(env_flag),
            report_every: report_every
                .and_then(|value| value.parse().ok())
                .filter(|value| *value > 0)
                .unwrap_or(DEFAULT_REPORT_EVERY),
        }
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn env_flag(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn settings() -> &'static ProfileSettings {
    SETTINGS.get_or_init(ProfileSettings::from_env)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TraceSettings {
    path: Option<PathBuf>,
    all: bool,
    slow_ms: u64,
}

impl TraceSettings {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn from_env() -> Self {
        Self::parse(
            std::env::var_os("ASCII_TRACE_PATH"),
            std::env::var("ASCII_TRACE_ALL").ok().as_deref(),
            std::env::var("ASCII_TRACE_SLOW_MS").ok().as_deref(),
        )
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn parse(path: Option<std::ffi::OsString>, all: Option<&str>, slow_ms: Option<&str>) -> Self {
        Self {
            path: Some(path.map(PathBuf::from).unwrap_or_else(default_trace_path)),
            all: all.is_some_and(env_flag),
            slow_ms: slow_ms.and_then(|value| value.parse().ok()).unwrap_or(32),
        }
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn default_trace_path() -> PathBuf {
    let mut path = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));
    path.push("ascii-renderer");
    path.push("renders.ndjson");
    path
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn trace_settings() -> &'static TraceSettings {
    TRACE_SETTINGS.get_or_init(TraceSettings::from_env)
}

pub(crate) fn init() -> Result<Option<(tracing_appender::non_blocking::WorkerGuard, tracing_appender::non_blocking::ErrorCounter)>, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(directory) = std::env::var_os("ASCII_FUNCTION_TRACE") {
        use tracing_subscriber::fmt::format::FmtSpan;
        std::fs::create_dir_all(&directory)?;
        let path = PathBuf::from(directory).join(format!("functions-{}.ndjson.gz", std::process::id()));
        let file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
        let (writer, guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
            .buffered_lines_limit(4096)
            .lossy(false)
            .thread_name("function-trace-writer")
            .finish(std::io::BufWriter::with_capacity(64 * 1024,
                flate2::write::GzEncoder::new(file, flate2::Compression::fast())));
        let dropped = writer.error_counter();
        tracing_subscriber::fmt()
            .json()
            .with_span_list(false)
            .with_ansi(false)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_env_filter(tracing_subscriber::EnvFilter::new("warn,ascii_renderer::functions=trace"))
            .with_writer(writer)
            .try_init()?;
        return Ok(Some((guard, dropped)));
    }
    let settings = *SETTINGS.get_or_init(ProfileSettings::from_env);
    let default_filter = if settings.enabled {
        "ascii_renderer::profile=info,ascii_renderer::profile::layer=debug,warn"
    } else {
        "warn"
    };
    let config = hafley_observe::Config::from_env(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        default_filter,
        std::io::stderr().is_terminal(),
    )?;
    hafley_observe::init(config)?;
    Ok(None)
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn measure_render<T>(
    mode: &str,
    width: usize,
    height: usize,
    render: impl FnOnce() -> T,
) -> T {
    if !settings().enabled {
        return render();
    }
    let started = Instant::now();
    let output = render();
    tracing::info!(
        target: FRAME_TARGET,
        mode,
        width,
        height,
        render_us = started.elapsed().as_secs_f64() * 1_000_000.0,
        "render profile"
    );
    output
}

#[derive(Clone, Copy)]
enum TraceEventKind {
    Render,
    SlowRender,
    AnimationFrame,
    SlowAnimationFrame,
}

impl TraceEventKind {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn as_str(self) -> &'static str {
        match self {
            Self::Render => "render",
            Self::SlowRender => "slow_render",
            Self::AnimationFrame => "animation_frame",
            Self::SlowAnimationFrame => "slow_animation_frame",
        }
    }
}

/// Borrowed inputs shared by static rendering, animation, stress tests and replay.
/// Mode authors declare controls once in Mode::params; the caller supplies values.
pub(crate) struct FrameInputs<'a> {
    pub(crate) mode: &'a str,
    pub(crate) theme: &'a str,
    pub(crate) seed: u64,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) terminal_size: Option<(u16, u16)>,
    pub(crate) time: f32,
    pub(crate) args: &'a [String],
    pub(crate) params: &'a [crate::registry::Param],
    pub(crate) values: &'a [f32],
    pub(crate) palette: &'a [crossterm::style::Color; 5],
}

impl FrameInputs<'_> {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn to_json(&self) -> serde_json::Value {
        assert_eq!(self.params.len(), self.values.len());
        serde_json::json!({
            "mode": self.mode, "theme": self.theme, "seed": self.seed,
            "grid": {"w": self.width, "h": self.height},
            "terminal_size": self.terminal_size.map(|(w, h)| serde_json::json!({"w": w, "h": h})),
            "terminal_size_fallback": self.terminal_size.is_none(),
            "time": self.time, "args": self.args, "palette": self.palette,
            "knobs": self.params.iter().zip(self.values).map(|(p, v)| (p.key, *v)).collect::<BTreeMap<_, _>>(),
        })
    }
}

/// Append-only NDJSON tracing for one CLI render. `ASCII_TRACE_PATH` selects a
/// file, `ASCII_TRACE_ALL=1` records every render, and `ASCII_TRACE_SLOW_MS`
/// changes the slow threshold.
pub(crate) struct RenderTrace<'a> {
    context: FrameInputs<'a>,
    started: Instant,
    render_finished: Option<Duration>,
}

impl<'a> RenderTrace<'a> {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn start(context: FrameInputs<'a>) -> Option<Self> {
        trace_settings().path.as_ref()?;
        layer_capture_begin();
        Some(Self {
            context,
            started: Instant::now(),
            render_finished: None,
        })
    }

    /// Mark the point at which mode dispatch has finished and ANSI/grid output
    /// begins. The final trace event then separates renderer generation from
    /// terminal serialization and presentation.
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn mark_render_complete(&mut self) {
        self.render_finished = Some(self.started.elapsed());
    }
}

impl Drop for RenderTrace<'_> {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn drop(&mut self) {
        let settings = trace_settings();
        let layers = layer_capture_end();
        let elapsed = self.started.elapsed();
        let elapsed_us = elapsed.as_micros() as u64;
        let render_us = self.render_finished.unwrap_or(elapsed).as_micros() as u64;
        let emit_us = elapsed_us.saturating_sub(render_us);
        let slow = elapsed_us >= settings.slow_ms.saturating_mul(1_000);
        if !settings.all && !slow {
            return;
        }
        let kind = if slow {
            TraceEventKind::SlowRender
        } else {
            TraceEventKind::Render
        };
        let layer_values = layers
            .into_iter()
            .map(|layer| {
                serde_json::json!({
                    "name": layer.layer,
                    "calls": layer.calls,
                    "total_us": layer.total_ns as f64 / 1_000.0,
                    "max_us": layer.max_ns as f64 / 1_000.0,
                })
            })
            .collect::<Vec<_>>();
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let mut event = self.context.to_json();
        event.as_object_mut().unwrap().extend(
            serde_json::json!({
                "v": 1,
                "kind": kind.as_str(),
                "ts_ms": timestamp_ms,
                "dur_us": elapsed_us,
                "render_us": render_us,
                "emit_us": emit_us,
                "layers": layer_values,
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        append_ndjson(settings.path.as_ref().unwrap(), &event);
    }
}

#[derive(Default)]
struct TraceWriters {
    files: BTreeMap<PathBuf, std::fs::File>,
}

impl TraceWriters {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn append(&mut self, path: &std::path::Path, event: &serde_json::Value) {
        if !self.files.contains_key(path) {
            let Some(parent) = path.parent() else {
                return;
            };
            if std::fs::create_dir_all(parent).is_err() {
                return;
            }
            let Ok(file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            else {
                return;
            };
            self.files.insert(path.to_path_buf(), file);
        }
        let Some(file) = self.files.get_mut(path) else {
            return;
        };
        if let Ok(mut bytes) = serde_json::to_vec(event) {
            bytes.push(b'\n');
            // O_APPEND plus one write_all call keeps records intact when the
            // supervisor and worker share the same trace path.
            let _ = file.write_all(&bytes);
        }
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn append_ndjson(path: &std::path::Path, event: &serde_json::Value) {
    let Ok(mut writers) = TRACE_WRITERS
        .get_or_init(|| Mutex::new(TraceWriters::default()))
        .lock()
    else {
        return;
    };
    writers.append(path, event);
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlaybackStage {
    InputReceived,
    InputApplied,
    WorkerStopped,
    SessionExit,
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn playback_event(stage: PlaybackStage, worker_pid: u32, detail: impl serde::Serialize) {
    let Some(path) = &trace_settings().path else {
        return;
    };
    append_ndjson(
        path,
        &serde_json::json!({
            "v": 1, "kind": "playback_event", "stage": stage,
            "ts_ms": unix_ms(), "pid": std::process::id(), "worker_pid": worker_pid,
            "detail": detail,
        }),
    );
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// One interval of the parent output relay, including intervals with no frames.
/// Timings partition work and waits; worker presentation time overlaps this work.
#[derive(Default, serde::Serialize)]
pub(crate) struct RelayTotals {
    pub(crate) bytes: usize,
    pub(crate) read_calls: usize,
    pub(crate) write_calls: usize,
    pub(crate) max_read_bytes: usize,
    pub(crate) max_write_bytes: usize,
    pub(crate) read_us: u64,
    pub(crate) write_us: u64,
    pub(crate) input_us: u64,
    pub(crate) controls_us: u64,
    pub(crate) child_wait_us: u64,
    pub(crate) terminal_wait_us: u64,
    pub(crate) write_blocked: usize,
    pub(crate) max_write_us: u64,
    pub(crate) max_input_gap_us: u64,
    pub(crate) controls_dropped: usize,
}

pub(crate) struct RelayProfiler {
    pub(crate) totals: RelayTotals,
    pub(crate) terminal_size: Option<(u16, u16)>,
    started: Instant,
    last_input: Instant,
    worker_pid: u32,
    animation: bool,
}

impl RelayProfiler {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn new(worker_pid: u32, animation: bool) -> Self {
        let now = Instant::now();
        Self {
            totals: RelayTotals::default(),
            terminal_size: crossterm::terminal::size().ok(),
            started: now,
            last_input: now,
            worker_pid,
            animation,
        }
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn tick(&mut self) {
        if self.started.elapsed() >= Duration::from_secs(1) {
            self.report(false);
        }
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn input_polled(&mut self) {
        let now = Instant::now();
        self.totals.max_input_gap_us = self
            .totals
            .max_input_gap_us
            .max(now.duration_since(self.last_input).as_micros() as u64);
        self.last_input = now;
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn report(&mut self, final_sample: bool) {
        if let Some(path) = &trace_settings().path {
            let interval_us = self.started.elapsed().as_micros() as u64;
            let measured = self.totals.read_us
                + self.totals.write_us
                + self.totals.input_us
                + self.totals.controls_us
                + self.totals.child_wait_us
                + self.totals.terminal_wait_us;
            append_ndjson(
                path,
                &serde_json::json!({
                    "v": 1, "kind": "playback_relay", "ts_ms": unix_ms(),
                    "pid": std::process::id(), "worker_pid": self.worker_pid,
                    "animation": self.animation, "final_sample": final_sample,
                    "interval_us": interval_us, "unattributed_us": interval_us.saturating_sub(measured),
                    "terminal_size": self.terminal_size.map(|(w,h)| serde_json::json!({"w":w,"h":h})),
                    "timing": self.totals,
                }),
            );
        }
        self.totals = RelayTotals::default();
        self.started = Instant::now();
    }
}

impl Drop for RelayProfiler {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn drop(&mut self) {
        self.report(true);
    }
}

/// Per-layer totals gathered in-process while a capture is open.
#[derive(Clone, Debug, Default)]
pub(crate) struct LayerTotal {
    pub(crate) layer: &'static str,
    pub(crate) calls: u64,
    pub(crate) total_ns: u128,
    pub(crate) max_ns: u128,
}

thread_local! {
    static LAYER_CAPTURE: RefCell<Option<Vec<LayerTotal>>> = const { RefCell::new(None) };
}

/// Open an in-process layer capture on this thread. Layer timers record into it
/// even when ASCII_PROFILE_LAYERS is off, so probes need no log parsing.
#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn layer_capture_begin() {
    LAYER_CAPTURE.with(|c| *c.borrow_mut() = Some(Vec::new()));
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn layer_capture_end() -> Vec<LayerTotal> {
    LAYER_CAPTURE.with(|c| c.borrow_mut().take().unwrap_or_default())
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn layer_capture_record(layer: &'static str, elapsed_ns: u128) {
    LAYER_CAPTURE.with(|c| {
        if let Some(totals) = c.borrow_mut().as_mut() {
            match totals.iter_mut().find(|t| t.layer == layer) {
                Some(t) => {
                    t.calls += 1;
                    t.total_ns += elapsed_ns;
                    t.max_ns = t.max_ns.max(elapsed_ns);
                }
                None => totals.push(LayerTotal {
                    layer,
                    calls: 1,
                    total_ns: elapsed_ns,
                    max_ns: elapsed_ns,
                }),
            }
        }
    });
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn measure_layer<T>(
    mode: &'static str,
    layer: &'static str,
    render: impl FnOnce() -> T,
) -> T {
    let capturing = LAYER_CAPTURE.with(|c| c.borrow().is_some());
    if !settings().layers && !capturing {
        return render();
    }
    let started = Instant::now();
    let output = render();
    let elapsed = started.elapsed();
    if capturing {
        layer_capture_record(layer, elapsed.as_nanos());
    }
    if settings().layers {
        tracing::debug!(
            target: LAYER_TARGET,
            mode,
            layer,
            elapsed_us = elapsed.as_secs_f64() * 1_000_000.0,
            "render layer profile"
        );
    }
    output
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FrameSample {
    pub(crate) generation: Duration,
    pub(crate) encoding: Duration,
    pub(crate) presentation: Duration,
    pub(crate) bytes: usize,
    pub(crate) changed_cells: usize,
    pub(crate) runs: usize,
    pub(crate) full_repaint: bool,
    pub(crate) width: usize,
    pub(crate) height: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FrameTotals {
    frames: u64,
    generation_ns: u128,
    generation_ns_max: u128,
    encoding_ns: u128,
    encoding_ns_max: u128,
    presentation_ns: u128,
    presentation_ns_max: u128,
    bytes: u128,
    changed_cells: u128,
    runs: u128,
    full_repaints: u64,
    width: usize,
    height: usize,
}

impl FrameTotals {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn record(&mut self, sample: FrameSample) {
        let generation_ns = sample.generation.as_nanos();
        let encoding_ns = sample.encoding.as_nanos();
        let presentation_ns = sample.presentation.as_nanos();
        self.frames += 1;
        self.generation_ns += generation_ns;
        self.generation_ns_max = self.generation_ns_max.max(generation_ns);
        self.encoding_ns += encoding_ns;
        self.encoding_ns_max = self.encoding_ns_max.max(encoding_ns);
        self.presentation_ns += presentation_ns;
        self.presentation_ns_max = self.presentation_ns_max.max(presentation_ns);
        self.bytes += sample.bytes as u128;
        self.changed_cells += sample.changed_cells as u128;
        self.runs += sample.runs as u128;
        self.full_repaints += u64::from(sample.full_repaint);
        self.width = sample.width;
        self.height = sample.height;
    }
}

/// One instance is uniquely owned by one morph session. It only accumulates
/// timing and encoder counters; the renderer, grid, RNG, and terminal buffers
/// remain caller-owned. A strategy change flushes the preceding strategy's
/// interval so aggregate events never combine distinct render paths.
pub(crate) struct FrameProfiler {
    mode: String,
    strategy: String,
    report_every: u64,
    interval_started: Instant,
    totals: FrameTotals,
    frame_index: u64,
    trace_started: Instant,
    trace_frames: u64,
    trace_bytes: u64,
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn deterministic_animation_sample(frame_index: u64) -> bool {
    frame_index > 0 && (frame_index - 1) % 10 == 0
}

impl FrameProfiler {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn from_env(mode: &str, strategy: &str) -> Option<Self> {
        let settings = settings();
        (settings.enabled || trace_settings().path.is_some()).then(|| Self {
            mode: mode.to_owned(),
            strategy: strategy.to_owned(),
            report_every: settings.report_every,
            interval_started: Instant::now(),
            totals: FrameTotals::default(),
            frame_index: 0,
            trace_started: Instant::now(),
            trace_frames: 0,
            trace_bytes: 0,
        })
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn record_with_context(
        &mut self,
        strategy: &str,
        sample: FrameSample,
        context: impl FnOnce() -> serde_json::Value,
    ) {
        let trace = trace_settings();
        let total = sample.generation + sample.encoding + sample.presentation;
        self.frame_index += 1;
        self.trace_frames += 1;
        self.trace_bytes += sample.bytes as u64;
        if let Some(path) = trace.path.as_ref() {
            let periodic = deterministic_animation_sample(self.frame_index);
            if trace.all || total.as_millis() >= trace.slow_ms as u128 || periodic {
                // Resolve knob names and allocate JSON only for emitted records.
                let mut event = context();
                let fields = event.as_object_mut().unwrap();
                fields.extend(serde_json::json!({
                    "v": 1,
                    "kind": if total.as_millis() >= trace.slow_ms as u128 { TraceEventKind::SlowAnimationFrame.as_str() } else { TraceEventKind::AnimationFrame.as_str() },
                    "ts_ms": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,
                    "strategy": strategy,
                    "pid": std::process::id(),
                    "frame_index": self.frame_index,
                    "sampled": periodic,
                    "interval_ms": self.trace_started.elapsed().as_secs_f64() * 1000.0,
                    "interval_frames": self.trace_frames,
                    "interval_bytes": self.trace_bytes,
                    "dur_us": total.as_micros() as u64,
                    "render_us": sample.generation.as_micros() as u64,
                    "encoding_us": sample.encoding.as_micros() as u64,
                    "presentation_us": sample.presentation.as_micros() as u64,
                    "bytes": sample.bytes,
                    "changed_cells": sample.changed_cells,
                    "runs": sample.runs,
                    "full_repaint": sample.full_repaint,
                    "grid": {"w": sample.width, "h": sample.height},
                }).as_object().unwrap().clone());
                append_ndjson(path, &event);
                self.trace_started = Instant::now();
                self.trace_frames = 0;
                self.trace_bytes = 0;
            }
        }
        self.record(strategy, sample);
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    pub(crate) fn record(&mut self, strategy: &str, sample: FrameSample) {
        if self.strategy != strategy {
            self.report("strategy change");
            self.strategy.clear();
            self.strategy.push_str(strategy);
        }
        self.totals.record(sample);
        if self.totals.frames >= self.report_every {
            self.report("frame interval");
        }
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn report(&mut self, reason: &'static str) {
        if self.totals.frames == 0 {
            self.interval_started = Instant::now();
            return;
        }

        let frames = self.totals.frames;
        let wall_seconds = self.interval_started.elapsed().as_secs_f64();
        let divisor = frames as f64 * 1_000.0;
        tracing::info!(
            target: FRAME_TARGET,
            mode = self.mode.as_str(),
            strategy = self.strategy.as_str(),
            reason,
            frames,
            width = self.totals.width,
            height = self.totals.height,
            wall_ms = wall_seconds * 1_000.0,
            fps = frames as f64 / wall_seconds.max(f64::EPSILON),
            generation_us_avg = self.totals.generation_ns as f64 / divisor,
            generation_us_max = self.totals.generation_ns_max as f64 / 1_000.0,
            encoding_us_avg = self.totals.encoding_ns as f64 / divisor,
            encoding_us_max = self.totals.encoding_ns_max as f64 / 1_000.0,
            presentation_us_avg = self.totals.presentation_ns as f64 / divisor,
            presentation_us_max = self.totals.presentation_ns_max as f64 / 1_000.0,
            bytes_total = self.totals.bytes as u64,
            bytes_avg = self.totals.bytes as f64 / frames as f64,
            changed_cells_total = self.totals.changed_cells as u64,
            changed_cells_avg = self.totals.changed_cells as f64 / frames as f64,
            runs_total = self.totals.runs as u64,
            runs_avg = self.totals.runs as f64 / frames as f64,
            full_repaints = self.totals.full_repaints,
            "animation frame profile"
        );
        self.totals = FrameTotals::default();
        self.interval_started = Instant::now();
    }
}

impl Drop for FrameProfiler {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn drop(&mut self) {
        self.report("session end");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn settings_and_frame_totals_are_deterministic() {
        let settings = [
            ProfileSettings::parse(None, None, None),
            ProfileSettings::parse(Some("yes"), Some("ON"), Some("7")),
            ProfileSettings::parse(Some("0"), Some("false"), Some("0")),
        ];
        let mut totals = FrameTotals::default();
        totals.record(FrameSample {
            generation: Duration::from_micros(120),
            encoding: Duration::from_micros(40),
            presentation: Duration::from_micros(600),
            bytes: 900,
            changed_cells: 80,
            runs: 7,
            full_repaint: true,
            width: 80,
            height: 24,
        });
        totals.record(FrameSample {
            generation: Duration::from_micros(80),
            encoding: Duration::from_micros(60),
            presentation: Duration::from_micros(400),
            bytes: 300,
            changed_cells: 20,
            runs: 3,
            full_repaint: false,
            width: 80,
            height: 24,
        });

        insta::assert_debug_snapshot!((settings, totals), @r###"
        (
            [
                ProfileSettings {
                    enabled: false,
                    layers: false,
                    report_every: 120,
                },
                ProfileSettings {
                    enabled: true,
                    layers: true,
                    report_every: 7,
                },
                ProfileSettings {
                    enabled: false,
                    layers: false,
                    report_every: 120,
                },
            ],
            FrameTotals {
                frames: 2,
                generation_ns: 200000,
                generation_ns_max: 120000,
                encoding_ns: 100000,
                encoding_ns_max: 60000,
                presentation_ns: 1000000,
                presentation_ns_max: 600000,
                bytes: 1200,
                changed_cells: 100,
                runs: 10,
                full_repaints: 1,
                width: 80,
                height: 24,
            },
        )
        "###);
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn trace_settings_enable_default_and_explicit_paths() {
        let defaults = TraceSettings::parse(None, None, None);
        assert_eq!(defaults.path, Some(default_trace_path()));
        assert_eq!(defaults.slow_ms, 32);

        let explicit =
            TraceSettings::parse(Some("/tmp/ascii.ndjson".into()), Some("true"), Some("7"));
        assert_eq!(explicit.path, Some(PathBuf::from("/tmp/ascii.ndjson")));
        assert!(explicit.all);
        assert_eq!(explicit.slow_ms, 7);
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn animation_sampling_starts_at_first_draw_and_repeats_every_tenth_frame() {
        let sampled: Vec<_> = (1..=32)
            .filter(|frame| deterministic_animation_sample(*frame))
            .collect();
        assert_eq!(sampled, [1, 11, 21, 31]);
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn trace_writer_reuses_one_open_file_for_repeated_records() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested/trace.ndjson");
        let mut writers = TraceWriters::default();

        for sequence in 0..100 {
            writers.append(&path, &serde_json::json!({"sequence": sequence}));
        }

        assert_eq!(writers.files.len(), 1);
        drop(writers);
        let records = std::fs::read_to_string(path).unwrap();
        assert_eq!(records.lines().count(), 100);
    }
}
