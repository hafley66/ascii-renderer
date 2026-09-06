#![cfg(unix)]

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

#[test]
fn trace_preserves_terminal_and_resolved_grid_dimensions() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("renders.ndjson");
    let cases = [
        (Some((120, 40)), None, None, (120, 40)),
        (Some((120, 40)), Some("320"), Some("103"), (320, 103)),
        (None, None, None, (80, 45)),
        (None, Some("320"), Some("103"), (320, 103)),
        (None, Some("320"), None, (320, 45)),
    ];
    for (index, (terminal, width, height, grid)) in cases.into_iter().enumerate() {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ascii-renderer"));
        command
            .args(["1701", "gem-aetherium-2", "deep"])
            .env_clear()
            // Prevent crossterm's tput subprocess fallback from finding tput.
            .env("PATH", "")
            .env("ASCII_TRACE_PATH", &path)
            .env("ASCII_TRACE_ALL", "1")
            .env("ASCII_T", "0")
            .stdin(Stdio::null());
        if let Some(width) = width {
            command.env("ASCII_GRID_W", width);
        }
        if let Some(height) = height {
            command.env("ASCII_GRID_H", height);
        }
        // Detach from the test runner's controlling terminal so crossterm
        // observes exactly the PTY below, or a failed lookup for piped stdout.
        unsafe {
            command.pre_exec(|| {
                nix::unistd::setsid()
                    .map(|_| ())
                    .map_err(std::io::Error::from)
            });
        }
        let reader = terminal.map(|(w, h)| {
            let size = nix::pty::Winsize {
                ws_col: w,
                ws_row: h,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            let pty = nix::pty::openpty(Some(&size), None).unwrap();
            command.stdout(Stdio::from(pty.slave));
            std::thread::spawn(move || {
                let mut master = std::fs::File::from(pty.master);
                let mut buffer = [0; 65536];
                loop {
                    match master.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(_) => (),
                        Err(error) if error.raw_os_error() == Some(nix::libc::EIO) => break,
                        Err(error) => panic!("read PTY: {error}"),
                    }
                }
            })
        });
        let output = command.output().unwrap();
        drop(command); // Close the parent's slave before joining the reader.
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if let Some(reader) = reader {
            reader.join().unwrap();
        }
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.ends_with('\n'));
        let events: Vec<serde_json::Value> = contents
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(events.len(), index + 1);
        let event = events.last().unwrap();
        assert_eq!(
            event["terminal_size"],
            terminal
                .map(|(w, h)| serde_json::json!({"w": w, "h": h}))
                .unwrap_or(serde_json::Value::Null)
        );
        assert_eq!(event["terminal_size_fallback"], terminal.is_none());
        assert_eq!(event["grid"], serde_json::json!({"w": grid.0, "h": grid.1}));
        assert_eq!(event["v"], 1);
        assert_eq!(event["mode"], "gem-aetherium-2");
        assert_eq!(event["theme"], "deep");
        assert_eq!(event["seed"], 1701);
        assert_eq!(event["time"], 0.0);
        assert_eq!(event["args"], serde_json::json!([]));
        assert!(matches!(
            event["kind"].as_str(),
            Some("render" | "slow_render")
        ));
        assert_eq!(
            event["dur_us"].as_u64().unwrap(),
            event["render_us"].as_u64().unwrap() + event["emit_us"].as_u64().unwrap()
        );
        assert!(event["ts_ms"].as_u64().unwrap() > 0);
        // Assert the complete key set so compatibility fields cannot disappear.
        let keys: Vec<_> = event
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "args",
                "dur_us",
                "emit_us",
                "grid",
                "kind",
                "knobs",
                "layers",
                "mode",
                "palette",
                "render_us",
                "seed",
                "terminal_size",
                "terminal_size_fallback",
                "theme",
                "time",
                "ts_ms",
                "v"
            ]
        );
    }
}
