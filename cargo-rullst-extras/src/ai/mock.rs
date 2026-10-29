//! Deterministic offline assistant (AGENTS.md 3.5).
//!
//! Selected when no provider is configured or the credential is empty or
//! `mock_*`. It runs through the same guarded `StreamingAiClient` as live
//! providers. Inside a project it answers a goal with a fixed two-step plan
//! (create a demo file, then edit it); outside one it first proposes
//! `cargo rullst new rullst-ai-demo` and continues inside the new project.
//! The plan, diff, confirmation and checkpoint paths can therefore be
//! exercised without network access or credentials.

use async_trait::async_trait;
use rullst_ai::{
    AiCancellation, AiError, AiProvider, AiStreamSink, Message, ProviderCapabilities, StreamLimits,
    StreamingAiProvider,
};

/// Marker that starts every action-results message sent back to the model.
pub(super) const RESULTS_MARKER: &str = "[action results]";
/// The demo file the offline plan creates.
pub(super) const DEMO_FILE: &str = "rullst-ai-demo.md";
/// The project the offline plan creates outside a project.
pub(super) const DEMO_PROJECT: &str = "rullst-ai-demo";

#[derive(Debug, Default)]
pub(super) struct MockAssistant;

/// The user's goal without attachments, bounded and on one line.
fn goal_of(message: &str) -> String {
    let goal = message
        .rsplit_once("\n\nGoal:\n")
        .map_or(message, |(_, goal)| goal)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut short: String = goal.chars().take(120).collect();
    if short.len() < goal.len() {
        short.push('…');
    }
    short
}

fn in_project(messages: &[Message]) -> bool {
    messages.iter().any(|message| {
        message.role == "system" && message.content.starts_with(super::prompt::CONTEXT_HEADING)
    })
}

fn is_results(message: &str) -> bool {
    message.starts_with(RESULTS_MARKER) && !message.contains("\n\nGoal:\n")
}

const INTRO: &str = "Offline mock assistant: no AI provider is connected, so this deterministic \
demo shows how `cargo rullst ai` proposes, previews and applies changes. Run \
`cargo rullst ai connect` to use a real model.";

fn demo_file_plan(goal: &str) -> String {
    let content = format!("# Rullst AI demo\n\nGoal: {goal}\n\nStatus: planned\n");
    let write = serde_json::json!({
        "action": "write_file",
        "path": DEMO_FILE,
        "content": content,
    });
    let edit = serde_json::json!({
        "action": "edit_file",
        "path": DEMO_FILE,
        "find": "Status: planned",
        "replace": "Status: reviewed",
    });
    format!(
        "{INTRO}\n\nPlan:\n1. Create `{DEMO_FILE}` describing the goal.\n2. Mark it as reviewed.\n\n\
```rullst-action\n{write}\n```\n\n```rullst-action\n{edit}\n```\n"
    )
}

fn new_project_plan() -> String {
    let new = serde_json::json!({
        "action": "run_rullst",
        "args": ["new", DEMO_PROJECT, "--default", "--blueprint", "blank", "--skip-initial-migration"],
    });
    format!(
        "{INTRO}\n\nYou are outside a Rust project, so the first step is creating one.\n\n\
Plan:\n1. Create the `{DEMO_PROJECT}` project from the blank blueprint.\n\
2. Continue inside it with a first reviewed change.\n\n```rullst-action\n{new}\n```\n"
    )
}

const UPGRADE_INTRO: &str = "Offline mock assistant: no AI provider is connected, so this \
deterministic demo shows how `cargo rullst ai upgrade` reviews fixes for the upgrade findings. Run \
`cargo rullst ai connect` to use a real model.";

fn upgrade_session(messages: &[Message]) -> bool {
    messages.iter().any(|message| {
        message.role == "system" && message.content.contains(super::upgrade::UPGRADE_HEADING)
    })
}

/// `render_page(a, ...)` rewritten as `render_page_with_lang(a, "en", ...)`.
pub(super) fn with_language(text: &str) -> Option<String> {
    const CALL: &str = "render_page(";
    let start = text.find(CALL)?;
    let open = start + CALL.len();
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for (offset, character) in text[open..].char_indices() {
        match character {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '(' | '[' | '{' if !quoted => depth += 1,
            ')' | ']' | '}' if !quoted => depth = depth.checked_sub(1)?,
            ',' if !quoted && depth == 0 => {
                let comma = open + offset + 1;
                let name = if text[..start].ends_with("::") {
                    "render_page_with_lang("
                } else {
                    "rullst::htmx::render_page_with_lang("
                };
                return Some(format!(
                    "{}{name}{} \"en\",{}",
                    &text[..start],
                    &text[open..comma],
                    &text[comma..]
                ));
            }
            _ => {}
        }
    }
    None
}

/// The first `V13-RENDER-PAGE-LANGUAGE` finding as `(path, find, replace)`.
fn page_language_fix(message: &str) -> Option<(String, String, String)> {
    let mut path: Option<String> = None;
    for line in message.lines() {
        let line = line.trim();
        if let Some((_, location)) = line.split_once(" V13-RENDER-PAGE-LANGUAGE at ") {
            path = location.rsplit_once(':').map(|(path, _)| path.to_string());
        } else if line.starts_with(|c: char| c.is_ascii_digit()) {
            path = None;
        } else if let (Some(file), Some(text)) = (&path, line.strip_prefix("line: "))
            && let Some(replace) = with_language(text)
        {
            return Some((file.clone(), text.to_string(), replace));
        }
    }
    None
}

fn upgrade_reply(message: &str) -> String {
    let Some((path, find, replace)) = page_language_fix(message) else {
        return format!(
            "{UPGRADE_INTRO}\n\nThe offline demo only rewrites `render_page` calls \
(V13-RENDER-PAGE-LANGUAGE). A connected model proposes fixes for the other findings; \
`cargo rullst upgrade --dry-run` lists them with their migration rows.\n"
        );
    };
    let edit = serde_json::json!({
        "action": "edit_file",
        "path": path,
        "find": find,
        "replace": replace,
    });
    let check = serde_json::json!({"action": "cargo", "args": ["check"]});
    format!(
        "{UPGRADE_INTRO}\n\nPlan:\n1. Declare the page language in `{path}` with \
`render_page_with_lang` (migration row: Starter page language). Remove `render_page` from the \
`use` list afterwards if no other call needs it.\n2. Check the project.\n\n\
```rullst-action\n{edit}\n```\n\n```rullst-action\n{check}\n```\n"
    )
}

fn system_contains(messages: &[Message], heading: &str) -> bool {
    messages
        .iter()
        .any(|message| message.role == "system" && message.content.contains(heading))
}

/// The value of `key: ` inside the error-context data block, bounded and
/// on one line.
fn context_value(message: &str, key: &str) -> String {
    let value = message
        .lines()
        .skip_while(|line| !line.contains(super::fix::CONTEXT_SOURCE))
        .find_map(|line| line.strip_prefix(key))
        .unwrap_or("unknown");
    let value: String = value.chars().take(160).collect();
    super::term::sanitize(&value).replace('`', "'")
}

fn fix_reply(message: &str) -> String {
    let check = serde_json::json!({"action": "cargo", "args": ["check"]});
    format!(
        "Offline mock assistant: no AI provider is connected, so this deterministic demo shows \
how `cargo rullst ai fix` works. I received the recorded error as quoted data:\n\n\
- panic: `{}`\n- location: `{}`\n- request: `{}`\n\n\
A connected model would explain the cause and propose a reviewed `edit_file` change for that \
location. Run `cargo rullst ai connect` to use one. The demo only checks the project.\n\n\
```rullst-action\n{check}\n```\n",
        context_value(message, "message: "),
        context_value(message, "location: "),
        context_value(message, "request: "),
    )
}

/// The deterministic reply for a conversation.
pub(super) fn reply(messages: &[Message]) -> String {
    let users: Vec<&str> = messages
        .iter()
        .filter(|message| message.role == "user")
        .map(|message| message.content.as_str())
        .collect();
    let last = users.last().copied().unwrap_or("");
    if system_contains(messages, super::review::REVIEW_HEADING) {
        return super::review::mock_review(last);
    }
    if system_contains(messages, super::fix::FIX_HEADING) {
        if is_results(last) {
            return "Offline mock assistant: the check ran. A connected model would continue \
until the panic is fixed and summarize the change.\n"
                .to_string();
        }
        return fix_reply(last);
    }
    if upgrade_session(messages) {
        if is_results(last) {
            return "Offline mock assistant: the reviewed upgrade step ran. A connected model \
would continue with the remaining findings; rerun `cargo rullst upgrade --dry-run` to see what is \
left.\n"
                .to_string();
        }
        return upgrade_reply(last);
    }
    if is_results(last) {
        let created = last.contains(&format!("new {DEMO_PROJECT} "))
            && last.contains("exit status 0")
            && in_project(messages);
        if !created {
            return "Offline mock assistant: I received the results of the reviewed actions. \
A connected model would now continue toward the goal or summarize the change; \
the demo plan is complete.\n"
                .to_string();
        }
        // The project exists now: plan the first change for the original goal.
        let goal = users
            .iter()
            .rev()
            .find(|message| !is_results(message))
            .map_or_else(String::new, |message| goal_of(message));
        return demo_file_plan(&goal);
    }
    if in_project(messages) {
        demo_file_plan(&goal_of(last))
    } else {
        new_project_plan()
    }
}

#[async_trait]
impl AiProvider for MockAssistant {
    fn provider_name(&self) -> &'static str {
        "offline mock"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            explicit_cancellation: true,
            ..ProviderCapabilities::PORTABLE
        }
    }

    async fn prompt(&self, text: &str) -> Result<String, AiError> {
        Ok(reply(&[Message::user(text)]))
    }

    async fn chat(&self, messages: &[Message]) -> Result<String, AiError> {
        Ok(reply(messages))
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>, AiError> {
        Err(AiError::UnsupportedCapability {
            provider: "offline mock",
            capability: "embeddings",
        })
    }
}

#[async_trait]
impl StreamingAiProvider for MockAssistant {
    async fn stream_chat<S>(
        &self,
        messages: &[Message],
        _limits: StreamLimits,
        cancellation: &AiCancellation,
        sink: &mut S,
    ) -> Result<(), AiError>
    where
        S: AiStreamSink,
    {
        // Word-sized chunks exercise incremental display like a live stream.
        let text = reply(messages);
        for chunk in text.split_inclusive(' ') {
            if cancellation.is_cancelled() {
                return Err(AiError::Cancelled);
            }
            sink.send(chunk)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::protocol::{Action, parse};

    fn context() -> Message {
        Message::system(format!(
            "{}\n\nproject inventory",
            super::super::prompt::CONTEXT_HEADING
        ))
    }

    #[test]
    fn a_goal_in_a_project_yields_a_parseable_two_step_plan() {
        let goal = Message::user("Add a \"posts\" page\nwith `html!`");
        let text = reply(&[Message::system("primer"), context(), goal.clone()]);
        let actions: Vec<Action> = parse(&text).into_iter().map(Result::unwrap).collect();
        assert_eq!(actions.len(), 2);
        let Action::WriteFile { path, content } = &actions[0] else {
            panic!("expected write_file");
        };
        assert_eq!(path, DEMO_FILE);
        assert!(content.contains("Goal: Add a \"posts\" page with `html!`"));
        assert!(matches!(&actions[1], Action::EditFile { find, .. } if find == "Status: planned"));
        // Deterministic.
        assert_eq!(text, reply(&[context(), goal]));
    }

    #[test]
    fn outside_a_project_the_plan_starts_with_a_new_project() {
        let text = reply(&[Message::system("primer"), Message::user("build a shop")]);
        let actions: Vec<Action> = parse(&text).into_iter().map(Result::unwrap).collect();
        assert_eq!(
            actions,
            vec![Action::RunRullst {
                args: [
                    "new",
                    DEMO_PROJECT,
                    "--default",
                    "--blueprint",
                    "blank",
                    "--skip-initial-migration"
                ]
                .map(str::to_string)
                .to_vec()
            }]
        );
        // Once the project exists (its context is present), the original goal
        // continues there.
        let results = format!(
            "{RESULTS_MARKER}\n1. cargo rullst new {DEMO_PROJECT} --default: exit status 0"
        );
        let text = reply(&[
            Message::system("primer"),
            context(),
            Message::user("build a shop"),
            Message::assistant("plan"),
            Message::user(results.clone()),
        ]);
        let actions = parse(&text);
        assert_eq!(actions.len(), 2);
        assert!(text.contains("Goal: build a shop"));
        // Without the new context the results only end the plan.
        let text = reply(&[Message::user("build a shop"), Message::user(results)]);
        assert!(parse(&text).is_empty());
    }

    #[test]
    fn results_end_the_plan_without_further_actions() {
        let text = reply(&[
            context(),
            Message::user("goal"),
            Message::assistant("plan"),
            Message::user(format!("{RESULTS_MARKER}\n1. ok")),
        ]);
        assert!(parse(&text).is_empty());
    }

    #[test]
    fn the_goal_is_taken_after_attachments_and_bounded() {
        assert_eq!(goal_of("<data>\n\nGoal:\nship it"), "ship it");
        assert_eq!(goal_of(&"x".repeat(500)).chars().count(), 121);
    }
}
