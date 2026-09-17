//! Animation delivery through a real PTY, driving the real binary.
//!
//! Three properties, each a defect listed in
//! plans/terminal-throughput/01_measurement_audit.md:
//! 1. the animation clock follows wall time, not the frame count, so a slow
//!    terminal shows fewer frames instead of slow motion;
//! 2. (in src/_1_playback.rs tests: `relay_moves_bytes_at_least_half_as_fast_as_cat`)
//! 3. a control key reaches the screen without first draining stale frames.

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const COLS: u16 = 200;
const ROWS: u16 = 50;

fn pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize { rows: ROWS, cols: COLS, pixel_width: 0, pixel_height: 0 })
        .unwrap()
}

fn renderer(args: &[&str]) -> CommandBuilder {
    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_ascii-renderer"));
    command.args(args);
    command.env_clear();
    command.env("PATH", "");
    command.env("TERM", "xterm-256color");
    command.env("ASCII_TRACE_PATH", std::env::temp_dir().join("ascii-relay-test-renders.ndjson"));
    command
}

fn morph_iterate() -> CommandBuilder {
    renderer(&["42", "morph", "moss", "prismata", "42", "prismata", "43", "iterate"])
}

/// The terminal: reads `chunk` bytes from the PTY, then pauses. Throttling here
/// is what makes the kernel queue fill and the worker stall.
fn reader_thread(mut reader: Box<dyn Read + Send>, chunk: usize, pause: Duration) -> mpsc::Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buffer = vec![0u8; chunk];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if sender.send(buffer[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
            std::thread::sleep(pause);
        }
    });
    receiver
}

const SLOW_CHUNK: usize = 512;
const SLOW_PAUSE: Duration = Duration::from_millis(10);

/// Status row of the vt100 screen: the worker writes it at the last row.
fn status_row(parser: &vt100::Parser) -> String {
    parser.screen().contents_between(ROWS - 1, 0, ROWS - 1, COLS)
}

/// Feed the parser for `duration`; returns bytes seen.
fn drain_for(receiver: &mpsc::Receiver<Vec<u8>>, parser: &mut vt100::Parser, duration: Duration) -> usize {
    let started = Instant::now();
    let mut total = 0;
    while let Some(left) = duration.checked_sub(started.elapsed()) {
        match receiver.recv_timeout(left) {
            Ok(chunk) => {
                total += chunk.len();
                parser.process(&chunk);
            }
            Err(_) => break,
        }
    }
    total
}

/// Defect 1 (morph.rs:1371): `clock += 0.06; phase += speed` per rendered frame.
/// Mode A == mode B is walk mode: every time `phase` crosses 1.0 the target seed
/// in the status row advances by one (morph.rs:1374-1378). At 0.011 per 60 fps
/// frame that is 0.66 phase per wall second. A terminal draining at 50 KiB/s
/// stalls the worker once the pipe and relay buffers fill; after 5 wall seconds
/// a wall-clock animation has advanced the seed 3 times, a frame-count one has
/// not advanced it at all.
#[test]
fn slow_terminal_drops_frames_instead_of_slowing_the_animation() {
    let pair = pty();
    let mut child = pair.slave.spawn_command(morph_iterate()).unwrap();
    drop(pair.slave);
    let receiver = reader_thread(pair.master.try_clone_reader().unwrap(), SLOW_CHUNK, SLOW_PAUSE);
    let mut writer = pair.master.take_writer().unwrap();
    let mut parser = vt100::Parser::new(ROWS, COLS, 0);

    // Close the options pane: the seed pair is only in the pane-less status row.
    drain_for(&receiver, &mut parser, Duration::from_millis(400));
    writer.write_all(b"o").unwrap();
    drain_for(&receiver, &mut parser, Duration::from_millis(200));
    let started = Instant::now();
    let throttled = drain_for(&receiver, &mut parser, Duration::from_secs(5));
    let wall = started.elapsed().as_secs_f32();
    // Freeze, then drain the backlog so the last throttled frame is on screen.
    writer.write_all(b" ").unwrap();
    let mut backlog = 0;
    loop {
        let seen = drain_for(&receiver, &mut parser, Duration::from_millis(200));
        backlog += seen;
        if seen == 0 {
            break;
        }
    }
    eprintln!("throttled {throttled} B, backlog {backlog} B");
    let status = status_row(&parser);
    writer.write_all(b"q").unwrap();
    let _ = child.wait();

    let seed = walk_seed(&status).unwrap_or_else(|| panic!("no target seed in status row: {status:?}"));
    let expected = 43 + (0.011 * 60.0 * wall).floor() as u64;
    eprintln!("wall {wall:.2}s seed {seed} expected {expected} status {status:?}");
    assert!(
        seed >= expected - 1,
        "after {wall:.2}s the walk target seed is {seed}; a wall-clock animation \
         would be at {expected}; status {status:?}"
    );
}

/// `... | morph prismata:42 → prismata:45 | ...`
fn walk_seed(status: &str) -> Option<u64> {
    let arrow = status.find('\u{2192}')? + '\u{2192}'.len_utf8();
    let rest = status[arrow..].trim_start();
    let colon = rest.find(':')? + 1;
    rest[colon..].split(' ').next()?.parse().ok()
}

/// Defect 3 (_1_playback.rs:216-272): every stale frame is relayed in full. With
/// the terminal draining at ~100 KiB/s, pressing space must put the pause glyph
/// on the status row within two throttled frames' worth of bytes, which means
/// the frames queued before the keypress were skipped rather than replayed.
#[test]
fn pause_reaches_the_screen_without_replaying_stale_frames() {
    let pair = pty();
    let mut child = pair.slave.spawn_command(morph_iterate()).unwrap();
    drop(pair.slave);
    let receiver = reader_thread(pair.master.try_clone_reader().unwrap(), SLOW_CHUNK, SLOW_PAUSE);
    let mut writer = pair.master.take_writer().unwrap();
    let mut parser = vt100::Parser::new(ROWS, COLS, 0);

    // Build a backlog: 3 s throttled.
    drain_for(&receiver, &mut parser, Duration::from_secs(3));
    writer.write_all(b" ").unwrap();
    let pressed = Instant::now();
    let mut seen_at = None;
    let mut bytes_after_press = 0usize;
    while pressed.elapsed() < Duration::from_secs(10) {
        bytes_after_press += drain_for(&receiver, &mut parser, Duration::from_millis(100));
        if status_row(&parser).contains('\u{2161}') {
            seen_at = Some(pressed.elapsed());
            break;
        }
    }
    writer.write_all(b"q").unwrap();
    let _ = child.wait();
    let latency = seen_at.unwrap_or_else(|| panic!("pause glyph never appeared; status {:?}", status_row(&parser)));
    eprintln!("pause visible after {latency:?} and {bytes_after_press} B");
    // 50 KiB/s drain; two full 200x50 frames are under 100 KiB.
    assert!(
        latency <= Duration::from_millis(2000),
        "pause took {latency:?} and {bytes_after_press} bytes to reach the screen"
    );
}
