//! Registry input export and exact single-frame replay from NDJSON records.
use std::io::{BufRead, BufReader};
use std::process::Command;

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
pub(crate) fn command(args: &[String]) -> bool {
    let verb = args.get(1).map(String::as_str);
    if !matches!(verb, Some("inputs" | "replay")) {
        return false;
    }
    if let Err(error) = run(args) {
        eprintln!("{error}");
        std::process::exit(2);
    }
    true
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args[1] == "inputs" {
        let mode = args
            .get(2)
            .ok_or("usage: ascii-renderer inputs MODE [max|default]")?;
        let setting = args.get(3).map(String::as_str).unwrap_or("default");
        if !matches!(setting, "max" | "default") {
            return Err("expected max or default".into());
        }
        let spec = crate::registry::mode_spec(mode);
        if crate::registry::registered_mode(mode).is_none() && spec.params.is_empty() {
            return Err(format!("no declared parameters for {mode}").into());
        }
        let values: Vec<_> = spec
            .params
            .iter()
            .map(|p| if setting == "max" { p.max } else { p.default })
            .collect();
        let terminal_size = crossterm::terminal::size().ok();
        let (w, h) = terminal_size.unwrap_or((80, 45));
        let palette = crate::color::make_palette(42);
        let mut event = crate::_0_profile::FrameInputs {
            mode,
            theme: "",
            seed: 42,
            width: w as usize,
            height: h as usize,
            terminal_size,
            time: 0.0,
            args: &[],
            params: spec.params,
            values: &values,
            palette: &palette,
        }
        .to_json();
        event["v"] = 1.into();
        event["kind"] = "inputs".into();
        println!("{event}");
        return Ok(());
    }
    let path = args
        .get(2)
        .ok_or("usage: ascii-renderer replay FILE [LINE] (1-based; default: last)")?;
    let line_number = args.get(3).map(|s| s.parse::<usize>()).transpose()?;
    if line_number == Some(0) {
        return Err("line numbers start at 1".into());
    }
    let lines = BufReader::new(std::fs::File::open(path)?).lines();
    let line = if let Some(n) = line_number {
        lines.skip(n - 1).next()
    } else {
        lines.last()
    }
    .ok_or("trace record not found")??;
    let event: serde_json::Value = serde_json::from_str(&line)?;
    let mode = event["mode"].as_str().ok_or("record has no mode")?;
    if crate::registry::registered_mode(mode).is_none() {
        return Err("frame replay currently requires a registered Mode".into());
    }
    if let Some(strategy) = event["strategy"].as_str() {
        if strategy != "iterate" {
            return Err("frame replay currently supports native iterate animation".into());
        }
    }
    let seed = event["seed"].as_u64().ok_or("record has no seed")?;
    let w = event["grid"]["w"]
        .as_u64()
        .ok_or("record has no grid width")?;
    let h = event["grid"]["h"]
        .as_u64()
        .ok_or("record has no grid height")?;
    let time = event["time"]
        .as_f64()
        .ok_or("record has no animation time")?;
    let knobs = event["knobs"].as_object().ok_or("record has no knobs")?;
    if w == 0 || h == 0 || !time.is_finite() {
        return Err("invalid recorded dimensions or time".into());
    }
    let mut child = Command::new(std::env::current_exe()?);
    child.env_remove("ASCII_PALETTE");
    if !event["palette"].is_null() {
        let palette: [crossterm::style::Color; 5] =
            serde_json::from_value(event["palette"].clone())?;
        child.env("ASCII_PALETTE", serde_json::to_string(&palette)?);
    }
    child.args([
        seed.to_string(),
        mode.to_owned(),
        event["theme"].as_str().unwrap_or("").to_owned(),
    ]);
    if let Some(args) = event["args"].as_array() {
        for arg in args {
            child.arg(arg.as_str().ok_or("invalid recorded argument")?);
        }
    }
    child
        .env("ASCII_GRID_W", w.to_string())
        .env("ASCII_GRID_H", h.to_string())
        .env("ASCII_T", time.to_string());
    for (key, value) in knobs {
        let value = value.as_f64().ok_or("invalid recorded knob value")?;
        child.env(format!("ASCII_P_{key}"), value.to_string());
    }
    let status = child.status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
