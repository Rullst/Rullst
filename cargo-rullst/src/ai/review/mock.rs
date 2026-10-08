//! The offline assistant's deterministic review (AGENTS.md 3.5): it flags
//! `unwrap()`, `expect(` and `panic!` on added lines outside test files and
//! every redacted secret, in diff order, in the same `rullst-review` format a
//! model uses.

use super::DIFF_SOURCE;
use super::report::FENCE;

const MAX_FINDINGS: usize = 30;

fn test_file(path: &str) -> bool {
    path.starts_with("tests/")
        || path.contains("/tests/")
        || path.ends_with("tests.rs")
        || path.ends_with("_test.rs")
        || path.starts_with("benches/")
}

/// `+ 42| code` as `(42, code)`.
fn added_line(line: &str) -> Option<(u64, &str)> {
    let rest = line.strip_prefix('+')?;
    let (number, code) = rest.split_once("| ")?;
    Some((number.trim().parse().ok()?, code))
}

/// The deterministic review of the diff blocks in `message`.
pub(crate) fn mock_review(message: &str) -> String {
    let mut findings = Vec::new();
    let mut file: Option<String> = None;
    for line in message.lines() {
        if let Some(rest) = line.strip_prefix("<untrusted-data source=\"") {
            file = rest
                .split('"')
                .next()
                .and_then(|label| label.strip_prefix(DIFF_SOURCE))
                .map(str::to_string);
            continue;
        }
        let (Some(path), Some((number, code))) = (&file, added_line(line)) else {
            continue;
        };
        if findings.len() == MAX_FINDINGS {
            break;
        }
        if code.contains("[redacted") {
            findings.push(serde_json::json!({
                "file": path,
                "line": number,
                "severity": "high",
                "title": "Secret-like value in the change",
                "detail": "A value matching a high-signal secret pattern was added; it was redacted before this review.",
                "suggestion": "Remove it from the source, rotate it, and read it from the environment or a secret store.",
            }));
        } else if !test_file(path)
            && (code.contains(".unwrap()") || code.contains(".expect(") || code.contains("panic!("))
        {
            findings.push(serde_json::json!({
                "file": path,
                "line": number,
                "severity": "medium",
                "title": "Possible panic in a production path",
                "detail": "`unwrap()`, `expect()` and `panic!` abort the request outside tests (zero-panic policy, AGENTS.md 3.1).",
                "suggestion": "Return a typed error with `?` and map it to a response instead.",
            }));
        }
    }
    let summary = if findings.is_empty() {
        "Offline mock review: no added unwrap/expect/panic outside tests and no redacted secret."
            .to_string()
    } else {
        format!(
            "Offline mock review: {} finding(s) from fixed patterns.",
            findings.len()
        )
    };
    let review = serde_json::json!({ "summary": summary, "findings": findings });
    format!(
        "Offline mock assistant: no AI provider is connected, so this deterministic review only \
flags `unwrap()`, `expect()` and `panic!` on added lines outside tests, and redacted secrets. Run \
`cargo rullst ai connect` for a model review.\n\n{FENCE}\n{review}\n```\n"
    )
}
