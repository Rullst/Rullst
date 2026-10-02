//! The chat loop: one-shot goals and the interactive REPL.
//!
//! Each user turn may take several model steps. A step streams the answer,
//! hides action blocks while streaming, then reviews the parsed actions one
//! by one. Without an interactive terminal (or with `--dry-run`) actions are
//! only shown, never executed.

use super::actions::{self, Overlay, Prepared};
use super::backend::Backend;
use super::checkpoint::{self, Checkpoint};
use super::credentials::Prices;
use super::input::{Input, Line, interrupt};
use super::mock::RESULTS_MARKER;
use super::prompt::{self, data};
use super::protocol::{self, DisplayFilter};
use super::term::{Style, sanitize};
use super::usage::UsageTotals;
use rullst_ai::{AiCancellation, AiError, AiGuardrails, Message};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

#[path = "session_review.rs"]
mod review;
#[path = "session_upgrade.rs"]
mod upgrade_turn;

/// Model steps allowed for one user turn.
const MAX_STEPS: usize = 8;
/// Conversation bytes kept besides the system prompt; older turns drop first.
const MAX_HISTORY_BYTES: usize = 192 * 1024;
/// Largest file the user can attach with `/add`.
const MAX_ATTACHMENT_BYTES: u64 = 64 * 1024;

/// Whether the session may execute actions, and why not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    Execute,
    PlanOnly(&'static str),
}

enum CheckpointState {
    Pending,
    Taken,
    Skipped,
}

/// Presentation and policy for one session.
pub(super) struct Settings {
    /// Short provider label, e.g. `OpenAI · gpt-4o-mini`.
    pub label: String,
    /// An optional line shown under the banner.
    pub notice: Option<String>,
    pub style: Style,
    pub mode: Mode,
    /// Canonical project root; `None` allows only `cargo rullst new`.
    pub root: Option<PathBuf>,
    /// Where `cargo rullst new` creates a project outside one.
    pub cwd: PathBuf,
    /// User-configured prices for cost estimates.
    pub prices: Option<Prices>,
}

pub(super) struct Session<'a, W: Write + Send> {
    backend: &'a Backend,
    label: String,
    notice: Option<String>,
    out: W,
    input: Input,
    style: Style,
    mode: Mode,
    root: Option<PathBuf>,
    cwd: PathBuf,
    prices: Option<Prices>,
    usage: UsageTotals,
    system: Message,
    /// Project context as a separate system message (inspected on its own).
    context: Option<Message>,
    history: Vec<Message>,
    attachments: Vec<String>,
    notes: Vec<String>,
    checkpoint: CheckpointState,
    /// Input ended while a prompt was waiting: finish after this turn.
    finished: bool,
}

/// What ended a step.
enum StepEnd {
    Done,
    Continue,
    Stop,
}

impl<'a, W: Write + Send> Session<'a, W> {
    pub(super) fn new(backend: &'a Backend, settings: Settings, out: W, input: Input) -> Self {
        let Settings {
            label,
            notice,
            style,
            mode,
            root,
            cwd,
            prices,
        } = settings;
        let context = root
            .as_deref()
            .map(|root| Message::system(prompt::project_context(root)));
        Self {
            backend,
            label,
            notice,
            out,
            input,
            style,
            mode,
            system: Message::system(prompt::system_prompt(root.is_some())),
            context,
            root,
            cwd,
            prices,
            usage: UsageTotals::default(),
            history: Vec::new(),
            attachments: Vec::new(),
            notes: Vec::new(),
            checkpoint: CheckpointState::Pending,
            finished: false,
        }
    }

    #[cfg(test)]
    pub(super) fn into_output(self) -> W {
        self.out
    }

    fn say(&mut self, text: &str) {
        let _ = writeln!(self.out, "{text}");
        let _ = self.out.flush();
    }

    fn banner(&mut self) {
        let title = self.style.bold("Rullst AI");
        let label = sanitize(&self.label);
        let project = match &self.root {
            Some(root) => format!(
                "project {}",
                sanitize(
                    &root
                        .file_name()
                        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
                )
            ),
            None => "no project (only `cargo rullst new` is available)".to_string(),
        };
        let line = format!("{title} · {label} · {project}");
        self.say(&line);
        if let Some(notice) = self.notice.clone() {
            let notice = self.style.dim(&notice);
            self.say(&notice);
        }
        if let Mode::PlanOnly(reason) = self.mode {
            let notice = self.style.yellow(&format!(
                "Plan only: {reason}; actions are shown, never executed."
            ));
            self.say(&notice);
        }
    }

    /// Runs one goal and returns.
    pub(super) async fn one_shot(&mut self, goal: &str) {
        self.banner();
        self.turn(goal).await;
        self.finish();
    }

    /// Prints the session usage, when any answer reported it.
    fn finish(&mut self) {
        if let Some(summary) = self.usage.summary() {
            let summary = self.style.dim(&summary);
            self.say(&summary);
        }
    }

    /// Continues the session inside a project the assistant just created.
    fn enter_project(&mut self, root: PathBuf) {
        let name = root
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        self.system = Message::system(prompt::system_prompt(true));
        self.context = Some(Message::system(prompt::project_context(&root)));
        self.root = Some(root);
        self.checkpoint = CheckpointState::Pending;
        let notice = self.style.green(&format!(
            "Now working in the new project {}; paths are relative to it.",
            sanitize(&name)
        ));
        self.say(&notice);
    }

    /// Reads goals until end of input or `/exit`.
    pub(super) async fn repl(&mut self) {
        self.banner();
        let hint = self.style.dim(
            "Type a goal or question. /help lists commands; Ctrl+C cancels an answer; Ctrl+D exits.",
        );
        self.say(&hint);
        while !self.finished {
            if self.mode == Mode::Execute {
                let prompt = self.style.cyan("› ");
                let _ = write!(self.out, "\n{prompt}");
                let _ = self.out.flush();
            }
            match self.input.next_line().await {
                Line::Eof => break,
                Line::Interrupted => {
                    self.say("\n(Ctrl+D or /exit quits)");
                }
                Line::Text(text) => {
                    let text = text.trim().to_string();
                    if text.is_empty() {
                        continue;
                    }
                    if text.starts_with('/') {
                        if !self.command(&text) {
                            break;
                        }
                        continue;
                    }
                    self.turn(&text).await;
                }
            }
        }
        self.finish();
    }

    /// Handles a slash command; `false` ends the session.
    fn command(&mut self, text: &str) -> bool {
        let (name, argument) = text.split_once(' ').unwrap_or((text, ""));
        match name {
            "/exit" | "/quit" => return false,
            "/help" => {
                self.say(
                    "/add <path>  share a project file with the assistant for your next message",
                );
                self.say("/reset       forget the conversation");
                self.say("/exit        quit (or Ctrl+D)");
            }
            "/reset" => {
                self.history.clear();
                self.attachments.clear();
                self.notes.clear();
                self.say("Conversation cleared.");
            }
            "/add" => self.attach(argument.trim()),
            _ => self.say("Unknown command; /help lists the commands."),
        }
        true
    }

    fn attach(&mut self, raw: &str) {
        let Some(root) = self.root.clone() else {
            self.say("Files can only be attached inside a Rust project.");
            return;
        };
        let target = match super::paths::resolve(&root, raw) {
            Ok(target) if target.exists => target,
            Ok(_) => return self.say("That file does not exist."),
            Err(error) => return self.say(&format!("Not attached: {error}.")),
        };
        let text = std::fs::symlink_metadata(&target.absolute)
            .ok()
            .filter(|metadata| metadata.len() <= MAX_ATTACHMENT_BYTES)
            .and_then(|_| std::fs::read(&target.absolute).ok())
            .and_then(|bytes| String::from_utf8(bytes).ok());
        let Some(text) = text else {
            return self.say("Not attached: only UTF-8 text files up to 64 KiB can be shared.");
        };
        if let Some(threat) = AiGuardrails::inspect(&text).threat() {
            return self.say(&format!(
                "Not attached: the file matches the `{}` prompt-injection heuristic.",
                threat.code()
            ));
        }
        let label = format!("file {}", target.display);
        self.attachments
            .push(data(&label, &text, MAX_ATTACHMENT_BYTES as usize));
        let message = format!(
            "Attached {} ({} bytes) to your next message.",
            sanitize(&target.display),
            text.len()
        );
        self.say(&message);
    }

    /// The user's message with pending attachments and notes.
    fn compose(&mut self, goal: &str) -> String {
        if self.attachments.is_empty() && self.notes.is_empty() {
            return goal.to_string();
        }
        let mut parts: Vec<String> = std::mem::take(&mut self.attachments);
        parts.append(&mut self.notes);
        format!("{}\n\nGoal:\n{goal}", parts.join("\n\n"))
    }

    fn trim_history(&mut self) {
        let size = |messages: &[Message]| messages.iter().map(|m| m.content.len()).sum::<usize>();
        while self.history.len() > 2 && size(&self.history) > MAX_HISTORY_BYTES {
            self.history.drain(..2);
        }
    }

    async fn turn(&mut self, goal: &str) {
        if let Some(threat) = AiGuardrails::inspect(goal).threat() {
            let message = format!(
                "Not sent: your message matches the `{}` prompt-injection heuristic of the rullst-ai guardrails. Please rephrase it.",
                threat.code()
            );
            self.say(&message);
            return;
        }
        let message = self.compose(goal);
        self.history.push(Message::user(message));
        let mut approve_all = false;
        let mut changed = false;
        let mut checked_after_change = false;
        for step in 0..MAX_STEPS {
            let Some(response) = self.ask().await else {
                return;
            };
            let proposals = protocol::parse(&response);
            if proposals.is_empty() {
                break;
            }
            let last_step = step + 1 == MAX_STEPS;
            let end = self
                .review(
                    &proposals,
                    &mut approve_all,
                    &mut changed,
                    &mut checked_after_change,
                    last_step,
                )
                .await;
            match end {
                StepEnd::Continue => {}
                StepEnd::Done | StepEnd::Stop => break,
            }
        }
        if changed && !checked_after_change && !self.finished {
            self.offer_check().await;
        }
    }

    /// Sends the conversation and records the answer. `None` when the step
    /// failed or was cancelled (the unanswered message is removed).
    async fn ask(&mut self) -> Option<String> {
        self.trim_history();
        let mut messages = Vec::with_capacity(self.history.len() + 2);
        messages.push(self.system.clone());
        messages.extend(self.context.iter().cloned());
        messages.extend(self.history.iter().cloned());
        let backend = self.backend;
        let watch = self.input.watches_interrupts();
        let style = self.style;
        let cancellation = AiCancellation::new();
        let mut filter = DisplayFilter::default();
        let mut full = String::new();
        let mut waiting = style.color;
        let started = Instant::now();
        let _ = writeln!(self.out);
        if waiting {
            let _ = write!(self.out, "{}", style.dim("thinking…"));
            let _ = self.out.flush();
        }
        let out = &mut self.out;
        let mut sink = |chunk: &str| -> Result<(), AiError> {
            if waiting {
                let _ = write!(out, "\r\x1b[2K");
                waiting = false;
            }
            full.push_str(chunk);
            let shown = filter.push(chunk);
            out.write_all(shown.as_bytes())
                .and_then(|()| out.flush())
                .map_err(|_| AiError::StreamSink)
        };
        let result = tokio::select! {
            result = backend.respond(&messages, &cancellation, &mut sink) => result,
            () = interrupt(watch) => {
                cancellation.cancel();
                Err(AiError::Cancelled)
            }
        };
        if waiting {
            let _ = write!(self.out, "\r\x1b[2K");
        }
        let tail = filter.finish();
        let _ = write!(self.out, "{tail}");
        if !full.ends_with('\n') {
            let _ = writeln!(self.out);
        }
        let reported = match result {
            Ok(usage) => usage,
            Err(error) => {
                let message = match error {
                    AiError::Cancelled => "[cancelled]".to_string(),
                    AiError::BlockedByFirewall(code) => {
                        format!("[blocked by the rullst-ai guardrails: {}]", sanitize(&code))
                    }
                    other => format!("[provider error: {}]", sanitize(&other.to_string())),
                };
                let message = style.red(&message);
                self.say(&message);
                if let Some(unanswered) = self.history.pop()
                    && unanswered.content.starts_with(RESULTS_MARKER)
                {
                    // Keep executed results for the next turn instead of losing them.
                    self.notes.push(unanswered.content);
                }
                return None;
            }
        };
        let elapsed = started.elapsed().as_secs_f64();
        let tokens = self.usage.record(reported, self.prices);
        let usage = style.dim(&format!(
            "[{} · {tokens} · {elapsed:.1}s]",
            sanitize(&self.label)
        ));
        self.say(&usage);
        let recorded = match AiGuardrails::inspect(&full).threat() {
            Some(threat) => format!(
                "[previous answer withheld from the conversation: it matched the `{}` heuristic]",
                threat.code()
            ),
            None => full.clone(),
        };
        self.history.push(Message::assistant(recorded));
        Some(full)
    }

    async fn offer_check(&mut self) {
        if self.mode != Mode::Execute || self.root.is_none() {
            return;
        }
        if !self.confirm("Run `cargo check` now? [y/N] ").await {
            return;
        }
        let Some(root) = self.root.clone() else {
            return;
        };
        let Ok(invocation) = super::commands::validate_cargo(vec!["check".to_string()]) else {
            return;
        };
        let prepared = Prepared::Command(invocation);
        let applied = actions::apply(&prepared, Some(&root), &mut self.out, self.style);
        let mut note = format!("cargo check after the last changes: {}", applied.result);
        if let Some(output) = applied.output.filter(|_| !applied.success) {
            note.push('\n');
            note.push_str(&output);
        }
        self.notes.push(data("cargo-check", &note, 12 * 1024));
    }

    /// A yes/no question; anything but `y`/`yes` is no.
    async fn confirm(&mut self, question: &str) -> bool {
        self.input.drain_typeahead();
        let _ = write!(self.out, "{question}");
        let _ = self.out.flush();
        match self.input.next_line().await {
            Line::Text(answer) => {
                matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
            }
            Line::Eof => {
                self.finished = true;
                false
            }
            Line::Interrupted => false,
        }
    }

    fn take_checkpoint(&mut self) -> Result<Checkpoint, String> {
        let Some(root) = self.root.clone() else {
            return Err("no project root".to_string());
        };
        checkpoint::create(&root, &checkpoint::timestamp()).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
#[path = "tests/session.rs"]
mod tests;
