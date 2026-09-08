//! Same Gem 2 scene, three independently owned library presentation paths.
use crate::{
    registry::{ModeFrame, registered_mode},
    types::{Cell, Grid},
};
use crossterm::{
    event::{self, Event, KeyCode},
    style::Color,
};
use rand::{SeedableRng, rngs::StdRng};
use ratatui::{Terminal, TerminalOptions, Viewport, backend::TermionBackend, layout::Rect};
use std::{
    io::{self, BufWriter},
    path::PathBuf,
    time::{Duration, Instant},
};
use termwiz::{
    surface::{Change, Position, Surface},
    terminal::{SystemTerminal, Terminal as WizTerminal, buffered::BufferedTerminal},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub(crate) fn command(args: &[String]) -> bool {
    if args.get(1).map(String::as_str) != Some("gem-lab") {
        return false;
    }
    if let Err(error) = run(args) {
        eprintln!("gem-lab: {error}");
        std::process::exit(2);
    }
    true
}

// The scene grid, RNG seed and explicit knob vector live in this loop. Each
// backend retains its own library buffers for the complete run. No app relay.
fn run(args: &[String]) -> Result<()> {
    if args.len() < 5 || args.len() > 9 {
        return Err("usage: gem-lab ratatui|console|termwiz FRAMES LOG [INPUTS.json|max] [WIDTH HEIGHT] [hold]".into());
    }
    let backend = args[2].as_str();
    if !["ratatui", "console", "termwiz"].contains(&backend) {
        return Err("unknown backend".into());
    }
    let frames: usize = args[3].parse()?;
    if !(2..=300).contains(&frames) {
        return Err("frame limit: 2..300".into());
    }
    let terminal_size = crossterm::terminal::size()?;
    let width = args
        .get(6)
        .map(|v| v.parse())
        .transpose()?
        .unwrap_or(terminal_size.0.saturating_sub(34));
    let height = args
        .get(7)
        .map(|v| v.parse())
        .transpose()?
        .unwrap_or(terminal_size.1.saturating_sub(1));
    if width == 0
        || height == 0
        || u32::from(terminal_size.0) * u32::from(terminal_size.1) > 80_000
        || width > terminal_size.0
        || height >= terminal_size.1
    {
        return Err(
            "limit: 80,000 terminal cells; art must fit with one spare terminal row".into(),
        );
    }
    #[cfg(not(feature = "gem-lab-only"))]
    let mode = registered_mode("gem-aetherium-2").unwrap();
    #[cfg(feature = "gem-lab-only")]
    let mode: &dyn crate::registry::Mode = &crate::_50_lab_scene::MODE;
    let fixture: serde_json::Value = match args.get(5).map(String::as_str) {
        Some("max") | None => serde_json::from_str(include_str!(
            "../perf/fixtures/12_gem_aetherium_2_bad_roll6.json"
        ))?,
        Some(path) => serde_json::from_reader(std::fs::File::open(path)?)?,
    };
    if fixture["mode"] != mode.name() {
        return Err("input fixture must be gem-aetherium-2".into());
    }
    let values: Vec<f32> = mode
        .params()
        .iter()
        .map(|p| {
            if args.get(5).map(String::as_str) == Some("max") {
                Ok(p.max)
            } else {
                fixture["knobs"][p.key]
                    .as_f64()
                    .map(|v| v as f32)
                    .filter(|v| v.is_finite() && *v >= p.min && *v <= p.max)
                    .ok_or_else(|| format!("missing or out-of-range knob {}", p.key))
            }
        })
        .collect::<std::result::Result<_, _>>()?;
    let seed = fixture["seed"].as_u64().ok_or("missing seed")?;
    let palette = if fixture["palette"].is_null() {
        crate::color::make_palette(seed)
    } else {
        serde_json::from_value(fixture["palette"].clone())?
    };
    let t0 = fixture["time"].as_f64().unwrap_or(0.0) as f32;
    if !t0.is_finite() {
        return Err("invalid time".into());
    }
    let log = PathBuf::from(&args[4]);
    let (writer, _writer_guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
        .buffered_lines_limit(64)
        .lossy(false)
        .finish(BufWriter::new(std::fs::File::create(&log)?));
    let subscriber = tracing_subscriber::fmt()
        .json()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(writer)
        .finish();
    let _subscriber = tracing::subscriber::set_default(subscriber);
    let inputs = crate::_0_profile::FrameInputs {
        mode: mode.name(),
        theme: "",
        seed,
        width: width as usize,
        height: height as usize,
        terminal_size: Some(terminal_size),
        time: t0,
        args: &[],
        params: mode.params(),
        values: &values,
        palette: &palette,
    }
    .to_json();
    tracing::info!(kind="inputs", ts_ms=crate::_0_profile::unix_ms(), backend, inputs=%inputs, "gem lab");
    let mut grid = vec![vec![Cell::blank(); width as usize]; height as usize];
    let mut output = Output::new(backend, width, height)?;
    let epoch = Instant::now();
    let result = (|| -> Result<()> {
        for index in 0..frames {
            if crossterm::terminal::size()? != terminal_size {
                return Err("terminal resized; repeat all backends at the new size".into());
            }
            let start = Instant::now();
            for row in &mut grid {
                row.fill(Cell::blank());
            }
            let time = t0 + index as f32 / 60.0;
            let mut rng = StdRng::seed_from_u64(seed);
            mode.render(&mut ModeFrame {
                grid: &mut grid,
                width: width as usize,
                height: height as usize,
                seed,
                palette: &palette,
                rng: &mut rng,
                time,
                args: &[],
                param_values: Some(&values),
            });
            let render_us = start.elapsed().as_micros() as u64;
            // Apply exactly the existing demo's palette quantization to all paths.
            let normalize = Instant::now();
            for cell in grid.iter_mut().flatten() {
                cell.fg = if cell.ch == ' ' {
                    Color::Reset
                } else {
                    crate::gridio::terminal_color(cell.fg)
                };
                cell.bg = crate::gridio::terminal_color(cell.bg);
            }
            reserve_wide_columns(&mut grid);
            let normalize_us = normalize.elapsed().as_micros() as u64;
            let (adapt_us, present_us) = output.draw(&grid)?;
            tracing::info!(
                kind = "frame",
                ts_ms = crate::_0_profile::unix_ms(),
                backend,
                frame = index,
                time,
                width,
                height,
                render_us,
                normalize_us,
                adapt_us,
                present_us,
                total_us = start.elapsed().as_micros() as u64,
                elapsed_us = epoch.elapsed().as_micros() as u64,
                "gem lab"
            );
            let wait = Duration::from_secs_f64(1.0 / 60.0).saturating_sub(start.elapsed());
            if quit(wait)? {
                break;
            }
        }
        // Canonical final scene evidence is written outside the measured frames.
        let cells: Vec<Vec<_>> = grid
            .iter()
            .map(|row| {
                row.iter()
                    .map(|c| {
                        let [fg, bg] = [c.fg, c.bg].map(|color| match wiz_color(color) {
                            termwiz::color::ColorAttribute::PaletteIndex(index) => Some(index),
                            termwiz::color::ColorAttribute::Default => None,
                            _ => unreachable!("all scene colors were normalized to indexed colors"),
                        });
                        (c.ch, fg, bg)
                    })
                    .collect()
            })
            .collect();
        serde_json::to_writer(
            BufWriter::new(std::fs::File::create(log.with_extension("grid"))?),
            &cells,
        )?;
        tracing::info!(
            kind = "complete",
            ts_ms = crate::_0_profile::unix_ms(),
            backend,
            "gem lab"
        );
        if args.get(8).map(String::as_str) == Some("hold") {
            quit(Duration::from_secs(2))?;
        }
        Ok(())
    })();
    drop(output);
    ratatui::restore();
    result
}

fn quit(wait: Duration) -> io::Result<bool> {
    Ok(event::poll(wait)?
        && matches!(event::read()?, Event::Key(key) if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)))
}

fn reserve_wide_columns(grid: &mut Grid) {
    for row in grid {
        let mut x = 0;
        while x < row.len() {
            let width = unicode_width::UnicodeWidthChar::width(row[x].ch).unwrap_or(0);
            if width == 0 || x + width > row.len() {
                row[x].ch = ' ';
                row[x].fg = Color::Reset;
                x += 1;
                continue;
            }
            let bg = row[x].bg;
            for trailing in &mut row[x + 1..x + width] {
                *trailing = Cell {
                    ch: ' ',
                    fg: Color::Reset,
                    bg,
                };
            }
            x += width;
        }
    }
}

enum Output {
    Ratatui(Terminal<TermionBackend<BufWriter<io::Stdout>>>),
    Console(console_engine::ConsoleEngine),
    Termwiz {
        terminal: BufferedTerminal<SystemTerminal>,
        scene: Surface,
    },
}
impl Output {
    fn new(name: &str, width: u16, height: u16) -> Result<Self> {
        Ok(match name {
            "ratatui" => {
                drop(ratatui::init());
                Self::Ratatui(Terminal::with_options(
                    TermionBackend::new(BufWriter::with_capacity(65536, io::stdout())),
                    TerminalOptions {
                        viewport: Viewport::Fixed(Rect::new(0, 0, width, height)),
                    },
                )?)
            }
            "console" => Self::Console(console_engine::ConsoleEngine::init(
                width.into(),
                height.into(),
                60,
            )?),
            "termwiz" => {
                let mut terminal =
                    SystemTerminal::new(termwiz::caps::Capabilities::new_from_env()?)?;
                terminal.set_raw_mode()?;
                terminal.enter_alternate_screen()?;
                Self::Termwiz {
                    terminal: BufferedTerminal::new(terminal)?,
                    scene: Surface::new(width as usize, height as usize),
                }
            }
            _ => unreachable!(),
        })
    }
    fn draw(&mut self, grid: &Grid) -> Result<(u64, u64)> {
        let start = Instant::now();
        let adapt_us;
        match self {
            Self::Ratatui(terminal) => {
                let mut adapted = 0;
                terminal.draw(|frame| {
                    let before = Instant::now();
                    for (cell, target) in grid.iter().flatten().zip(&mut frame.buffer_mut().content)
                    {
                        target
                            .set_char(cell.ch)
                            .set_fg(crate::gridio::ratatui_color(cell.fg))
                            .set_bg(crate::gridio::ratatui_color(cell.bg));
                    }
                    adapted = before.elapsed().as_micros() as u64;
                })?;
                adapt_us = adapted;
            }
            Self::Console(engine) => {
                for (y, row) in grid.iter().enumerate() {
                    for (x, cell) in row.iter().enumerate() {
                        engine.set_pxl(
                            x as i32,
                            y as i32,
                            console_engine::pixel::Pixel {
                                chr: cell.ch,
                                fg: cell.fg,
                                bg: cell.bg,
                            },
                        );
                    }
                }
                adapt_us = start.elapsed().as_micros() as u64;
                engine.draw();
            }
            Self::Termwiz { terminal, scene } => {
                populate_surface(scene, grid);
                adapt_us = start.elapsed().as_micros() as u64;
                terminal.draw_from_screen(scene, 0, 0);
                terminal.flush()?;
            }
        }
        Ok((
            adapt_us,
            (start.elapsed().as_micros() as u64).saturating_sub(adapt_us),
        ))
    }
}

fn populate_surface(surface: &mut Surface, grid: &Grid) {
    // Batch adjacent equal-style cells into library Text changes for each row.
    for (y, row) in grid.iter().enumerate() {
        let mut changes = vec![Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Absolute(y),
        }];
        let mut x = 0;
        while x < row.len() {
            let cell = row[x];
            let mut attrs = termwiz::cell::CellAttributes::default();
            attrs
                .set_foreground(wiz_color(cell.fg))
                .set_background(wiz_color(cell.bg));
            let mut text = String::new();
            while x < row.len() && row[x].fg == cell.fg && row[x].bg == cell.bg {
                text.push(row[x].ch);
                x += unicode_width::UnicodeWidthChar::width(row[x].ch)
                    .unwrap_or(1)
                    .max(1);
            }
            changes.push(Change::AllAttributes(attrs));
            changes.push(Change::Text(text));
        }
        let seq = surface.add_changes(changes);
        surface.flush_changes_older_than(seq);
    }
}

fn wiz_color(color: Color) -> termwiz::color::ColorAttribute {
    use termwiz::color::ColorAttribute as W;
    match color {
        Color::Reset => W::Default,
        Color::AnsiValue(index) => W::PaletteIndex(index),
        Color::Black => W::PaletteIndex(0),
        Color::DarkRed => W::PaletteIndex(1),
        Color::DarkGreen => W::PaletteIndex(2),
        Color::DarkYellow => W::PaletteIndex(3),
        Color::DarkBlue => W::PaletteIndex(4),
        Color::DarkMagenta => W::PaletteIndex(5),
        Color::DarkCyan => W::PaletteIndex(6),
        Color::Grey => W::PaletteIndex(7),
        Color::DarkGrey => W::PaletteIndex(8),
        Color::Red => W::PaletteIndex(9),
        Color::Green => W::PaletteIndex(10),
        Color::Yellow => W::PaletteIndex(11),
        Color::Blue => W::PaletteIndex(12),
        Color::Magenta => W::PaletteIndex(13),
        Color::Cyan => W::PaletteIndex(14),
        Color::White => W::PaletteIndex(15),
        Color::Rgb { .. } => wiz_color(crate::gridio::terminal_color(color)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wide_glyphs_reserve_columns_and_clip_at_the_edge() {
        let mut grid = vec![
            "♌x♍"
                .chars()
                .map(|ch| Cell::new(ch, Color::AnsiValue(42)))
                .collect(),
        ];
        reserve_wide_columns(&mut grid);
        assert_eq!(grid[0].iter().map(|c| c.ch).collect::<String>(), "♌  ");
        assert_eq!(grid[0][1].fg, Color::Reset);
        assert_eq!(grid[0][2].fg, Color::Reset);
    }
    #[test]
    fn termwiz_surface_preserves_rows_colors_and_repeated_frames() {
        let mut surface = Surface::new(3, 2);
        let grid = vec![
            vec![
                Cell {
                    ch: 'a',
                    fg: Color::AnsiValue(42),
                    bg: Color::AnsiValue(16)
                };
                3
            ],
            vec![
                Cell {
                    ch: 'b',
                    fg: Color::AnsiValue(99),
                    bg: Color::Reset
                };
                3
            ],
        ];
        for _ in 0..3 {
            populate_surface(&mut surface, &grid);
            assert_eq!(surface.screen_chars_to_string(), "aaa\nbbb\n");
            let cells = surface.screen_cells();
            assert_eq!(
                cells[0][0].attrs().foreground(),
                wiz_color(Color::AnsiValue(42))
            );
            assert_eq!(
                cells[1][2].attrs().foreground(),
                wiz_color(Color::AnsiValue(99))
            );
        }
    }
}
