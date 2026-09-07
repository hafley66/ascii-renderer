//! Native PTY + vt100 terminal driver. JSON stdin is control only, never frame data.
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde_json::{Value, json};
use std::io::{self, BufRead, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;

struct State {
    parser: vt100::Parser,
    bytes: u64,
    reads: u64,
    parse_ns: u64,
    eof: bool,
    error: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        return Err("usage: 1_native_terminal COLS ROWS RAW_LOG COMMAND [ARGS...]".into());
    }
    let cols: u16 = args[0].parse()?;
    let rows: u16 = args[1].parse()?;
    if cols == 0 || rows == 0 || u32::from(cols) * u32::from(rows) > 80_000 {
        return Err("native terminal limit: 80,000 cells".into());
    }
    let size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };
    let pair = native_pty_system().openpty(size)?;
    let mut command = CommandBuilder::new(&args[3]);
    command.args(&args[4..]);
    command.env("TERM", "xterm-256color");
    command.env_remove("NO_COLOR");
    let mut child = pair.slave.spawn_command(command)?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader()?;
    let mut writer = pair.master.take_writer()?;
    let state = Arc::new(Mutex::new(State {
        parser: vt100::Parser::new(rows, cols, 0),
        bytes: 0,
        reads: 0,
        parse_ns: 0,
        eof: false,
        error: None,
    }));
    let shared = Arc::clone(&state);
    let raw = std::fs::File::create(&args[2])?;
    let consumer = std::thread::spawn(move || {
        let mut raw = io::BufWriter::with_capacity(
            65536,
            flate2::write::GzEncoder::new(raw, flate2::Compression::fast()),
        );
        let mut buffer = [0_u8; 65536];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if let Err(error) = raw.write_all(&buffer[..n]) {
                        shared.lock().unwrap().error = Some(error.to_string());
                        break;
                    }
                    let mut s = shared.lock().unwrap();
                    let started = Instant::now();
                    s.parser.process(&buffer[..n]);
                    s.parse_ns += started.elapsed().as_nanos() as u64;
                    s.bytes += n as u64;
                    s.reads += 1;
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    if error.raw_os_error() != Some(5) {
                        shared.lock().unwrap().error = Some(error.to_string());
                    }
                    break;
                }
            }
        }
        shared.lock().unwrap().eof = true;
    });
    let mut out = io::BufWriter::new(io::stdout().lock());
    serde_json::to_writer(
        &mut out,
        &json!({"ready":true,"pid":std::process::id(),"cols":cols,"rows":rows}),
    )?;
    writeln!(out)?;
    out.flush()?;
    for line in io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line?)?;
        let op = request["op"].as_str().ok_or("missing op")?;
        let response = match op {
            "send" => {
                writer.write_all(request["text"].as_str().ok_or("missing text")?.as_bytes())?;
                writer.flush()?;
                json!({"ok":true})
            }
            "resize" => {
                let cols = u16::try_from(request["cols"].as_u64().ok_or("missing cols")?)?;
                let rows = u16::try_from(request["rows"].as_u64().ok_or("missing rows")?)?;
                if cols == 0 || rows == 0 || u32::from(cols) * u32::from(rows) > 80_000 {
                    return Err("resize exceeds cell limit".into());
                }
                state
                    .lock()
                    .unwrap()
                    .parser
                    .screen_mut()
                    .set_size(rows, cols);
                pair.master.resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })?;
                json!({"ok":true})
            }
            "screen" => json!({"text":state.lock().unwrap().parser.screen().contents()}),
            "capture" => {
                let s = state.lock().unwrap();
                let screen = s.parser.screen();
                let (rows, cols) = screen.size();
                let width = request["width"]
                    .as_u64()
                    .unwrap_or(cols as u64)
                    .min(cols as u64) as u16;
                let height = request["height"]
                    .as_u64()
                    .unwrap_or(rows as u64)
                    .min(rows as u64) as usize;
                let art: Vec<_> = screen
                    .rows_formatted(0, width)
                    .take(height)
                    .map(|r| String::from_utf8_lossy(&r).into_owned())
                    .collect();
                let text: Vec<_> = screen.rows(0, width).take(height).collect();
                json!({"rows":art,"text":text})
            }
            "status" => {
                let exit = child.try_wait()?;
                let s = state.lock().unwrap();
                json!({"alive":exit.is_none(),"exit_code":exit.map(|e|e.exit_code()),
                    "reader_done":s.eof,"error":s.error,"bytes":s.bytes,"reads":s.reads,"parse_us":s.parse_ns/1000})
            }
            "quit" => {
                let _ = child.kill();
                break;
            }
            _ => return Err("unknown op".into()),
        };
        serde_json::to_writer(&mut out, &response)?;
        writeln!(out)?;
        out.flush()?;
    }
    let _ = child.kill();
    let _ = child.wait();
    drop(writer);
    drop(pair.master);
    consumer.join().map_err(|_| "terminal reader panicked")?;
    Ok(())
}
