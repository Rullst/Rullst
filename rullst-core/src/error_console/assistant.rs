//! The "Ask your AI assistant" panel of the development error page.
//!
//! It gives the developer a prompt to copy into their own editor or terminal
//! assistant, quoting only what the page shows: the error message, its
//! location, the first project frames of the stack trace and the request
//! method and path (no query string, headers, cookies or body). When the
//! server records errors it also names the path of the recorded JSON context,
//! `GET /_rullst/errors/{id}`. The page never runs an assistant and never
//! contacts a provider.

use super::store;
use crate::html::escape_str;
use std::fmt::Write as _;

/// Title of the panel; the retired autofix endpoint points to it.
pub(crate) const PANEL_TITLE: &str = "Ask your AI assistant";

/// The recorded-context path with an id placeholder, as the retired autofix
/// endpoint names it.
pub(crate) const CONTEXT_PATH_TEMPLATE: &str = "/_rullst/errors/<error-id>";

/// Project frames quoted in the prompt; the page shows the whole trace.
pub(crate) const PROMPT_FRAMES: usize = 5;

/// What the prompt quotes from the error page.
pub(crate) struct PromptSource<'a> {
    pub(crate) message: &'a str,
    /// The file (with `/` separators) and line the page shows.
    pub(crate) location: Option<(&'a str, u32)>,
    /// The backtrace the page shows; empty when none was captured.
    pub(crate) backtrace: &'a str,
    /// Method and path (without query string) of the request that panicked.
    pub(crate) request: Option<(&'a str, &'a str)>,
    /// Id of the recorded context, when this server serves it.
    pub(crate) error_id: Option<&'a str>,
}

/// Path at which the server serves the recorded context `id`.
pub(crate) fn context_path(id: &str) -> String {
    format!("/_rullst/errors/{id}")
}

/// The prompt as plain text (the panel escapes it).
pub(crate) fn prompt(source: &PromptSource<'_>) -> String {
    let mut text = String::with_capacity(1024);
    text.push_str("A request to my Rullst application panicked on the development server.\n\n");
    let _ = writeln!(
        text,
        "Error: {}",
        store::bounded(source.message, store::MAX_MESSAGE_BYTES)
    );
    match source.location {
        Some((file, line)) => {
            let _ = writeln!(
                text,
                "Location: {}:{line}",
                store::bounded(file, store::MAX_FILE_BYTES)
            );
        }
        None => text.push_str("Location: unknown\n"),
    }
    if let Some((method, path)) = source.request {
        let _ = writeln!(
            text,
            "Request: {} {}",
            store::bounded(method, store::MAX_METHOD_BYTES),
            store::bounded(path, store::MAX_PATH_BYTES)
        );
    }
    let frames = store::project_frames(source.backtrace);
    if source.backtrace.trim().is_empty() {
        text.push_str("Stack trace: not captured (set RUST_BACKTRACE=1 to record one).\n");
    } else if frames.is_empty() {
        text.push_str("Stack trace: no frame from the project's own code was captured.\n");
    } else {
        text.push_str("First stack frames from the project:\n");
        for frame in frames.iter().take(PROMPT_FRAMES) {
            let _ = writeln!(text, "- {frame}");
        }
    }
    if let Some(id) = source.error_id {
        let _ = writeln!(
            text,
            "Recorded context (JSON): GET {} on the same development server, from this machine only; it expires after {} minutes or when the server restarts.",
            context_path(id),
            store::TTL.as_secs() / 60
        );
    }
    text.push_str(
        "\nFollow the project's AGENTS.md. Explain the cause, then propose the smallest fix as a diff and wait for my confirmation before you edit any file.",
    );
    text
}

/// The panel's HTML. Every quoted value is escaped. The copy button stays
/// hidden until the page's script, which carries the CSP nonce, reveals it,
/// so a policy that blocks the script leaves only the selectable text.
pub(crate) fn panel(source: &PromptSource<'_>) -> String {
    let context = match source.error_id {
        Some(id) => format!(
            "<p class=\"fix-note\">A terminal assistant can also read this error as JSON at \
<code class=\"context-url\">{}</code> on this server, from this machine only, for {} minutes \
or until the server restarts.</p>",
            escape_str(&context_path(id)),
            store::TTL.as_secs() / 60
        ),
        None => "<p class=\"fix-note\">This server does not record errors, so the prompt is all \
the assistant receives from this page.</p>"
            .to_string(),
    };
    format!(
        r#"<div class="ai-panel">
                    <p class="fix-note">Select this prompt and copy it into your editor's or terminal's AI assistant. It quotes only this page: the error, its location, the first project frames of the stack trace and the request method and path.</p>
                    <textarea id="ai-prompt" class="ai-prompt" readonly rows="12" spellcheck="false" aria-label="Prompt for your AI assistant">{prompt}</textarea>
                    <button id="btn-copy" type="button" class="btn-copy" hidden>Copy prompt</button>
                    {context}
                    <p class="fix-note">Give the assistant your project's <code>AGENTS.md</code> (<code>cargo rullst generate:ai-context</code> writes it) and https://rullst.github.io/Rullst/llms.txt, and review every edit it proposes.</p>
                    <p class="fix-note">This page makes no network request and sends nothing to an AI provider.</p>
                </div>"#,
        prompt = escape_str(&prompt(source)),
    )
}
