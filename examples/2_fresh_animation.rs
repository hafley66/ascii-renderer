//! Independent animation: Ratatui -> buffered stdout -> terminal.
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::style::Color;
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use std::{
    io::{self, BufWriter},
    time::{Duration, Instant},
};

fn draw(frame: &mut Frame<'_>, t: f32) {
    // Recompute a parametric pattern directly into Ratatui's retained buffer.
    let area = frame.area();
    let glyphs = b" .,:;ox%#@";
    for (i, cell) in frame.buffer_mut().content.iter_mut().enumerate() {
        let x = (i % usize::from(area.width)) as f32;
        let y = (i / usize::from(area.width)) as f32;
        let wave = ((x * 0.07 + t).sin() + (y * 0.13 - t * 1.3).cos()) * 0.25 + 0.5;
        let shade = (wave * 9.0).round() as usize;
        cell.set_char(glyphs[shade] as char)
            .set_fg(Color::Indexed(16 + shade as u8 * 24))
            .set_bg(Color::Reset);
    }
}

fn run(frames: usize) -> io::Result<()> {
    // One terminal owns the two diff buffers for the entire animation.
    let output = BufWriter::with_capacity(64 * 1024, io::stdout());
    let mut terminal = Terminal::new(CrosstermBackend::new(output))?;
    let epoch = Instant::now();
    for index in 0..frames {
        let size = terminal.size()?;
        if size.width == 0
            || size.height == 0
            || u32::from(size.width) * u32::from(size.height) > 80_000
        {
            return Err(io::Error::other("animation limit: 80,000 terminal cells"));
        }
        let start = Instant::now();
        terminal.draw(|frame| draw(frame, index as f32 / 60.0))?;
        tracing::info!(
            frame = index,
            width = size.width,
            height = size.height,
            draw_us = start.elapsed().as_micros() as u64,
            elapsed_us = epoch.elapsed().as_micros() as u64,
            "fresh frame"
        );
        let wait = Duration::from_secs_f64(1.0 / 60.0).saturating_sub(start.elapsed());
        if event::poll(wait)?
            && matches!(event::read()?, Event::Key(key) if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
        {
            break;
        }
    }
    terminal.show_cursor()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: 2_fresh_animation FRAMES LOG.ndjson (q exits)".into());
    }
    let frames: usize = args[0].parse()?;
    if !(2..=300).contains(&frames) {
        return Err("frame limit: 2..300".into());
    }
    let (writer, _guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
        .buffered_lines_limit(64)
        .lossy(false)
        .finish(BufWriter::new(std::fs::File::create(&args[1])?));
    tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .with_writer(writer)
        .init();
    ratatui::crossterm::style::force_color_output(true);
    let (width, height) = ratatui::crossterm::terminal::size()?;
    if width == 0 || height == 0 || u32::from(width) * u32::from(height) > 80_000 {
        return Err("animation limit: 80,000 terminal cells".into());
    }
    drop(ratatui::init()); // Library panic hook and terminal lifecycle setup.
    let result = run(frames);
    ratatui::restore();
    result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    #[test]
    fn deterministic_motion() {
        let mut terminal = Terminal::new(TestBackend::new(48, 16)).unwrap();
        terminal.draw(|f| draw(f, 0.0)).unwrap();
        let first = terminal.backend().buffer().clone();
        terminal.draw(|f| draw(f, 0.0)).unwrap();
        assert_eq!(&first, terminal.backend().buffer());
        terminal.draw(|f| draw(f, 1.0)).unwrap();
        assert_ne!(&first, terminal.backend().buffer());
    }
}
