//! Keeps secrets and terminal control sequences out of rendered errors. Error
//! messages can quote connection strings, provider responses or file names,
//! so every displayed line passes through [`sanitize`].

use regex::Regex;
use std::sync::OnceLock;

/// Shown instead of a message when the redaction rules are unavailable.
const UNREDACTABLE: &str = "[details hidden: the secret filter is unavailable]";
/// Longest message rendered; longer ones are cut on a character boundary.
const MESSAGE_LIMIT: usize = 4096;

fn rules() -> Option<&'static [(Regex, &'static str)]> {
    static RULES: OnceLock<Option<Vec<(Regex, &'static str)>>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            [
                // scheme://user:password@host keeps the user, hides the password.
                (
                    r"(?i)\b([a-z][a-z0-9+.\-]*://)([^\s:/@]*):([^\s@/]+)@",
                    "${1}${2}:***@",
                ),
                // scheme://<long token>@host
                (r"(?i)\b([a-z][a-z0-9+.\-]*://)([^\s:/@]{16,})@", "${1}***@"),
                // NAME=value / name: value for secret-like names.
                (
                    r#"(?i)\b([a-z0-9_.\-]*(?:password|passwd|secret|token|api[_\-]?key|access[_\-]?key|private[_\-]?key|app[_\-]?key|client[_\-]?secret|credential)s?[a-z0-9_]*)(\s*(?:=|:\s)\s*)("[^"]*"|'[^']*'|[^\s&;,]+)"#,
                    "${1}${2}***",
                ),
                (r"(?i)\b(bearer|basic)\s+[a-z0-9._~+/=\-]{8,}", "${1} ***"),
                // Well-known credential formats.
                (
                    r"\b(?:sk-[A-Za-z0-9_\-]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|xox[abpr]-[A-Za-z0-9\-]{10,}|AKIA[0-9A-Z]{16}|AIza[0-9A-Za-z_\-]{30,})",
                    "***",
                ),
            ]
            .into_iter()
            .map(|(pattern, replacement)| {
                Regex::new(pattern)
                    .ok()
                    .map(|regex| (regex, replacement))
            })
            .collect()
        })
        .as_deref()
}

/// Replaces control characters other than newline and tab, so a message
/// cannot move the cursor, retitle the terminal or clear the screen.
fn without_controls(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() && character != '\n' && character != '\t' {
                '?'
            } else {
                character
            }
        })
        .collect()
}

/// `text` with secrets masked, control characters replaced and its length bounded.
pub(crate) fn sanitize(text: &str) -> String {
    let Some(rules) = rules() else {
        return UNREDACTABLE.to_string();
    };
    let mut bounded: String = text.chars().take(MESSAGE_LIMIT).collect();
    if text.chars().nth(MESSAGE_LIMIT).is_some() {
        bounded.push('…');
    }
    let mut redacted = bounded;
    for (regex, replacement) in rules {
        redacted = regex.replace_all(&redacted, *replacement).into_owned();
    }
    without_controls(&redacted)
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn connection_strings_keep_the_host_but_not_the_password() {
        let text = sanitize(
            "error connecting to postgres://app:hunter2@db.internal:5432/shop and redis://:s3cr3t@cache:6379",
        );
        assert_eq!(
            text,
            "error connecting to postgres://app:***@db.internal:5432/shop and redis://:***@cache:6379"
        );
        assert_eq!(
            sanitize("clone https://ghp_abcdefghijklmnopqrstuvwxyz@github.com/x"),
            "clone https://***@github.com/x"
        );
        assert_eq!(
            sanitize("open sqlite://db.sqlite?mode=rwc"),
            "open sqlite://db.sqlite?mode=rwc"
        );
    }

    #[test]
    fn secret_assignments_headers_and_known_tokens_are_masked() {
        for (input, expected) in [
            ("APP_KEY=abcdefghijklmnop", "APP_KEY=***"),
            ("OPENAI_API_KEY=\"sk-live\" next", "OPENAI_API_KEY=*** next"),
            ("url?password=hunter2&user=me", "url?password=***&user=me"),
            ("client_secret: zyx987 rest", "client_secret: *** rest"),
            (
                "Authorization: Bearer abc.def.ghi-123",
                "Authorization: Bearer ***",
            ),
            ("key sk-ABCDEFGHIJKLMNOPQRSTUV rejected", "key *** rejected"),
            ("AKIAABCDEFGHIJKLMNOP leaked", "*** leaked"),
        ] {
            assert_eq!(sanitize(input), expected, "{input}");
        }
        assert_eq!(
            sanitize("unknown pkg action 'x'; use 'add' or 'list'"),
            "unknown pkg action 'x'; use 'add' or 'list'"
        );
    }

    #[test]
    fn control_sequences_are_neutralized_and_length_is_bounded() {
        assert_eq!(
            sanitize("bad \x1b]0;title\x07name\nnext\tline"),
            "bad ?]0;title?name\nnext\tline"
        );
        let long = "x".repeat(5000);
        let text = sanitize(&long);
        assert_eq!(text.chars().count(), 4097);
        assert!(text.ends_with('…'));
    }
}
