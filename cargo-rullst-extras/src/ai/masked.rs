//! Values the mandatory `rullst-ai` PII guardrail masks on the way out.
//!
//! Every request masks e-mail usernames, card-like digit runs and CPF/CNPJ
//! numbers in every message, the user's goal and shared files included
//! (`help@acme.com` reaches the model as `h***@acme.com`). The masking stays:
//! it is the guardrail the CLI promises. Instead, the session remembers the
//! masked forms the model saw, tells the user when their own message is
//! masked, and flags a proposed file change that writes a masked form back,
//! because the model never saw the real value.

use rullst_ai::{AiGuardrails, Message};

/// Most masked forms remembered for one request.
const MAX_TOKENS: usize = 32;
/// Longest masked form remembered.
const MAX_TOKEN_CHARS: usize = 128;

/// Characters that belong to a masked value: e-mail addresses, digit runs
/// and formatted tax IDs.
fn value_character(character: char) -> bool {
    character.is_alphanumeric()
        || matches!(character, '*' | '.' | '_' | '%' | '+' | '-' | '@' | '/')
}

/// The masked forms the guardrail produces for `text` (none when nothing
/// is masked): each `*` run of the redacted text, widened to the value
/// around it, that does not occur in `text` itself.
pub(super) fn tokens(text: &str) -> Vec<String> {
    let report = AiGuardrails::inspect(text);
    if !report.pii_was_masked() {
        return Vec::new();
    }
    let redacted: Vec<char> = report.redacted_text().chars().collect();
    let mut found: Vec<String> = Vec::new();
    let mut index = 0;
    while index < redacted.len() && found.len() < MAX_TOKENS {
        if redacted[index] != '*' {
            index += 1;
            continue;
        }
        let mut start = index;
        while start > 0 && value_character(redacted[start - 1]) {
            start -= 1;
        }
        let mut end = index;
        while end < redacted.len() && value_character(redacted[end]) {
            end += 1;
        }
        let token: String = redacted[start..end].iter().collect();
        let token = token.trim_end_matches(['.', '-', '/']).to_string();
        if token.chars().count() <= MAX_TOKEN_CHARS
            && !text.contains(&token)
            && !found.contains(&token)
        {
            found.push(token);
        }
        index = end.max(index + 1);
    }
    found
}

/// The masked forms of every message in one request.
pub(super) fn in_messages(messages: &[Message]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for token in messages.iter().flat_map(|message| tokens(&message.content)) {
        if found.len() == MAX_TOKENS {
            break;
        }
        if !found.contains(&token) {
            found.push(token);
        }
    }
    found
}

/// A masked form that `new` writes and `old` does not already contain.
pub(super) fn written_back<'a>(
    masked: &'a [String],
    old: Option<&str>,
    new: &str,
) -> Option<&'a str> {
    masked
        .iter()
        .map(String::as_str)
        .find(|token| new.contains(token) && !old.is_some_and(|old| old.contains(token)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_values_are_found_and_ordinary_stars_are_not() {
        let text = "let support = \"help@acme.com\";\nlet glob = \"*.rs\"; // a * b\n";
        assert_eq!(tokens(text), ["h***@acme.com"]);
        assert!(tokens("let glob = \"*.rs\"; /* note */").is_empty());
        let card = tokens("charge 4111111111111111 now");
        assert_eq!(card.len(), 1);
        assert!(
            card[0].starts_with('*') && card[0].ends_with("1111"),
            "masked card"
        );
    }

    #[test]
    fn a_change_that_writes_a_masked_value_back_is_detected() {
        let masked = in_messages(&[
            Message::user("set the sender to help@acme.com"),
            Message::assistant("ok"),
        ]);
        let old = "const SENDER: &str = \"old@acme.com\";\n";
        let new = "const SENDER: &str = \"h***@acme.com\";\n";
        assert_eq!(written_back(&masked, Some(old), new), Some("h***@acme.com"));
        assert_eq!(written_back(&masked, Some(new), new), None, "already there");
        assert_eq!(written_back(&masked, None, "no values\n"), None);
    }
}
