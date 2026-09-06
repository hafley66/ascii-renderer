use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const FRAME_TARGET: &str = "ascii_renderer::profile";
const LAYER_TARGET: &str = "ascii_renderer::profile::layer";
const DEFAULT_REPORT_EVERY: u64 = 120;

static SETTINGS: OnceLock<ProfileSettings> = OnceLock::new();
static TRACE_SETTINGS: OnceLock<TraceSettings> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProfileSettings {
    enabled: bool,
    layers: bool,
    report_every: u64,
}

impl ProfileSettings {
    fn from_env() -> Self {
        Self::parse(
            std::env::var("ASCII_PROFILE").ok().as_deref(),
            std::env::var("ASCII_PROFILE_LAYERS").ok().as_deref(),
            std::env::var("ASCII_PROFILE_EVERY").ok().as_deref(),
        )
    }

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

fn env_flag(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

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
    fn from_env() -> Self {
        Self::parse(
            std::env::var_os("ASCII_TRACE_PATH"),
            std::env::var("ASCII_TRACE").ok().as_deref(),
            std::env::var("ASCII_TRACE_ALL").ok().as_deref(),
            std::env::var("ASCII_TRACE_SLOW_MS").ok().as_deref(),
        )
    }

    fn parse(
        path: Option<std::ffi::OsString>,
        enabled: Option<&str>,
        all: Option<&str>,
        slow_ms: Option<&str>,
    ) -> Self {
        let disabled = enabled.is_some_and(|value| !env_flag(value));
        let path = path
            .map(PathBuf::from)
            .or_else(|| (!disabled).then(default_trace_path));
        Self {
            path,
            all: all.is_some_and(env_flag),
            slow_ms: slow_ms.and_then(|value| value.parse().ok()).unwrap_or(32),
        }
    }
}

fn default_trace_path() -> PathBuf {
    let mut path = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));
    path.push("ascii-renderer");
    path.push("renders.ndjson");
    path
}

fn trace_settings() -> &'static TraceSettings {
    TRACE_SETTINGS.get_or_init(TraceSettings::from_env)
}

pub(crate) fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    hafley_observe::init(config)
}

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
}

impl TraceEventKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Render => "render",
            Self::SlowRender => "slow_render",
        }
    }
}

/// Immutable render inputs captured before mode dispatch. The trace guard owns
/// this data so every early return from the CLI still records the same inputs.
pub(crate) struct RenderTraceContext {
    pub(crate) mode: String,
    pub(crate) theme: String,
    pub(crate) seed: u64,
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) terminal_size: Option<(u16, u16)>,
    pub(crate) time: f32,
    pub(crate) args: Vec<String>,
    pub(crate) knobs: BTreeMap<String, f32>,
}

/// Conditional append-only NDJSON tracing for one CLI render. Slow renders are
/// enabled by default; `ASCII_TRACE=0` disables them, `ASCII_TRACE_PATH` selects
/// a file, `ASCII_TRACE_ALL=1` records every render, and `ASCII_TRACE_SLOW_MS`
/// changes the slow threshold.
pub(crate) struct RenderTrace {
    context: RenderTraceContext,
    started: Instant,
    render_finished: Option<Duration>,
}

impl RenderTrace {
    pub(crate) fn start(context: RenderTraceContext) -> Option<Self> {
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
    pub(crate) fn mark_render_complete(&mut self) {
        self.render_finished = Some(self.started.elapsed());
    }
}

impl Drop for RenderTrace {
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
        let event = serde_json::json!({
            "v": 1,
            "kind": kind.as_str(),
            "ts_ms": timestamp_ms,
            "dur_us": elapsed_us,
            "render_us": render_us,
            "emit_us": emit_us,
            "mode": self.context.mode,
            "theme": self.context.theme,
            "seed": self.context.seed,
            "time": self.context.time,
            "grid": { "w": self.context.width, "h": self.context.height },
            "terminal_size": self.context.terminal_size.map(|(w, h)| serde_json::json!({ "w": w, "h": h })),
            "terminal_size_fallback": self.context.terminal_size.is_none(),
            "args": self.context.args,
            "knobs": self.context.knobs,
            "layers": layer_values,
        });
        append_ndjson(settings.path.as_ref().unwrap(), &event);
    }
}

fn append_ndjson(path: &std::path::Path, event: &serde_json::Value) {
    let Some(parent) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    if serde_json::to_writer(&mut file, event).is_ok() {
        let _ = file.write_all(b"\n");
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
pub(crate) fn layer_capture_begin() {
    LAYER_CAPTURE.with(|c| *c.borrow_mut() = Some(Vec::new()));
}

pub(crate) fn layer_capture_end() -> Vec<LayerTotal> {
    LAYER_CAPTURE.with(|c| c.borrow_mut().take().unwrap_or_default())
}

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
}

impl FrameProfiler {
    pub(crate) fn from_env(mode: &str, strategy: &str) -> Option<Self> {
        let settings = settings();
        settings.enabled.then(|| Self {
            mode: mode.to_owned(),
            strategy: strategy.to_owned(),
            report_every: settings.report_every,
            interval_started: Instant::now(),
            totals: FrameTotals::default(),
        })
    }

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
    fn drop(&mut self) {
        self.report("session end");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
    fn trace_settings_enable_default_and_explicit_paths() {
        let disabled = TraceSettings::parse(None, None, None, None);
        assert_eq!(disabled.path, Some(default_trace_path()));
        assert_eq!(disabled.slow_ms, 32);

        let opt_out = TraceSettings::parse(None, Some("0"), None, None);
        assert_eq!(opt_out.path, None);

        let explicit = TraceSettings::parse(
            Some("/tmp/ascii.ndjson".into()),
            None,
            Some("true"),
            Some("7"),
        );
        assert_eq!(explicit.path, Some(PathBuf::from("/tmp/ascii.ndjson")));
        assert!(explicit.all);
        assert_eq!(explicit.slow_ms, 7);
    }
}
