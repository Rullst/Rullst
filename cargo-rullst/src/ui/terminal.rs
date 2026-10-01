//! Terminal capabilities for the interactive home: colour depth, motion and
//! whether prompts may run. The decision is a pure function of an environment
//! snapshot so every rule (`NO_COLOR`, `RULLST_REDUCED_MOTION`, `CI`,
//! `TERM=dumb`, non-TTY streams, `COLORTERM`) is unit tested.

use super::palette::ColorDepth;
use std::io::{IsTerminal, Write};
use std::time::Duration;

/// The inputs that decide how the home screen renders.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TerminalEnv {
    pub stdin_tty: bool,
    pub stdout_tty: bool,
    pub stderr_tty: bool,
    /// `NO_COLOR` is present (any value, as the prompt library also treats it).
    pub no_color: bool,
    /// `CI` holds a value other than empty, `0`, `false`, `no` or `off`.
    pub ci: bool,
    /// `TERM=dumb`.
    pub dumb: bool,
    /// `RULLST_REDUCED_MOTION` is `1`, `true` or `yes`.
    pub reduced_motion: bool,
    /// `COLORTERM` is `truecolor` or `24bit`.
    pub truecolor: bool,
}

impl TerminalEnv {
    pub(super) fn detect() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Self {
            stdin_tty: std::io::stdin().is_terminal(),
            stdout_tty: std::io::stdout().is_terminal(),
            stderr_tty: std::io::stderr().is_terminal(),
            no_color: std::env::var_os("NO_COLOR").is_some(),
            ci: ci_flag(var("CI").as_deref()),
            dumb: var("TERM").is_some_and(|term| term.trim() == "dumb"),
            reduced_motion: super::update_check::enabled_env_flag(
                std::env::var_os("RULLST_REDUCED_MOTION").as_deref(),
            ),
            truecolor: var("COLORTERM").is_some_and(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "truecolor" | "24bit"
                )
            }),
        }
    }
}

fn ci_flag(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "no" | "off"
        )
    })
}

/// How the home screen may behave in the attached terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TerminalProfile {
    pub color: ColorDepth,
    /// The opening may animate (it still plays only on the day's first run).
    pub motion: bool,
    /// Menus may prompt: every standard stream is a terminal outside CI.
    pub interactive: bool,
}

impl TerminalProfile {
    pub(super) fn from_env(env: &TerminalEnv) -> Self {
        let automation = env.ci || env.dumb;
        let color = if env.stdout_tty && !env.no_color && !automation {
            if env.truecolor {
                ColorDepth::TrueColor
            } else {
                ColorDepth::Ansi256
            }
        } else {
            ColorDepth::None
        };
        let interactive = env.stdin_tty && env.stdout_tty && env.stderr_tty && !automation;
        Self {
            color,
            motion: interactive && color != ColorDepth::None && !env.reduced_motion,
            interactive,
        }
    }

    pub(super) fn detect() -> Self {
        Self::from_env(&TerminalEnv::detect())
    }
}

/// What a key press during an animation asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum KeyWait {
    /// The frame delay elapsed without a key.
    Elapsed,
    /// A key was pressed: jump to the final frame.
    Skip,
    /// Ctrl+C: stop like an interrupted prompt.
    Interrupt,
}

/// Raw keyboard input for the duration of an animation. Dropping it restores
/// cooked mode and the cursor on every path, including early returns.
pub(super) struct RawInput {
    _private: (),
}

impl RawInput {
    /// `None` when the terminal refuses raw mode; callers then skip motion.
    pub(super) fn enable() -> Option<Self> {
        crossterm::terminal::enable_raw_mode().ok()?;
        let input = Self { _private: () };
        let mut stdout = std::io::stdout();
        let _ = write!(stdout, "\x1b[?25l");
        let _ = stdout.flush();
        Some(input)
    }

    /// Waits up to `delay` for a key without blocking past it.
    pub(super) fn wait(&self, delay: Duration) -> KeyWait {
        use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
        match crossterm::event::poll(delay) {
            Ok(false) => KeyWait::Elapsed,
            Ok(true) => match crossterm::event::read() {
                Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        KeyWait::Interrupt
                    } else {
                        KeyWait::Skip
                    }
                }
                Ok(_) => KeyWait::Elapsed,
                Err(_) => KeyWait::Skip,
            },
            Err(_) => KeyWait::Skip,
        }
    }

    /// Discards keys typed during the animation so they never answer the
    /// next prompt. Bounded so a flood of input cannot stall the home screen.
    pub(super) fn drain(&self) {
        for _ in 0..256 {
            match crossterm::event::poll(Duration::ZERO) {
                Ok(true) => {
                    if crossterm::event::read().is_err() {
                        return;
                    }
                }
                _ => return,
            }
        }
    }
}

impl Drop for RawInput {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = write!(stdout, "\x1b[?25h");
        let _ = stdout.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tty() -> TerminalEnv {
        TerminalEnv {
            stdin_tty: true,
            stdout_tty: true,
            stderr_tty: true,
            ..TerminalEnv::default()
        }
    }

    #[test]
    fn an_interactive_terminal_gets_colour_motion_and_prompts() {
        let profile = TerminalProfile::from_env(&tty());
        assert_eq!(profile.color, ColorDepth::Ansi256);
        assert!(profile.motion);
        assert!(profile.interactive);

        let truecolor = TerminalProfile::from_env(&TerminalEnv {
            truecolor: true,
            ..tty()
        });
        assert_eq!(truecolor.color, ColorDepth::TrueColor);
    }

    #[test]
    fn automation_and_opt_outs_select_plain_static_output() {
        let plain = |env: TerminalEnv| TerminalProfile::from_env(&env);

        let no_color = plain(TerminalEnv {
            no_color: true,
            truecolor: true,
            ..tty()
        });
        assert_eq!(no_color.color, ColorDepth::None);
        assert!(!no_color.motion);
        assert!(no_color.interactive, "NO_COLOR only removes colour");

        let reduced = plain(TerminalEnv {
            reduced_motion: true,
            ..tty()
        });
        assert_eq!(reduced.color, ColorDepth::Ansi256);
        assert!(!reduced.motion);
        assert!(reduced.interactive);

        for env in [
            TerminalEnv { ci: true, ..tty() },
            TerminalEnv {
                dumb: true,
                ..tty()
            },
        ] {
            let profile = plain(env);
            assert_eq!(profile.color, ColorDepth::None);
            assert!(!profile.motion);
            assert!(!profile.interactive);
        }

        for env in [
            TerminalEnv {
                stdout_tty: false,
                ..tty()
            },
            TerminalEnv {
                stdin_tty: false,
                ..tty()
            },
            TerminalEnv {
                stderr_tty: false,
                ..tty()
            },
        ] {
            let profile = plain(env);
            assert!(!profile.motion);
            assert!(!profile.interactive);
        }
        assert_eq!(
            plain(TerminalEnv {
                stdout_tty: false,
                ..tty()
            })
            .color,
            ColorDepth::None
        );
    }

    #[test]
    fn ci_values_follow_common_conventions() {
        for value in ["true", "1", "TRUE", "yes", "github"] {
            assert!(ci_flag(Some(value)), "{value}");
        }
        for value in ["", " ", "0", "false", "No", "off"] {
            assert!(!ci_flag(Some(value)), "{value}");
        }
        assert!(!ci_flag(None));
    }
}
