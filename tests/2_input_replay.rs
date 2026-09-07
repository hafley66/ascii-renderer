use serde_json::{Value, json};
use std::process::{Command, Output};

fn renderer() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ascii-renderer"));
    command.env_clear().env("PATH", "").env("ASCII_TRACE", "0");
    command
}

fn success(output: Output) -> Vec<u8> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn registry_max_inputs_and_recorded_frames_replay_exactly() {
    let directory = tempfile::tempdir().unwrap();
    let trace = directory.path().join("frames.ndjson");
    let fixture: Value = serde_json::from_slice(&success(
        renderer()
            .args(["inputs", "gem-aetherium-2", "max"])
            .output()
            .unwrap(),
    ))
    .unwrap();
    assert_eq!(
        fixture["knobs"],
        json!({
            "RINGS":10.0, "PLANETS":12.0, "GEARS":8.0, "ZODIAC":24.0,
            "SPEED":3.0, "TILT":1.0, "NEBULA":1.5, "RAYS":1.5,
            "COMETS":12.0, "RUNES":1.0, "PULSE":2.0, "HARMONY":8.0
        })
    );
    let palette = json!(["red", "green", "blue", "yellow", "white"]);
    let mut originals = Vec::new();
    for time in ["0.25", "3.75"] {
        let mut command = renderer();
        command
            .args(["1701", "gem-aetherium-2", "deep", "9"])
            .env("ASCII_GRID_W", "80")
            .env("ASCII_GRID_H", "24")
            .env("ASCII_GRID_DUMP", "1")
            .env("ASCII_T", time)
            .env("ASCII_PALETTE", palette.to_string())
            .env("ASCII_TRACE_PATH", &trace)
            .env("ASCII_TRACE_ALL", "1");
        for (key, value) in fixture["knobs"].as_object().unwrap() {
            command.env(format!("ASCII_P_{key}"), value.to_string());
        }
        originals.push(success(command.output().unwrap()));
    }
    assert_ne!(originals[0], originals[1]);
    let events: Vec<Value> = std::fs::read_to_string(&trace)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["args"], json!(["9"]));
    assert_eq!(events[0]["knobs"]["RINGS"], 9.0);
    assert_eq!(events[0]["palette"], palette);
    // The parent continues logging relay/cleanup events after the last frame.
    use std::io::Write;
    writeln!(
        std::fs::OpenOptions::new()
            .append(true)
            .open(&trace)
            .unwrap(),
        "{}",
        json!({"kind": "playback_relay", "pid": 17})
    )
    .unwrap();
    for (line, expected) in [
        (Some("1"), &originals[0]),
        (Some("2"), &originals[1]),
        (None, &originals[1]),
    ] {
        let mut command = renderer();
        command
            .arg("replay")
            .arg(&trace)
            .env("ASCII_GRID_DUMP", "1")
            // Recorded inputs override conflicting caller settings.
            .env("ASCII_GRID_W", "3")
            .env("ASCII_GRID_H", "2")
            .env("ASCII_T", "99")
            .env("ASCII_P_RINGS", "2")
            .env("ASCII_PALETTE", "invalid");
        if let Some(line) = line {
            command.arg(line);
        }
        assert_eq!(&success(command.output().unwrap()), expected);
    }
    for line in ["0", "3", "invalid"] {
        let output = renderer()
            .arg("replay")
            .arg(&trace)
            .arg(line)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
    std::fs::write(&trace, "{}\n").unwrap();
    assert_eq!(
        renderer()
            .arg("replay")
            .arg(&trace)
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
}
