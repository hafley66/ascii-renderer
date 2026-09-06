//! Input stays in the terminal process. A killable process group owns rendering,
//! encoding, caches and any render subprocesses. Pipes provide backpressure.
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use nix::fcntl::{FcntlArg, OFlag, fcntl};
use nix::poll::{PollFd, PollFlags, poll};
use nix::sys::signal::{Signal, killpg};
use nix::sys::termios::{FlushArg, tcflush};
use nix::unistd::Pid;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::{fs::OpenOptionsExt, process::CommandExt};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const OUTPUT_BYTES: usize = 32 * 1024;
const INPUT_EVENTS: usize = 32;
const INPUT_POLL: Duration = Duration::from_millis(2);

#[derive(Debug, PartialEq)]
pub(crate) enum Exit {
    Finished,
    Quit,
    Interrupt,
    Input(Event),
}

fn quit(event: &Event) -> Option<Exit> {
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
            KeyCode::Char('c' | 'C') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(Exit::Interrupt)
            }
            KeyCode::Char('q' | 'Q') | KeyCode::Esc => Some(Exit::Quit),
            _ => None,
        },
        _ => None,
    }
}

// The child remains unreaped until this guard kills its whole group. This also
// prevents nested render_frame children from surviving an interrupted session.
struct Job(Child);
impl Drop for Job {
    fn drop(&mut self) {
        let _ = killpg(Pid::from_raw(self.0.id() as i32), Signal::SIGKILL);
        let _ = self.0.wait();
    }
}

fn nonblocking(fd: impl AsFd) -> io::Result<()> {
    let flags = OFlag::from_bits_truncate(fcntl(&fd, FcntlArg::F_GETFL)?);
    fcntl(fd, FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK))?;
    Ok(())
}

pub(crate) fn supervise(command: &mut Command, animation: bool) -> io::Result<Exit> {
    // A distinct open file description avoids changing the caller's stdout flags.
    let mut terminal = OpenOptions::new()
        .write(true)
        .custom_flags(OFlag::O_NONBLOCK.bits())
        .open("/dev/tty")?;
    let readiness = terminal.try_clone()?;
    let result = pump(
        command,
        animation,
        &mut terminal,
        Some(readiness.as_fd()),
        || {
            if event::poll(Duration::ZERO)? {
                event::read().map(Some)
            } else {
                Ok(None)
            }
        },
    );
    // Discard queued frame bytes on cancellation before the caller redraws or
    // leaves alternate screen. Do not wait for a congested terminal to drain.
    if !matches!(result, Ok(Exit::Finished)) {
        let _ = tcflush(&terminal, FlushArg::TCOFLUSH);
    }
    // A partial ANSI frame can end inside a CSI sequence. CAN cancels it; reset
    // attributes and cursor without acquiring the worker's stdout lock.
    let _ = terminal.write_all(b"\x18\x1b[0m\x1b[?25h");
    result
}

fn pump(
    command: &mut Command,
    animation: bool,
    terminal: &mut impl Write,
    terminal_fd: Option<BorrowedFd<'_>>,
    mut input: impl FnMut() -> io::Result<Option<Event>>,
) -> io::Result<Exit> {
    command
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut job = Job(command.spawn()?);
    let mut output = job.0.stdout.take().unwrap();
    let mut controls = job.0.stdin.take().unwrap();
    nonblocking(&output)?;
    nonblocking(&controls)?;
    let mut pending = VecDeque::<Vec<u8>>::new();
    let mut sending = Vec::new();
    let mut sent = 0;
    let mut bytes = vec![0u8; OUTPUT_BYTES];
    let mut display = Vec::with_capacity(OUTPUT_BYTES * 2);
    let mut displayed = 0;
    let mut eof = false;
    loop {
        // Quit bypasses the control queue even when rendering and output stall.
        // Drain a bounded batch so key auto-repeat cannot starve output forever.
        for _ in 0..256 {
            let Some(event) = input()? else {
                break;
            };
            if let Some(exit) = quit(&event) {
                return Ok(exit);
            }
            if !matches!(event, Event::Key(_) | Event::Resize(_, _)) {
                continue;
            }
            if !animation {
                return Ok(Exit::Input(event));
            }
            let mut encoded = serde_json::to_vec(&event)?;
            encoded.push(b'\n');
            // Under saturation retain recent controls. Never drop a partial
            // record already being written, and never enqueue quit commands.
            if pending.len() == INPUT_EVENTS {
                pending.pop_front();
            }
            pending.push_back(encoded);
        }
        for _ in 0..INPUT_EVENTS {
            if sent == sending.len() {
                sending = pending.pop_front().unwrap_or_default();
                sent = 0;
            }
            if sent < sending.len() {
                match controls.write(&sending[sent..]) {
                    Ok(n) => sent += n,
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) =>
                    {
                        break;
                    }
                    Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {}
                    Err(e) => return Err(e),
                }
            }
            if sending.is_empty() {
                break;
            }
        }
        let mut progressed = false;
        if displayed == display.len() && !eof {
            match output.read(&mut bytes) {
                Ok(0) => eof = true,
                Ok(n) => {
                    display.clear();
                    displayed = 0;
                    for &byte in &bytes[..n] {
                        // Preview CLI output uses LF, while raw terminal mode
                        // requires explicit carriage return. Worker output is ANSI.
                        if !animation && byte == b'\n' {
                            display.push(b'\r');
                        }
                        display.push(byte);
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(e),
            }
        }
        if displayed < display.len() {
            match terminal.write(&display[displayed..]) {
                Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero)),
                Ok(n) => {
                    displayed += n;
                    progressed = true;
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(e),
            }
        }
        if eof && displayed == display.len() {
            return Ok(Exit::Finished);
        }
        if !progressed {
            // Wait for the blocked stage, waking as soon as it can progress.
            // The timeout bounds keyboard latency even without descriptor input.
            let waiting = if displayed == display.len() {
                Some(PollFd::new(output.as_fd(), PollFlags::POLLIN))
            } else {
                terminal_fd.map(|fd| PollFd::new(fd, PollFlags::POLLOUT))
            };
            if let Some(fd) = waiting {
                match poll(&mut [fd], INPUT_POLL.as_millis() as u16) {
                    Ok(_) | Err(nix::errno::Errno::EINTR) => (),
                    Err(error) => return Err(error.into()),
                }
            } else {
                // In-memory test writers have no readiness descriptor.
                std::thread::sleep(INPUT_POLL);
            }
        }
    }
}

pub(crate) fn animate(
    mode_a: &str,
    seed_a: u64,
    mode_b: &str,
    seed_b: u64,
    strat: &str,
    theme: &str,
) -> io::Result<Exit> {
    let (w, h) = crossterm::terminal::size()?;
    let mut command = Command::new(std::env::current_exe()?);
    command.args([
        "--animation-worker",
        mode_a,
        &seed_a.to_string(),
        mode_b,
        &seed_b.to_string(),
        strat,
        theme,
        &w.to_string(),
        &h.to_string(),
    ]);
    crate::opts::live_params_to_command(&mut command);
    supervise(&mut command, true)
}

pub(crate) fn worker(args: &[String]) {
    use std::io::BufRead;
    if args.len() != 10 {
        return;
    }
    // A stalled reader must not prevent the render thread from applying input.
    nonblocking(io::stdout()).expect("nonblocking animation output");
    let (sender, receiver) = std::sync::mpsc::sync_channel(INPUT_EVENTS);
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let Ok(line) = line else {
                break;
            };
            let Ok(event) = serde_json::from_str::<Event>(&line) else {
                break;
            };
            if sender.send(event).is_err() {
                break;
            }
        }
    });
    crate::morph::morph_worker_session(
        &args[2],
        args[3].parse().unwrap_or(42),
        &args[4],
        args[5].parse().unwrap_or(43),
        &args[6],
        &args[7],
        Some(receiver),
        Some((args[8].parse().unwrap_or(80), args[9].parse().unwrap_or(45))),
    );
}

/// Write bounded chunks while checking controls. An interrupted frame is
/// abandoned; the caller invalidates its encoder and starts a full repaint.
pub(crate) fn write_frame(
    output: &mut (impl Write + AsFd),
    mut bytes: &[u8],
    input: &std::sync::mpsc::Receiver<Event>,
    events: &mut Vec<Event>,
) -> io::Result<bool> {
    while !bytes.is_empty() {
        for _ in 0..INPUT_EVENTS {
            match input.try_recv() {
                Ok(event) => events.push(event),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Err(io::ErrorKind::BrokenPipe.into());
                }
            }
        }
        if !events.is_empty() {
            return Ok(false);
        }
        match output.write(&bytes[..bytes.len().min(OUTPUT_BYTES)]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(n) => bytes = &bytes[n..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                // Wake immediately when the pipe accepts bytes. The timeout
                // bounds control latency while the pipe remains saturated.
                match poll(
                    &mut [PollFd::new(output.as_fd(), PollFlags::POLLOUT)],
                    INPUT_POLL.as_millis() as u16,
                ) {
                    Ok(_) | Err(nix::errno::Errno::EINTR) => (),
                    Err(error) => return Err(error.into()),
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    #[test]
    fn controls_interrupt_partial_frames_and_disconnected_workers_stop() {
        struct PartialWriter {
            readiness: std::fs::File,
            sender: std::sync::mpsc::Sender<Event>,
            written: Vec<u8>,
            blocked: bool,
        }
        impl AsFd for PartialWriter {
            fn as_fd(&self) -> BorrowedFd<'_> {
                self.readiness.as_fd()
            }
        }
        impl Write for PartialWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.blocked {
                    self.sender
                        .send(Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)))
                        .unwrap();
                    return Err(io::ErrorKind::WouldBlock.into());
                }
                self.blocked = true;
                self.written.extend_from_slice(&bytes[..3]);
                Ok(3)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut writer = PartialWriter {
            readiness: OpenOptions::new().write(true).open("/dev/null").unwrap(),
            sender,
            written: Vec::new(),
            blocked: false,
        };
        let mut events = Vec::new();
        assert!(!write_frame(&mut writer, b"abcdef", &receiver, &mut events).unwrap());
        assert_eq!(writer.written, b"abc");
        assert_eq!(
            events,
            [Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))]
        );

        let mut file = tempfile::tempfile().unwrap();
        assert!(
            write_frame(
                &mut file,
                b"\x18complete replacement",
                &receiver,
                &mut Vec::new()
            )
            .unwrap()
        );
        use std::io::{Seek, SeekFrom};
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"\x18complete replacement");
        drop(writer);
        assert_eq!(
            write_frame(&mut file, b"orphan", &receiver, &mut Vec::new())
                .unwrap_err()
                .kind(),
            io::ErrorKind::BrokenPipe
        );
    }

    /// Complete CLI frames through the same pipe and nonblocking PTY relay as demo.
    #[test]
    #[ignore = "release relay probe; ASCII_PERF_BIN selects the renderer executable"]
    fn perf_preview_relay_over_time() {
        let binary = std::env::var("ASCII_PERF_BIN").expect("set ASCII_PERF_BIN");
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("renders.ndjson");
        for (w, h) in [(320, 103), (2000, 2000)] {
            let pty = nix::pty::openpty(None, None).unwrap();
            let mut attrs = nix::sys::termios::tcgetattr(&pty.slave).unwrap();
            nix::sys::termios::cfmakeraw(&mut attrs);
            nix::sys::termios::tcsetattr(&pty.slave, nix::sys::termios::SetArg::TCSANOW, &attrs)
                .unwrap();
            let mut terminal = std::fs::File::from(pty.slave);
            nonblocking(&terminal).unwrap();
            let reader = std::thread::spawn(move || {
                let mut master = std::fs::File::from(pty.master);
                let mut buffer = [0; 65536];
                let mut total = 0usize;
                loop {
                    match master.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => total += n,
                        Err(e) if e.raw_os_error() == Some(nix::libc::EIO) => break,
                        Err(e) => panic!("{e}"),
                    }
                }
                total
            });
            let readiness = terminal.try_clone().unwrap();
            for frame in 0..12 {
                let time = frame as f32 * 0.06;
                let mut command = Command::new(&binary);
                command
                    .args(["42", "gem-aetherium-2"])
                    .env("ASCII_GRID_W", w.to_string())
                    .env("ASCII_GRID_H", h.to_string())
                    .env("ASCII_T", time.to_string())
                    .env("ASCII_TRACE_ALL", "1")
                    .env("ASCII_TRACE_PATH", &path);
                let started = Instant::now();
                assert_eq!(
                    pump(
                        &mut command,
                        false,
                        &mut terminal,
                        Some(readiness.as_fd()),
                        || Ok(None)
                    )
                    .unwrap(),
                    Exit::Finished
                );
                let log = std::fs::read_to_string(&path).unwrap();
                let event: serde_json::Value =
                    serde_json::from_str(log.lines().last().unwrap()).unwrap();
                eprintln!(
                    "{w}x{h} t={time:.2} relay_ms={:.3} render_us={} emit_us={}",
                    started.elapsed().as_secs_f64() * 1000.0,
                    event["render_us"],
                    event["emit_us"]
                );
            }
            drop(readiness);
            drop(terminal);
            eprintln!("{w}x{h} bytes={}", reader.join().unwrap());
        }
    }

    struct Congested;
    impl Write for Congested {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::WouldBlock.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn quit_interrupts_cpu_sleep_and_output_backpressure() {
        for script in ["while :; do :; done", "sleep 10", "yes frame"] {
            for (code, modifiers, expected) in [
                ('q', KeyModifiers::NONE, Exit::Quit),
                ('Q', KeyModifiers::SHIFT, Exit::Quit),
                ('c', KeyModifiers::CONTROL, Exit::Interrupt),
            ] {
                let started = Instant::now();
                let mut queued = 0;
                let result = pump(
                    Command::new("sh").args(["-c", script]),
                    true,
                    &mut Congested,
                    None,
                    || {
                        // Flood ordinary controls first, then quit while the child
                        // cannot consume/render them. Quit must bypass the queue.
                        if queued < 512 {
                            queued += 1;
                            return Ok(Some(Event::Key(KeyEvent::new(
                                KeyCode::Right,
                                KeyModifiers::NONE,
                            ))));
                        }
                        Ok((started.elapsed() >= Duration::from_millis(40))
                            .then(|| Event::Key(KeyEvent::new(KeyCode::Char(code), modifiers))))
                    },
                )
                .unwrap();
                assert_eq!(result, expected, "{script}");
                assert!(
                    started.elapsed() < Duration::from_secs(1),
                    "{script}: {:?}",
                    started.elapsed()
                );
            }
        }
    }

    #[test]
    fn quit_interrupts_real_terminal_backpressure() {
        let pty = nix::pty::openpty(None, None).unwrap();
        let mut terminal = std::fs::File::from(pty.slave);
        nonblocking(&terminal).unwrap();
        let readiness = terminal.try_clone().unwrap();
        // Keep the master open without draining it so POLLOUT remains blocked.
        let started = Instant::now();
        let result = pump(
            Command::new("sh").args(["-c", "yes frame"]),
            true,
            &mut terminal,
            Some(readiness.as_fd()),
            || {
                Ok((started.elapsed() >= Duration::from_millis(40))
                    .then(|| Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))))
            },
        )
        .unwrap();
        assert_eq!(result, Exit::Quit);
        assert!(started.elapsed() < Duration::from_secs(1));
        drop(pty.master);
    }

    #[test]
    fn cancellation_kills_nested_render_children() {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join("ready");
        let leaked = directory.path().join("leaked");
        let started = Instant::now();
        let result = pump(
            Command::new("sh")
                .args([
                    "-c",
                    "(touch \"$1\"; sleep 0.2; touch \"$2\") & wait",
                    "test",
                ])
                .arg(&ready)
                .arg(&leaked),
            true,
            &mut Vec::new(),
            None,
            || {
                assert!(
                    started.elapsed() < Duration::from_secs(2),
                    "descendant did not start"
                );
                Ok(ready
                    .exists()
                    .then(|| Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))))
            },
        )
        .unwrap();
        assert_eq!(result, Exit::Quit);
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            !leaked.exists(),
            "nested render child survived cancellation"
        );
    }

    #[test]
    fn preview_navigation_cancels_pending_render() {
        let key = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        let result = pump(
            Command::new("sh").args(["-c", "sleep 10"]),
            false,
            &mut Vec::new(),
            None,
            || Ok(Some(key.clone())),
        )
        .unwrap();
        assert_eq!(result, Exit::Input(key));
    }

    #[test]
    fn preview_preserves_output_and_converts_newlines() {
        let mut output = Vec::new();
        let result = pump(
            Command::new("sh").args(["-c", "printf 'one\\ntwo\\n'"]),
            false,
            &mut output,
            None,
            || Ok(None),
        )
        .unwrap();
        assert_eq!(
            (result, output),
            (Exit::Finished, b"one\r\ntwo\r\n".to_vec())
        );
    }
}
