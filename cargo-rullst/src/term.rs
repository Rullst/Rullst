//! Terminal policy shared by `cargo rullst add` previews and the
//! `cargo rullst green` report: colour and how untrusted text is made safe to
//! print.
//!
//! The rules match the CLI home screen (`NO_COLOR`, `CI`, `TERM=dumb` and
//! non-TTY streams select plain output). File contents and command output are
//! untrusted, so every byte that reaches the terminal goes through
//! [`sanitize`] first: an escape sequence could otherwise rewrite the screen,
//! set the window title or write to the clipboard.

use std::io::IsTerminal;

/// The environment inputs that decide colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TermEnv {
    pub stdout_tty: bool,
    pub no_color: bool,
    pub ci: bool,
    pub dumb: bool,
}

impl TermEnv {
    pub fn detect() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Self {
            stdout_tty: std::io::stdout().is_terminal(),
            no_color: std::env::var_os("NO_COLOR").is_some(),
            ci: var("CI").is_some_and(|value| {
                !matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "" | "0" | "false" | "no" | "off"
                )
            }),
            dumb: var("TERM").is_some_and(|term| term.trim() == "dumb"),
        }
    }

    /// Colour only for a terminal that has not opted out.
    pub fn color(&self) -> bool {
        self.stdout_tty && !self.no_color && !self.ci && !self.dumb
    }
}

/// ANSI styling that collapses to plain text when colour is disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub color: bool,
}

impl Style {
    fn paint(self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub fn bold(self, text: &str) -> String {
        self.paint("1", text)
    }

    pub fn dim(self, text: &str) -> String {
        self.paint("2", text)
    }

    pub fn green(self, text: &str) -> String {
        self.paint("32", text)
    }

    pub fn red(self, text: &str) -> String {
        self.paint("31", text)
    }

    pub fn yellow(self, text: &str) -> String {
        self.paint("33", text)
    }

    pub fn cyan(self, text: &str) -> String {
        self.paint("36", text)
    }
}

/// Characters that must never reach the terminal from untrusted text: C0
/// controls other than tab and line feed (including ESC and CR), DEL, C1
/// controls and bidirectional overrides that can disguise source code.
fn unsafe_char(character: char) -> bool {
    matches!(character,
        '\u{0}'..='\u{8}' | '\u{b}'..='\u{1f}' | '\u{7f}'..='\u{9f}'
        | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Replaces unsafe characters with a visible `\u{..}` escape so a reviewer
/// sees them instead of the terminal interpreting them.
pub fn sanitize(text: &str) -> String {
    if !text.chars().any(unsafe_char) {
        return text.to_string();
    }
    let mut output = String::with_capacity(text.len() + 8);
    for character in text.chars() {
        if unsafe_char(character) {
            output.push_str(&format!("\\u{{{:x}}}", u32::from(character)));
        } else {
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tty() -> TermEnv {
        TermEnv {
            stdout_tty: true,
            ..TermEnv::default()
        }
    }

    #[test]
    fn automation_and_opt_outs_disable_colour() {
        assert!(tty().color());
        for env in [
            TermEnv {
                no_color: true,
                ..tty()
            },
            TermEnv { ci: true, ..tty() },
            TermEnv {
                dumb: true,
                ..tty()
            },
            TermEnv {
                stdout_tty: false,
                ..tty()
            },
        ] {
            assert!(!env.color(), "{env:?}");
        }
    }

    #[test]
    fn untrusted_text_cannot_emit_terminal_controls() {
        let hostile = "ok\x1b]52;c;ZXZpbA==\x07\r\u{202e}txt\u{9b}2J\tend\n";
        let safe = sanitize(hostile);
        assert!(!safe.contains('\x1b') && !safe.contains('\r') && !safe.contains('\u{202e}'));
        assert!(safe.contains("\\u{1b}") && safe.contains("\\u{202e}") && safe.contains("\\u{9b}"));
        assert!(safe.contains("\tend\n"));
        assert_eq!(sanitize("plain ✓ text\n"), "plain ✓ text\n");
    }

    #[test]
    fn plain_style_adds_no_escape_codes() {
        let plain = Style { color: false };
        assert_eq!(plain.red("x"), "x");
        assert_eq!(Style { color: true }.green("x"), "\x1b[32mx\x1b[0m");
    }
}
