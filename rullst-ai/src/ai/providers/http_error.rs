//! Text of [`AiError::ApiError`] for an unsuccessful provider HTTP response.
//!
//! The text always starts with `"{provider} returned HTTP {status}"`. When the
//! response body is JSON with a provider error message (`error.message`, an
//! `error` string, `message` or `detail`), that message follows as one line
//! after a colon, preceded by the provider's error code (`error.status`,
//! `error.code` or `error.type`) when it has one. The body is read up to
//! [`MAX_ERROR_BODY_BYTES`]; the detail is cut to [`MAX_ERROR_DETAIL_CHARS`]
//! characters, and URL queries, URL credentials, `key=value` secrets and
//! key-shaped tokens are redacted. Other bodies (HTML, plain text, oversized
//! or unreadable) add nothing.

use crate::ai::AiError;

/// Largest error body read to look for a provider message.
const MAX_ERROR_BODY_BYTES: usize = 16 * 1_024;

/// Longest provider detail kept in the error text, in characters.
const MAX_ERROR_DETAIL_CHARS: usize = 300;

/// Shortest unbroken run of letters and digits (between `-` or `_`) redacted
/// as a possible key or token. Model names and dates have shorter segments.
const MIN_SECRET_LIKE_CHARS: usize = 20;

/// Prefixes of well-known API keys and JSON Web Tokens.
const SECRET_PREFIXES: [&str; 6] = ["sk-", "sk_", "AIza", "xai-", "gsk_", "eyJ"];

/// Names whose `name=value` values are always redacted.
const SECRET_PARAMETER_NAMES: [&str; 6] =
    ["key", "token", "secret", "password", "signature", "auth"];

/// Builds the error for an unsuccessful response, reading at most
/// [`MAX_ERROR_BODY_BYTES`] of its body.
pub(super) async fn status_error(response: reqwest::Response, provider: &str) -> AiError {
    let status = response.status();
    let detail = read_bounded(response)
        .await
        .and_then(|body| provider_detail(&body));
    status_error_text(provider, status, detail.as_deref())
}

fn status_error_text(provider: &str, status: reqwest::StatusCode, detail: Option<&str>) -> AiError {
    AiError::ApiError(match detail {
        Some(detail) => format!("{provider} returned HTTP {status}: {detail}"),
        None => format!("{provider} returned HTTP {status}"),
    })
}

async fn read_bounded(mut response: reqwest::Response) -> Option<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_ERROR_BODY_BYTES as u64)
    {
        return None;
    }
    let mut body = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if body.len().saturating_add(chunk.len()) > MAX_ERROR_BODY_BYTES {
                    return None;
                }
                body.extend_from_slice(&chunk);
            }
            Ok(None) => return Some(body),
            Err(_) => return None,
        }
    }
}

/// The provider's own error message, one line, bounded and redacted.
fn provider_detail(body: &[u8]) -> Option<String> {
    let json: serde_json::Value = serde_json::from_slice(body).ok()?;
    let error = &json["error"];
    let message = error["message"]
        .as_str()
        .or_else(|| error.as_str())
        .or_else(|| json["message"].as_str())
        .or_else(|| json["detail"].as_str())?;
    let code = [&error["status"], &error["code"], &error["type"]]
        .into_iter()
        .filter_map(serde_json::Value::as_str)
        .find(|code| is_error_code(code));
    let detail = match code {
        Some(code) => one_line_redacted(&format!("{code}: {message}")),
        None => one_line_redacted(message),
    };
    (!detail.is_empty()).then_some(detail)
}

fn is_error_code(code: &str) -> bool {
    (1..=64).contains(&code.len())
        && code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

fn one_line_redacted(text: &str) -> String {
    let printable = text
        .chars()
        .filter(|character| !is_bidi_control(*character))
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let line = printable
        .split_whitespace()
        .map(redact_word)
        .collect::<Vec<_>>()
        .join(" ");
    if line.chars().count() <= MAX_ERROR_DETAIL_CHARS {
        return line;
    }
    let mut cut = line
        .chars()
        .take(MAX_ERROR_DETAIL_CHARS - 1)
        .collect::<String>();
    cut.push('…');
    cut
}

/// Bidirectional formatting characters, which can make a log line read
/// differently from its bytes.
fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

fn redact_word(word: &str) -> String {
    if word.contains("://") {
        return redact_url(word);
    }
    if let Some((name, _)) = word.split_once('=') {
        let lowercase = name.to_ascii_lowercase();
        if SECRET_PARAMETER_NAMES
            .iter()
            .any(|secret| lowercase.contains(secret))
        {
            return format!("{name}=[redacted]");
        }
    }
    let core = word.trim_matches(|character: char| !is_token_character(character));
    if looks_like_secret(core) {
        word.replacen(core, "[redacted]", 1)
    } else {
        word.to_string()
    }
}

/// Keeps a URL's scheme, host and path; drops its query and fragment, and the
/// whole URL when it carries credentials.
fn redact_url(word: &str) -> String {
    let without_query = word.split(['?', '#']).next().unwrap_or_default();
    let authority = without_query
        .split_once("://")
        .map_or("", |(_, rest)| rest.split('/').next().unwrap_or_default());
    if authority.contains('@') {
        return "[redacted URL]".to_string();
    }
    without_query.to_string()
}

fn is_token_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
}

/// A token with a well-known key prefix, or one whose `-`/`_`-separated
/// segment is a long run mixing letters and digits.
fn looks_like_secret(core: &str) -> bool {
    if core.len() >= 16
        && SECRET_PREFIXES
            .iter()
            .any(|prefix| core.starts_with(prefix))
    {
        return true;
    }
    core.chars().all(is_token_character)
        && core.split(['-', '_']).any(|segment| {
            segment.len() >= MIN_SECRET_LIKE_CHARS
                && segment.bytes().any(|byte| byte.is_ascii_digit())
                && segment.bytes().any(|byte| byte.is_ascii_alphabetic())
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn detail(body: &str) -> Option<String> {
        provider_detail(body.as_bytes())
    }

    #[test]
    fn keeps_the_status_prefix_with_and_without_detail() {
        let status = reqwest::StatusCode::TOO_MANY_REQUESTS;
        assert_eq!(
            status_error_text("Gemini", status, None).to_string(),
            "API error: Gemini returned HTTP 429 Too Many Requests"
        );
        assert_eq!(
            status_error_text("Gemini", status, Some("RESOURCE_EXHAUSTED: quota")).to_string(),
            "API error: Gemini returned HTTP 429 Too Many Requests: RESOURCE_EXHAUSTED: quota"
        );
    }

    #[test]
    fn reads_each_provider_error_shape() {
        // Gemini (google.rpc.Status).
        assert_eq!(
            detail(
                r#"{"error":{"code":404,"message":"models/x is not found","status":"NOT_FOUND"}}"#
            )
            .as_deref(),
            Some("NOT_FOUND: models/x is not found")
        );
        // OpenAI and DeepSeek: `code` before `type`, a null code falls back.
        assert_eq!(
            detail(r#"{"error":{"message":"No quota","type":"insufficient_quota","code":null}}"#)
                .as_deref(),
            Some("insufficient_quota: No quota")
        );
        assert_eq!(
            detail(r#"{"error":{"message":"Gone","type":"invalid_request_error","code":"model_not_found"}}"#)
                .as_deref(),
            Some("model_not_found: Gone")
        );
        // Anthropic.
        assert_eq!(
            detail(r#"{"type":"error","error":{"type":"rate_limit_error","message":"Slow down"}}"#)
                .as_deref(),
            Some("rate_limit_error: Slow down")
        );
        // Ollama and OpenAI-compatible servers.
        assert_eq!(
            detail(r#"{"error":"model 'llama' not found"}"#).as_deref(),
            Some("model 'llama' not found")
        );
        assert_eq!(
            detail(r#"{"object":"error","message":"bad model","code":404}"#).as_deref(),
            Some("bad model")
        );
        assert_eq!(
            detail(r#"{"detail":"Not Found"}"#).as_deref(),
            Some("Not Found")
        );
    }

    #[test]
    fn ignores_bodies_without_a_provider_message() {
        assert_eq!(detail("<html><body>Bad gateway</body></html>"), None);
        assert_eq!(detail("upstream connect error"), None);
        assert_eq!(detail(r#"{"error":{"code":500}}"#), None);
        assert_eq!(detail(r#"{"error":{"message":"  \n "}}"#), None);
        assert_eq!(detail(""), None);
    }

    #[test]
    fn rejects_codes_that_are_not_identifiers() {
        assert_eq!(
            detail(r#"{"error":{"message":"m","status":"two words"}}"#).as_deref(),
            Some("m")
        );
        let long = "A".repeat(65);
        assert_eq!(
            detail(&format!(
                r#"{{"error":{{"message":"m","status":"{long}"}}}}"#
            ))
            .as_deref(),
            Some("m")
        );
    }

    #[test]
    fn makes_one_bounded_line() {
        assert_eq!(
            one_line_redacted("first\nsecond\r\n\tthird\u{0}fourth\u{202E}fifth"),
            "first second third fourthfifth"
        );
        let long = "word ".repeat(200);
        let line = one_line_redacted(&long);
        assert_eq!(line.chars().count(), MAX_ERROR_DETAIL_CHARS);
        assert!(line.ends_with('…'));
        let exact = "a".repeat(MAX_ERROR_DETAIL_CHARS);
        assert_eq!(one_line_redacted(&exact), exact);
    }

    #[test]
    fn redacts_url_queries_credentials_and_key_shaped_tokens() {
        assert_eq!(
            one_line_redacted("see https://ai.dev/usage?tab=rate-limit&key=AIzaSecret."),
            "see https://ai.dev/usage"
        );
        assert_eq!(
            one_line_redacted("at https://user:pass@example.com/v1 now"),
            "at [redacted URL] now"
        );
        assert_eq!(
            one_line_redacted("bad api_key=abc123 and token=xyz"),
            "bad api_key=[redacted] and token=[redacted]"
        );
        assert_eq!(
            one_line_redacted("Incorrect API key provided: sk-proj-abcdefghij1234."),
            "Incorrect API key provided: [redacted]."
        );
        assert_eq!(
            one_line_redacted("key 'AIzaSyA1234567890abcdef' is invalid"),
            "key '[redacted]' is invalid"
        );
        assert_eq!(
            one_line_redacted("id a1b2c3d4e5f6a7b8c9d0e1f2a3b4 seen"),
            "id [redacted] seen"
        );
    }

    #[test]
    fn keeps_ordinary_provider_wording() {
        let text = "models/gemini-2.5-flash is not found for API version v1beta, or is not \
                    supported for generateContent. Quota exceeded for metric: \
                    generativelanguage.googleapis.com/generate_content_free_tier_requests, \
                    limit: 0. Please retry in 52.04s. RESOURCE_EXHAUSTED";
        assert_eq!(
            one_line_redacted(text),
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        );
        for model in [
            "claude-3-5-sonnet-20241022",
            "gemini-embedding-exp-03-07",
            "deepseek-v4-flash",
            "text-embedding-3-small",
        ] {
            assert!(!looks_like_secret(model), "{model}");
        }
        assert!(!looks_like_secret("sk-short"));
        assert!(!looks_like_secret("abcdefghijklmnopqrstuvwxyzabc"));
        assert!(!looks_like_secret("123456789012345678901234567"));
        assert!(looks_like_secret("abcdefghijklmnopqrs1"));
        assert!(!looks_like_secret("abcdefghijklmnopqr1"));
        assert!(looks_like_secret("req_abcdefghijklmnopqrs1"));
        assert!(looks_like_secret(
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.sig"
        ));
    }
}
