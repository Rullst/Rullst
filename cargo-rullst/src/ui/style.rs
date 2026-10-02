//! Plain-or-coloured text for CLI output outside the home screen (doctor,
//! friendly errors, suggestions, next steps). Colour follows the same rules
//! as the home: none for `NO_COLOR`, `CI`, `TERM=dumb` or a stream that is
//! not a terminal; 24-bit with `COLORTERM=truecolor|24bit`, else xterm-256.

pub(crate) use super::palette::Rgb;
use super::palette::{self, ColorDepth, STOPS};
use super::terminal::TerminalEnv;

/// Brand blue: links and accents.
pub(crate) const ACCENT: Rgb = STOPS[0];
/// Brand green: success.
pub(crate) const PASS: Rgb = STOPS[1];
/// Brand orange: warnings.
pub(crate) const WARN: Rgb = STOPS[2];
/// Failures.
pub(crate) const FAIL: Rgb = (240, 72, 72);
/// Labels and secondary text.
pub(crate) const MUTED: Rgb = (150, 155, 175);
/// Primary text.
pub(crate) const BRIGHT: Rgb = (240, 240, 248);

/// How text is painted on one output stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Style {
    depth: ColorDepth,
}

impl Style {
    /// Deterministic text without escape sequences.
    pub(crate) const PLAIN: Self = Self {
        depth: ColorDepth::None,
    };

    pub(crate) const fn with_depth(depth: ColorDepth) -> Self {
        Self { depth }
    }

    /// The colour depth for a stream, given whether it is a terminal.
    pub(crate) fn depth_for(env: &TerminalEnv, tty: bool) -> ColorDepth {
        if !tty || env.no_color || env.ci || env.dumb {
            ColorDepth::None
        } else if env.truecolor {
            ColorDepth::TrueColor
        } else {
            ColorDepth::Ansi256
        }
    }

    /// The style for standard output.
    pub(crate) fn stdout() -> Self {
        let env = TerminalEnv::detect();
        Self::with_depth(Self::depth_for(&env, env.stdout_tty))
    }

    /// The style for standard error.
    pub(crate) fn stderr() -> Self {
        let env = TerminalEnv::detect();
        Self::with_depth(Self::depth_for(&env, env.stderr_tty))
    }

    pub(crate) fn is_plain(self) -> bool {
        self.depth == ColorDepth::None
    }

    /// `text` in `color`; unchanged when plain.
    pub(crate) fn paint(self, text: &str, color: Rgb) -> String {
        self.styled(text, color, false)
    }

    /// `text` in bold `color`; unchanged when plain.
    pub(crate) fn bold(self, text: &str, color: Rgb) -> String {
        self.styled(text, color, true)
    }

    fn styled(self, text: &str, color: Rgb, bold: bool) -> String {
        if self.is_plain() || text.is_empty() {
            return text.to_string();
        }
        let weight = if bold { "\x1b[1m" } else { "" };
        format!(
            "{weight}{}{text}\x1b[0m",
            palette::foreground(color, self.depth)
        )
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
    fn colour_follows_the_stream_and_the_opt_outs() {
        assert_eq!(Style::depth_for(&tty(), true), ColorDepth::Ansi256);
        let truecolor = TerminalEnv {
            truecolor: true,
            ..tty()
        };
        assert_eq!(Style::depth_for(&truecolor, true), ColorDepth::TrueColor);
        assert_eq!(Style::depth_for(&tty(), false), ColorDepth::None);
        for env in [
            TerminalEnv {
                no_color: true,
                ..tty()
            },
            TerminalEnv { ci: true, ..tty() },
            TerminalEnv {
                dumb: true,
                ..tty()
            },
        ] {
            assert_eq!(Style::depth_for(&env, true), ColorDepth::None);
        }
    }

    #[test]
    fn plain_text_has_no_escape_sequences() {
        assert_eq!(Style::PLAIN.bold("ok", PASS), "ok");
        let coloured = Style::with_depth(ColorDepth::TrueColor).bold("ok", PASS);
        assert_eq!(coloured, "\x1b[1m\x1b[38;2;30;205;110mok\x1b[0m");
        assert_eq!(Style::with_depth(ColorDepth::Ansi256).paint("", FAIL), "");
    }
}
