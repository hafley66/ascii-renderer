use std::io::IsTerminal;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const FRAME_TARGET: &str = "ascii_renderer::profile";
const LAYER_TARGET: &str = "ascii_renderer::profile::layer";
const DEFAULT_REPORT_EVERY: u64 = 120;

static SETTINGS: OnceLock<ProfileSettings> = OnceLock::new();

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

pub(crate) fn measure_layer<T>(
    mode: &'static str,
    layer: &'static str,
    render: impl FnOnce() -> T,
) -> T {
    if !settings().layers {
        return render();
    }
    let started = Instant::now();
    let output = render();
    tracing::debug!(
        target: LAYER_TARGET,
        mode,
        layer,
        elapsed_us = started.elapsed().as_secs_f64() * 1_000_000.0,
        "render layer profile"
    );
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
}
