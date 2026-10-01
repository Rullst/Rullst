//! The approved v13 opening: the ANSI Shadow "RULLST" wordmark in a diagonal
//! blue → green → orange gradient, a ~0.7 s flowing animation on the first run
//! of the day, and the bold slogan line with coloured keywords.

use super::palette::{self, ColorDepth, Rgb, STOPS};
use super::terminal::{KeyWait, RawInput, TerminalProfile};
use std::io::{self, Write};
use std::time::Duration;

const WORDMARK: [&str; 6] = [
    "██████╗ ██╗   ██╗██╗     ██╗     ███████╗████████╗",
    "██╔══██╗██║   ██║██║     ██║     ██╔════╝╚══██╔══╝",
    "██████╔╝██║   ██║██║     ██║     ███████╗   ██║   ",
    "██╔══██╗██║   ██║██║     ██║     ╚════██║   ██║   ",
    "██║  ██║╚██████╔╝███████╗███████╗███████║   ██║   ",
    "╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚══════╝╚══════╝   ╚═╝   ",
];
/// Every opening line starts at this margin.
const MARGIN: &str = "  ";
/// The palette flows across the word in 42 steps of 17 ms (~0.7 s).
const STEPS: u32 = 42;
const FRAME_DELAY: Duration = Duration::from_millis(17);
/// The first frame is drawn before any delay with the palette almost a full
/// cycle away from its final position.
const START_PHASE: f64 = 0.999;
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

/// Plain, deterministic opening for pipes, CI, `TERM=dumb` and `NO_COLOR`.
pub(super) const PLAIN_SLOGAN: &str = concat!(
    "RULLST v",
    env!("CARGO_PKG_VERSION"),
    " · SECURE, FAST AND AI-NATIVE RUST FRAMEWORK"
);

const SLOGAN_GREY: Rgb = (150, 155, 175);
/// `(text, colour, bold)`; only the separator is not bold.
const SLOGAN: [(&str, Rgb, bool); 8] = [
    (
        concat!("v", env!("CARGO_PKG_VERSION")),
        (240, 240, 248),
        true,
    ),
    ("  ·  ", (110, 110, 130), false),
    ("SECURE", STOPS[0], true),
    (", ", SLOGAN_GREY, true),
    ("FAST", STOPS[1], true),
    (" AND ", SLOGAN_GREY, true),
    ("AI-NATIVE", STOPS[2], true),
    (" RUST FRAMEWORK", SLOGAN_GREY, true),
];

fn wordmark_width() -> usize {
    WORDMARK
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
}

/// The colour of the wordmark cell at `row`/`column` for an animation phase;
/// phase `0` is the final diagonal gradient.
fn cell_color(row: usize, column: usize, width: usize, phase: f64) -> Rgb {
    let span = width.saturating_sub(3).max(1) as f64;
    let position = (column as f64 + row as f64 * 0.6) / span;
    palette::flowing(position.min(1.0), phase)
}

fn wordmark_lines(phase: f64, depth: ColorDepth) -> Vec<String> {
    let width = wordmark_width();
    WORDMARK
        .iter()
        .enumerate()
        .map(|(row, line)| {
            let mut rendered = String::from(MARGIN);
            let mut current = None;
            for (column, character) in line.chars().enumerate() {
                if character != ' ' && depth != ColorDepth::None {
                    let color = cell_color(row, column, width, phase);
                    // Equal neighbours share one escape sequence.
                    let sequence = palette::foreground(color, depth);
                    if current.as_ref() != Some(&sequence) {
                        rendered.push_str(&sequence);
                        current = Some(sequence);
                    }
                }
                rendered.push(character);
            }
            if current.is_some() {
                rendered.push_str(RESET);
            }
            rendered
        })
        .collect()
}

fn slogan_line(depth: ColorDepth) -> String {
    if depth == ColorDepth::None {
        return format!("{MARGIN}{PLAIN_SLOGAN}");
    }
    let mut line = format!("{MARGIN} ");
    for (text, color, bold) in SLOGAN {
        if bold {
            line.push_str(BOLD);
        }
        line.push_str(&palette::foreground(color, depth));
        line.push_str(text);
        line.push_str(RESET);
    }
    line
}

/// The full opening for one phase: the wordmark, a blank line and the slogan.
fn frame_lines(phase: f64, depth: ColorDepth) -> Vec<String> {
    let mut lines = wordmark_lines(phase, depth);
    lines.push(String::new());
    lines.push(slogan_line(depth));
    lines
}

/// Animation phases after the first frame; the last one is exactly `0`.
fn animation_phases() -> impl Iterator<Item = f64> {
    (1..=STEPS).map(|step| {
        if step == STEPS {
            0.0
        } else {
            1.0 - f64::from(step) / f64::from(STEPS)
        }
    })
}

fn redraw(out: &mut impl Write, phase: f64, depth: ColorDepth) -> io::Result<()> {
    let lines = frame_lines(phase, depth);
    write!(out, "\x1b[{}A", lines.len())?;
    for line in lines {
        // Raw mode disables newline translation: return the carriage too.
        write!(out, "\r\x1b[2K{line}\r\n")?;
    }
    out.flush()
}

/// Plays the flowing palette; `wait` sleeps one frame and reports keys. Any
/// key jumps to the final frame, which is always the last thing drawn.
fn play(
    out: &mut impl Write,
    depth: ColorDepth,
    mut wait: impl FnMut(Duration) -> KeyWait,
) -> io::Result<KeyWait> {
    write!(out, "\r\n")?;
    for line in frame_lines(START_PHASE, depth) {
        write!(out, "\r\x1b[2K{line}\r\n")?;
    }
    out.flush()?;
    for phase in animation_phases() {
        match wait(FRAME_DELAY) {
            KeyWait::Elapsed => redraw(out, phase, depth)?,
            interrupted => {
                redraw(out, 0.0, depth)?;
                write!(out, "\r\n")?;
                out.flush()?;
                return Ok(interrupted);
            }
        }
    }
    write!(out, "\r\n")?;
    out.flush()?;
    Ok(KeyWait::Elapsed)
}

fn print_static(out: &mut impl Write, depth: ColorDepth) -> io::Result<()> {
    if depth == ColorDepth::None {
        writeln!(out, "{PLAIN_SLOGAN}")?;
        writeln!(out)?;
        return out.flush();
    }
    writeln!(out)?;
    for line in frame_lines(0.0, depth) {
        writeln!(out, "{line}")?;
    }
    writeln!(out)?;
    out.flush()
}

/// Prints the opening for `profile`. It animates only on the first run of the
/// day in a motion-capable terminal; Ctrl+C during the animation returns an
/// `Interrupted` error after the terminal has been restored.
pub(super) fn print_opening(profile: &TerminalProfile, out: &mut impl Write) -> io::Result<()> {
    let animate = profile.motion
        && super::opening_marker::marker_path().is_some_and(|path| {
            super::opening_marker::claim_daily_animation(&path, &super::opening_marker::today())
        });
    if !animate {
        return print_static(out, profile.color);
    }
    let Some(input) = RawInput::enable() else {
        return print_static(out, profile.color);
    };
    let outcome = play(out, profile.color, |delay| input.wait(delay));
    input.drain();
    drop(input);
    match outcome? {
        KeyWait::Interrupt => Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "read interrupted",
        )),
        KeyWait::Elapsed | KeyWait::Skip => Ok(()),
    }
}

#[cfg(test)]
#[path = "dashboard_brand_tests.rs"]
mod tests;
