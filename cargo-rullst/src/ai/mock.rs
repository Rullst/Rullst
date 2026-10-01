//! Deterministic offline assistant (AGENTS.md 3.5).
//!
//! Selected when no provider is configured or the credential is empty or
//! `mock_*`. It runs through the same guarded `StreamingAiClient` as live
//! providers and answers a goal with a fixed two-step plan (create a demo
//! file, then edit it), so the plan, diff, confirmation and checkpoint paths
//! can be exercised without network access or credentials.

use async_trait::async_trait;
use rullst_ai::{
    AiCancellation, AiError, AiProvider, AiStreamSink, Message, ProviderCapabilities, StreamLimits,
    StreamingAiProvider,
};

/// Marker that starts every action-results message sent back to the model.
pub(super) const RESULTS_MARKER: &str = "[action results]";
/// The demo file the offline plan creates.
pub(super) const DEMO_FILE: &str = "rullst-ai-demo.md";

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

/// The deterministic reply for a conversation.
pub(super) fn reply(messages: &[Message]) -> String {
    let last = messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map_or("", |message| message.content.as_str());
    // Results arrive on their own; a new goal may carry earlier results first.
    if last.starts_with(RESULTS_MARKER) && !last.contains("\n\nGoal:\n") {
        return "Offline mock assistant: I received the results of the reviewed actions. \
A connected model would now continue toward the goal or summarize the change; \
the demo plan is complete.\n"
            .to_string();
    }
    let goal = goal_of(last);
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
        "Offline mock assistant: no AI provider is connected, so this deterministic demo \
shows how `cargo rullst ai` proposes, previews and applies changes. Run \
`cargo rullst ai connect` to use a real model.\n\n\
Plan:\n1. Create `{DEMO_FILE}` describing the goal.\n2. Mark it as reviewed.\n\n\
```rullst-action\n{write}\n```\n\n```rullst-action\n{edit}\n```\n"
    )
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

    #[test]
    fn a_goal_yields_a_parseable_two_step_plan() {
        let text = reply(&[
            Message::system("primer"),
            Message::user("Add a \"posts\" page\nwith `html!`"),
        ]);
        let actions: Vec<Action> = parse(&text).into_iter().map(Result::unwrap).collect();
        assert_eq!(actions.len(), 2);
        let Action::WriteFile { path, content } = &actions[0] else {
            panic!("expected write_file");
        };
        assert_eq!(path, DEMO_FILE);
        assert!(content.contains("Goal: Add a \"posts\" page with `html!`"));
        assert!(matches!(&actions[1], Action::EditFile { find, .. } if find == "Status: planned"));
        // Deterministic.
        assert_eq!(
            text,
            reply(&[Message::user("Add a \"posts\" page\nwith `html!`")])
        );
    }

    #[test]
    fn results_end_the_plan_without_further_actions() {
        let text = reply(&[
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
