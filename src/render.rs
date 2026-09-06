use crate::types::*;
use crossterm::style::Color;
use std::io::{self, Write};

/// Render grid to plain text (no ANSI escapes).
pub fn grid_to_plain(grid: &Grid) -> Vec<String> {
    let mut lines = Vec::with_capacity(grid.len());
    for row in grid {
        let mut line = String::with_capacity(row.len());
        let mut skip_next = false;
        for cell in row {
            if skip_next {
                skip_next = false;
                continue;
            }
            line.push(cell.ch);
            if char_width(cell.ch) == 2 {
                skip_next = true;
            }
        }
        lines.push(line);
    }
    lines
}

// Two maximum-length RGB escapes (19 bytes each) plus a UTF-8 glyph.
const MAX_CELL_BYTES: usize = 42;
const ANSI_CHUNK_BYTES: usize = 64 * 1024;

/// Encode one-shot output with bounded scratch storage. The callback is the
/// write boundary: chunks contain complete cells/escapes and are at most 64 KiB.
/// Foreground persists across rows; background resets before every newline.
fn encode_grid_ansi(grid: &Grid, mut emit: impl FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
    let mut buffer = String::with_capacity(ANSI_CHUNK_BYTES);
    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;
    for row in grid {
        let mut skip_next = false;
        for cell in row {
            if skip_next {
                skip_next = false;
                continue;
            }
            if buffer.len() + MAX_CELL_BYTES > ANSI_CHUNK_BYTES {
                emit(buffer.as_bytes())?;
                buffer.clear();
            }
            if cell.fg != cur_fg {
                push_color(&mut buffer, cell.fg, true);
                cur_fg = cell.fg;
            }
            if cell.bg != cur_bg {
                push_color(&mut buffer, cell.bg, false);
                cur_bg = cell.bg;
            }
            buffer.push(cell.ch);
            skip_next = char_width(cell.ch) == 2;
        }
        // Room for a background reset and newline, including empty rows.
        if buffer.len() + 6 > ANSI_CHUNK_BYTES {
            emit(buffer.as_bytes())?;
            buffer.clear();
        }
        if cur_bg != Color::Reset {
            buffer.push_str("\x1b[49m");
            cur_bg = Color::Reset;
        }
        buffer.push('\n');
    }
    if buffer.len() + 4 > ANSI_CHUNK_BYTES {
        emit(buffer.as_bytes())?;
        buffer.clear();
    }
    buffer.push_str("\x1b[0m");
    emit(buffer.as_bytes())
}

fn push_u8(buffer: &mut String, value: u8) {
    if value >= 100 {
        buffer.push(char::from(b'0' + value / 100));
    }
    if value >= 10 {
        buffer.push(char::from(b'0' + value / 10 % 10));
    }
    buffer.push(char::from(b'0' + value % 10));
}

/// Same SGR bytes as crossterm 0.26, without nested formatting per cell.
fn push_color(buffer: &mut String, color: Color, foreground: bool) {
    if color == Color::Reset {
        buffer.push_str(if foreground { "\x1b[39m" } else { "\x1b[49m" });
        return;
    }
    buffer.push_str(if foreground { "\x1b[38;" } else { "\x1b[48;" });
    let index = match color {
        Color::Rgb { r, g, b } => {
            buffer.push_str("2;");
            push_u8(buffer, r);
            buffer.push(';');
            push_u8(buffer, g);
            buffer.push(';');
            push_u8(buffer, b);
            buffer.push('m');
            return;
        }
        Color::Black => 0,
        Color::DarkRed => 1,
        Color::DarkGreen => 2,
        Color::DarkYellow => 3,
        Color::DarkBlue => 4,
        Color::DarkMagenta => 5,
        Color::DarkCyan => 6,
        Color::Grey => 7,
        Color::DarkGrey => 8,
        Color::Red => 9,
        Color::Green => 10,
        Color::Yellow => 11,
        Color::Blue => 12,
        Color::Magenta => 13,
        Color::Cyan => 14,
        Color::White => 15,
        Color::AnsiValue(value) => value,
        Color::Reset => unreachable!(),
    };
    buffer.push_str("5;");
    push_u8(buffer, index);
    buffer.push('m');
}

/// Print the grid with ANSI color escape sequences.
pub fn render_grid(grid: &Grid) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    encode_grid_ansi(grid, |chunk| out.write_all(chunk)).unwrap();
    out.flush().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pre-optimization implementation, retained as a byte oracle and baseline.
    fn reference_ansi(grid: &Grid, writer: impl Write) {
        let mut out = io::BufWriter::new(writer);

        let mut cur_fg = Color::Reset;
        let mut cur_bg = Color::Reset;

        for row in grid {
            let mut skip_next = false;
            for cell in row {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                if cell.fg != cur_fg {
                    write!(out, "{}", crossterm::style::SetForegroundColor(cell.fg)).unwrap();
                    cur_fg = cell.fg;
                }
                if cell.bg != cur_bg {
                    write!(out, "{}", crossterm::style::SetBackgroundColor(cell.bg)).unwrap();
                    cur_bg = cell.bg;
                }
                write!(out, "{}", cell.ch).unwrap();
                if char_width(cell.ch) == 2 {
                    skip_next = true;
                }
            }
            if cur_bg != Color::Reset {
                write!(
                    out,
                    "{}",
                    crossterm::style::SetBackgroundColor(Color::Reset)
                )
                .unwrap();
                cur_bg = Color::Reset;
            }
            writeln!(out).unwrap();
        }

        write!(out, "{}", crossterm::style::ResetColor).unwrap();
        out.flush().unwrap();
    }

    fn colored_grid() -> Grid {
        (0..103)
            .map(|y| {
                (0..320)
                    .map(|x| {
                        Cell::with_bg(
                            ['x', '░', 'é', '·'][(x + y) % 4],
                            Color::Rgb {
                                r: x as u8,
                                g: y as u8,
                                b: (x + y) as u8,
                            },
                            Color::Rgb {
                                r: (x / 4) as u8,
                                g: (y * 2) as u8,
                                b: 255,
                            },
                        )
                    })
                    .collect()
            })
            .collect()
    }

    fn compare_reference(grid: &Grid) -> (usize, usize) {
        let mut expected = Vec::new();
        reference_ansi(grid, &mut expected);
        let mut actual = Vec::new();
        let mut chunks = 0;
        encode_grid_ansi(grid, |chunk| {
            assert!(chunk.len() <= ANSI_CHUNK_BYTES);
            // Chunk boundaries never split UTF-8 or an escape sequence.
            let text = std::str::from_utf8(chunk).unwrap();
            if let Some((_, escape)) = text.rsplit_once('\x1b') {
                assert!(escape.contains('m'));
            }
            chunks += 1;
            actual.extend_from_slice(chunk);
            Ok(())
        })
        .unwrap();
        assert_eq!(actual, expected);
        (actual.len(), chunks)
    }

    #[test]
    fn ansi_matches_reference_colors_and_unicode() {
        let mut colors = vec![
            Color::Reset,
            Color::Black,
            Color::DarkGrey,
            Color::Red,
            Color::DarkRed,
            Color::Green,
            Color::DarkGreen,
            Color::Yellow,
            Color::DarkYellow,
            Color::Blue,
            Color::DarkBlue,
            Color::Magenta,
            Color::DarkMagenta,
            Color::Cyan,
            Color::DarkCyan,
            Color::White,
            Color::Grey,
        ];
        colors.extend((0..=255).map(Color::AnsiValue));
        // Exercise every byte value in every RGB channel, including 9/10/99/100.
        colors.extend((0..=255).map(|n| Color::Rgb {
            r: n,
            g: 255 - n,
            b: n ^ 0x55,
        }));
        let glyphs = ['x', 'é', '界', '🦀', '\u{0301}', '\0', ' '];
        let grid = colors
            .iter()
            .enumerate()
            .map(|(i, &fg)| {
                colors
                    .iter()
                    .enumerate()
                    .map(|(j, &bg)| Cell::with_bg(glyphs[(i + j) % glyphs.len()], fg, bg))
                    .collect()
            })
            .collect();
        compare_reference(&grid);
        // All colors also occur on single-width cells, without skipped slots.
        compare_reference(&vec![
            colors.iter().map(|&c| Cell::with_bg('x', c, c)).collect(),
        ]);
    }

    #[test]
    fn ansi_row_state_and_skipped_cell_colors() {
        let grid = vec![
            vec![
                Cell::with_bg('界', Color::Red, Color::Blue),
                Cell::with_bg('X', Color::Green, Color::Yellow),
                Cell::new('é', Color::Red),
            ],
            vec![],
            vec![
                Cell::with_bg('a', Color::Red, Color::Blue),
                Cell::with_bg('b', Color::Reset, Color::Blue),
            ],
            vec![Cell::new('界', Color::Reset)],
            vec![Cell::new('z', Color::Reset)],
        ];
        let mut bytes = Vec::new();
        encode_grid_ansi(&grid, |chunk| {
            bytes.extend_from_slice(chunk);
            Ok(())
        })
        .unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "\x1b[38;5;9m\x1b[48;5;12m界\x1b[49mé\n\n\x1b[48;5;12ma\x1b[39mb\x1b[49m\n界\nz\n\x1b[0m"
        );
        compare_reference(&grid);
        assert_eq!(grid_to_plain(&grid), ["界é", "", "ab", "界", "z"]);
    }

    #[test]
    fn ansi_chunks_are_bounded_across_cells_rows_and_final_reset() {
        compare_reference(&Vec::new());
        compare_reference(&vec![Vec::new(); ANSI_CHUNK_BYTES]);
        for width in [ANSI_CHUNK_BYTES - 5, ANSI_CHUNK_BYTES - 4, ANSI_CHUNK_BYTES] {
            compare_reference(&vec![vec![Cell::blank(); width]]);
        }
        let (bytes, chunks) = compare_reference(&colored_grid());
        assert!(bytes > ANSI_CHUNK_BYTES);
        assert!(chunks > 1);
        // Maximum RGB escape lengths and a four-byte wide character.
        compare_reference(&vec![vec![
            Cell::with_bg(
                '🦀',
                Color::Rgb {
                    r: 255,
                    g: 255,
                    b: 255
                },
                Color::Rgb {
                    r: 255,
                    g: 255,
                    b: 255
                }
            );
            ANSI_CHUNK_BYTES
        ]]);
    }

    #[test]
    fn ansi_stops_at_failed_write_boundary() {
        let mut calls = 0;
        let result = encode_grid_ansi(&colored_grid(), |_| {
            calls += 1;
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(calls, 1);
    }

    #[cfg(all(unix, not(debug_assertions)))]
    #[test]
    #[ignore = "release-only 320x103 ANSI probe; run with --release --ignored --nocapture"]
    fn perf_terminal_emission_320x103() {
        use std::hint::black_box;
        use std::time::Instant;

        struct Consume;
        impl Write for Consume {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                Ok(black_box(bytes).len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let grid = colored_grid();
        let mut encoded = Vec::new();
        encode_grid_ansi(&grid, |chunk| {
            encoded.extend_from_slice(chunk);
            Ok(())
        })
        .unwrap();
        let iterations = 200;
        let average_us =
            |elapsed: std::time::Duration| elapsed.as_secs_f64() * 1e6 / iterations as f64;
        // Warm both paths, then include each encoder's scratch allocation.
        for _ in 0..10 {
            reference_ansi(black_box(&grid), Consume);
            encode_grid_ansi(black_box(&grid), |chunk| {
                black_box(chunk);
                Ok(())
            })
            .unwrap();
        }
        let started = Instant::now();
        for _ in 0..iterations {
            reference_ansi(black_box(&grid), Consume);
        }
        let reference_us = average_us(started.elapsed());
        let started = Instant::now();
        for _ in 0..iterations {
            encode_grid_ansi(black_box(&grid), |chunk| {
                black_box(chunk);
                Ok(())
            })
            .unwrap();
        }
        let encoding_us = average_us(started.elapsed());

        // A raw PTY drained by a reader thread measures kernel transport and
        // backpressure. It does not measure a terminal emulator's paint time.
        let pty = nix::pty::openpty(None, None).unwrap();
        let mut attrs = nix::sys::termios::tcgetattr(&pty.slave).unwrap();
        nix::sys::termios::cfmakeraw(&mut attrs);
        nix::sys::termios::tcsetattr(&pty.slave, nix::sys::termios::SetArg::TCSANOW, &attrs)
            .unwrap();
        let expected = encoded.len() * iterations * 3;
        let reader = std::thread::spawn(move || {
            use std::io::Read;
            let mut master = std::fs::File::from(pty.master);
            let mut received = 0;
            let mut buffer = [0; ANSI_CHUNK_BYTES];
            while received < expected {
                let n = master.read(&mut buffer).unwrap();
                assert_ne!(n, 0);
                received += n;
            }
            assert_eq!(received, expected);
        });
        let mut out = std::fs::File::from(pty.slave);
        let started = Instant::now();
        for _ in 0..iterations {
            for chunk in encoded.chunks(ANSI_CHUNK_BYTES) {
                out.write_all(chunk).unwrap();
            }
            out.flush().unwrap();
        }
        let write_us = average_us(started.elapsed());
        let started = Instant::now();
        for _ in 0..iterations {
            reference_ansi(&grid, &mut out);
        }
        let reference_total_us = average_us(started.elapsed());
        let started = Instant::now();
        for _ in 0..iterations {
            encode_grid_ansi(&grid, |chunk| out.write_all(chunk)).unwrap();
            out.flush().unwrap();
        }
        let total_us = average_us(started.elapsed());
        reader.join().unwrap();
        eprintln!(
            "320x103 colored grid: {} bytes, {iterations} iterations; reference encoding {reference_us:.1} us, optimized encoding {encoding_us:.1} us, preencoded PTY write {write_us:.1} us; reference encode+PTY {reference_total_us:.1} us, optimized encode+PTY {total_us:.1} us",
            encoded.len()
        );
    }
}
