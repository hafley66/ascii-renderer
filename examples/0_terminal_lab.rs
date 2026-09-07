//! Small drawing-only lab: cargo run --release --example 0_terminal_lab -- ...
use std::io::{self, Write};
use std::time::Instant;
use ratatui::{Terminal, TerminalOptions, Viewport};
use ratatui::backend::{Backend, CrosstermBackend, TermionBackend};
use ratatui::layout::Rect;
use ratatui::style::Color;

fn run<B: Backend<Error = io::Error>>(
    backend: B, bytes: impl Fn(&mut B) -> &mut Vec<u8>,
    pattern: &str, width: u16, height: u16, frames: usize, emit: bool,
) -> io::Result<()> {
    let mut terminal = Terminal::with_options(backend, TerminalOptions {
        viewport: Viewport::Fixed(Rect::new(0, 0, width, height)),
    })?;
    let mut stdout = io::stdout().lock();
    let glyphs = b" .,:;ox%#@";
    for index in 0..frames {
        let start = Instant::now();
        let mut fill_us = 0;
        bytes(terminal.backend_mut()).clear();
        crossterm::queue!(bytes(terminal.backend_mut()), crossterm::terminal::BeginSynchronizedUpdate)?;
        terminal.draw(|frame| {
            let fill = Instant::now();
            let phase = if pattern == "static" { 0 } else { index };
            for (i, cell) in frame.buffer_mut().content.iter_mut().enumerate() {
                let x = i % usize::from(width);
                let y = i / usize::from(width);
                let active = pattern != "sparse" || (x + y * 3 + phase) % 31 == 0;
                let ch = if active { glyphs[(x / 2 + y + phase) % glyphs.len()] as char } else { ' ' };
                let fg = if pattern == "mono" { Color::Indexed(7) }
                else { Color::Indexed(16 + ((x + y * 7 + phase / 2) % 8) as u8 * 24) };
                cell.set_char(ch).set_fg(if ch == ' ' { Color::Reset } else { fg }).set_bg(Color::Reset);
            }
            fill_us = fill.elapsed().as_micros() as u64;
        })?;
        crossterm::queue!(bytes(terminal.backend_mut()), crossterm::terminal::EndSynchronizedUpdate)?;
        let prepare_us = start.elapsed().as_micros() as u64;
        let payload = bytes(terminal.backend_mut());
        let count = payload.len();
        let output_start = Instant::now();
        if emit {
            stdout.write_all(payload)?;
            stdout.flush()?;
        } else {
            std::hint::black_box(payload.as_slice());
        }
        let output_us = output_start.elapsed().as_micros() as u64;
        tracing::info!(frame = index, width, height, pattern, bytes = count,
            fill_us, library_us = prepare_us.saturating_sub(fill_us), output_us,
            total_us = start.elapsed().as_micros() as u64, "lab frame");
    }
    bytes(terminal.backend_mut()).clear();
    terminal.show_cursor()?;
    if emit {
        stdout.write_all(bytes(terminal.backend_mut()))?;
        stdout.flush()?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 7 {
        return Err("usage: 0_terminal_lab crossterm|termion static|sparse|mono|color WIDTH HEIGHT FRAMES memory|stdout LOG".into());
    }
    let width: u16 = args[2].parse()?;
    let height: u16 = args[3].parse()?;
    let frames: usize = args[4].parse()?;
    if width == 0 || height == 0 || u32::from(width)*u32::from(height) > 80_000 || !(2..=12).contains(&frames) {
        return Err("lab limit: 80,000 cells, 2..12 frames".into());
    }
    if !["static", "sparse", "mono", "color"].contains(&args[1].as_str()) || !["memory", "stdout"].contains(&args[5].as_str()) {
        return Err("unknown pattern or output target".into());
    }
    let file = std::fs::File::create(&args[6])?;
    let (writer, _guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
        .buffered_lines_limit(64).lossy(false).finish(io::BufWriter::new(file));
    tracing_subscriber::fmt().json().with_ansi(false).with_writer(writer).init();
    ratatui::crossterm::style::force_color_output(true);
    match args[0].as_str() {
        "crossterm" => run(CrosstermBackend::new(Vec::with_capacity(2*1024*1024)), |b| b.writer_mut(),
            &args[1], width, height, frames, args[5] == "stdout")?,
        "termion" => run(TermionBackend::new(Vec::with_capacity(2*1024*1024)), |b| b.writer_mut(),
            &args[1], width, height, frames, args[5] == "stdout")?,
        _ => return Err("unknown backend".into()),
    }
    Ok(())
}
