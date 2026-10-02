//! The terminal side of [`super::Screen`]: colour painting, raw-mode key
//! reading and in-place redraws. Raw mode is restored on every path.

use super::super::palette::{self, ColorDepth, Rgb, STOPS};
use super::super::terminal::{RawInput, TerminalProfile};
use super::{Answer, Cursor, Key, LABEL, Line, MUTED, Outcome, Screen, Tone, VALUE};
use super::{echo_line, frame, handle_key};
use std::io::{self, Write};

fn sgr(rgb: Rgb, bold: bool, depth: ColorDepth) -> String {
    let weight = if bold { "\x1b[1m" } else { "" };
    format!("{weight}{}", palette::foreground(rgb, depth))
}

/// `line` with colour sequences for `depth` (none at [`ColorDepth::None`]).
pub(super) fn paint(line: &Line, depth: ColorDepth) -> String {
    if depth == ColorDepth::None {
        return line.text();
    }
    let mut out = String::new();
    for span in &line.spans {
        let style = match span.tone {
            Tone::Plain => None,
            Tone::Brand => {
                let count = span.text.chars().count().max(2) - 1;
                for (index, character) in span.text.chars().enumerate() {
                    let rgb = palette::gradient(index as f64 / count as f64);
                    out.push_str(&sgr(rgb, true, depth));
                    out.push(character);
                }
                out.push_str("\x1b[0m");
                continue;
            }
            Tone::Label => Some((LABEL, false)),
            Tone::Value => Some((VALUE, false)),
            Tone::Strong => Some((VALUE, true)),
            Tone::Accent => Some((STOPS[1], true)),
            Tone::Muted => Some((MUTED, false)),
            Tone::Directory => Some((STOPS[0], false)),
            Tone::Warning => Some((STOPS[2], false)),
        };
        match style {
            Some((rgb, bold)) => {
                out.push_str(&sgr(rgb, bold, depth));
                out.push_str(&span.text);
                out.push_str("\x1b[0m");
            }
            None => out.push_str(&span.text),
        }
    }
    out
}

/// The attached terminal: whether it may prompt and how it shows colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Terminal {
    profile: TerminalProfile,
}

impl Terminal {
    pub(crate) fn detect() -> Self {
        Self {
            profile: TerminalProfile::detect(),
        }
    }

    /// A terminal that never prompts and shows no colour (pipes, CI).
    #[cfg(test)]
    pub(crate) fn non_interactive() -> Self {
        Self {
            profile: TerminalProfile::from_env(&super::super::terminal::TerminalEnv::default()),
        }
    }

    /// A terminal that may prompt, without colour, whatever the test runner's
    /// streams are.
    #[cfg(test)]
    pub(crate) fn interactive_for_tests() -> Self {
        Self {
            profile: TerminalProfile::from_env(&super::super::terminal::TerminalEnv {
                stdin_tty: true,
                stdout_tty: true,
                stderr_tty: true,
                no_color: true,
                ..super::super::terminal::TerminalEnv::default()
            }),
        }
    }

    /// Every standard stream is a terminal outside CI and `TERM=dumb`.
    pub(crate) fn interactive(&self) -> bool {
        self.profile.interactive
    }

    /// Writes `lines` to standard output, coloured when the terminal allows.
    pub(crate) fn print(&self, lines: &[Line]) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        for line in lines {
            writeln!(stdout, "{}", paint(line, self.profile.color))?;
        }
        stdout.flush()
    }

    /// Shows `screen` until it is answered. Ctrl+C is an `Interrupted` error.
    pub(crate) fn choose(&self, screen: &Screen) -> io::Result<Answer> {
        let depth = self.profile.color;
        let raw = RawInput::enable()
            .ok_or_else(|| io::Error::other("the terminal refused raw keyboard input"))?;
        let mut stdout = io::stdout();
        let mut cursor = Cursor::new(screen);
        let mut drawn = 0;
        loop {
            let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
            let lines = frame(screen, &cursor, width, height);
            clear(&mut stdout, drawn)?;
            let painted: Vec<String> = lines.iter().map(|line| paint(line, depth)).collect();
            write!(stdout, "{}", painted.join("\r\n"))?;
            stdout.flush()?;
            drawn = lines.len();
            match handle_key(screen, &mut cursor, read_key()) {
                Outcome::Continue => {}
                Outcome::Interrupted => {
                    clear(&mut stdout, drawn)?;
                    drop(raw);
                    return Err(io::Error::new(
                        io::ErrorKind::Interrupted,
                        "read interrupted",
                    ));
                }
                Outcome::Done(answer) => {
                    clear(&mut stdout, drawn)?;
                    if let Some(line) = echo_line(screen, &answer) {
                        write!(stdout, "{}\r\n", paint(&line, depth))?;
                    }
                    stdout.flush()?;
                    drop(raw);
                    return Ok(answer);
                }
            }
        }
    }
}

/// Moves to the first line of the previous frame and erases it.
pub(super) fn clear(out: &mut impl Write, drawn: usize) -> io::Result<()> {
    if drawn == 0 {
        return Ok(());
    }
    write!(out, "\r")?;
    if drawn > 1 {
        write!(out, "\x1b[{}A", drawn - 1)?;
    }
    write!(out, "\x1b[J")
}

fn read_key() -> Key {
    use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
    match crossterm::event::read() {
        Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Key::Interrupt,
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => Key::Up,
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => Key::Down,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            KeyCode::Enter => Key::Enter,
            KeyCode::Char(' ') => Key::Space,
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Left => Key::Back,
            KeyCode::Char(character) => character
                .to_digit(10)
                .and_then(|digit| u8::try_from(digit).ok())
                .map_or(Key::Other, Key::Digit),
            _ => Key::Other,
        },
        Ok(Event::Resize(..)) => Key::Redraw,
        Ok(_) => Key::Other,
        // A broken input stream must not spin; treat it like Ctrl+C.
        Err(_) => Key::Interrupt,
    }
}
