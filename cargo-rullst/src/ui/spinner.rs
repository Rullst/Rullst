// src/ui/spinner.rs — Neon animated terminal spinner.

use super::terminal::TerminalEnv;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// How progress is shown on standard output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpinnerMode {
    /// Coloured frames redrawn in place with carriage returns.
    Animated,
    /// One plain line, without escape sequences or carriage returns.
    Plain,
}

/// Animation needs standard output to be a terminal without `NO_COLOR`,
/// `CI`, `TERM=dumb` or `RULLST_REDUCED_MOTION`; CI logs and pipes get a
/// plain line.
fn spinner_mode(env: &TerminalEnv) -> SpinnerMode {
    if env.stdout_tty && !env.no_color && !env.ci && !env.dumb && !env.reduced_motion {
        SpinnerMode::Animated
    } else {
        SpinnerMode::Plain
    }
}

/// Writes `msg` as one line, then runs `f`.
fn plain_progress<W, F, T>(out: &mut W, msg: &str, f: F) -> T
where
    W: Write,
    F: FnOnce() -> T,
{
    let _ = writeln!(out, "{msg}");
    let _ = out.flush();
    f()
}

/// Runs `f` while showing `msg`: an animated spinner in a terminal, one plain
/// line in CI logs, pipes or with `NO_COLOR`.
pub fn with_spinner<F, T>(msg: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    match spinner_mode(&TerminalEnv::detect()) {
        SpinnerMode::Plain => return plain_progress(&mut std::io::stdout(), msg, f),
        SpinnerMode::Animated => {}
    }
    let msg = msg.to_string();
    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_clone = is_running.clone();

    let handle = thread::spawn(move || {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut i = 0;
        let colors = [
            colored::Color::Cyan,
            colored::Color::Magenta,
            colored::Color::BrightCyan,
            colored::Color::Blue,
        ];

        while is_running_clone.load(Ordering::SeqCst) {
            use colored::Colorize;
            let frame = frames[i % frames.len()];
            let color = colors[(i / 2) % colors.len()];

            let mut animated_msg = String::new();

            let found_target = ["Application", "migrations"]
                .into_iter()
                .filter_map(|target| msg.find(target).map(|position| (target, position)))
                .min_by_key(|(_, position)| *position);

            if let Some((target, pos)) = found_target {
                animated_msg.push_str(&msg[..pos].bold().to_string());

                let custom_colors = colors;

                for (j, ch) in target.chars().enumerate() {
                    let is_upper = ((i + j) % 4) < 2;
                    let wave_char = if is_upper {
                        ch.to_ascii_uppercase()
                    } else {
                        ch.to_ascii_lowercase()
                    };
                    let c_idx = (i + j) % custom_colors.len();
                    animated_msg.push_str(
                        &wave_char
                            .to_string()
                            .color(custom_colors[c_idx])
                            .bold()
                            .to_string(),
                    );
                }
                animated_msg.push_str(&msg[pos + target.len()..].bold().to_string());
            } else {
                animated_msg = msg.bold().to_string();
            }

            print!("\r\x1B[K{} {}", frame.color(color).bold(), animated_msg);
            let _ = std::io::stdout().flush();

            thread::sleep(Duration::from_millis(80));
            i += 1;
        }
        print!("\r\x1B[K");
        let _ = std::io::stdout().flush();
    });

    let result = f();

    is_running.store(false, Ordering::SeqCst);
    let _ = handle.join();

    result
}

#[cfg(test)]
#[path = "spinner_tests.rs"]
mod tests;
