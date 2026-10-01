//! Reviewing and applying the actions of one model step.

use super::super::protocol::Action;
use super::*;

/// The user's answer to one action.
enum Decision {
    Yes,
    No,
    All,
    Quit,
}

/// Indents captured output under its result line.
fn indent(text: &str) -> String {
    text.lines()
        .map(|line| format!("   {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn results_message(results: &[String]) -> String {
    format!(
        "{RESULTS_MARKER}\n{}\nContinue toward the user's goal. If it is complete, reply with a short summary and no action blocks.",
        data("action-results", &results.join("\n"), 24 * 1024)
    )
}

impl<W: Write + Send> Session<'_, W> {
    async fn decide(&mut self) -> Decision {
        self.input.drain_typeahead();
        for _ in 0..3 {
            let question = self
                .style
                .bold("Apply? [y]es / [n]o / [a]ll this turn / [q]uit turn: ");
            let _ = write!(self.out, "{question}");
            let _ = self.out.flush();
            match self.input.next_line().await {
                Line::Text(answer) => match answer.trim().to_ascii_lowercase().as_str() {
                    "y" | "yes" => return Decision::Yes,
                    "n" | "no" => return Decision::No,
                    "a" | "all" => return Decision::All,
                    "q" | "quit" => return Decision::Quit,
                    _ => self.say("Please answer y, n, a or q."),
                },
                Line::Eof => {
                    self.finished = true;
                    return Decision::Quit;
                }
                Line::Interrupted => return Decision::Quit,
            }
        }
        Decision::No
    }

    /// Takes the session checkpoint before the first change; `false` when
    /// none could be taken and the user chose not to continue.
    async fn ensure_checkpoint(&mut self) -> bool {
        if !matches!(self.checkpoint, CheckpointState::Pending) {
            return true;
        }
        match self.take_checkpoint() {
            Ok(checkpoint) => {
                let saved = self.style.green(&format!(
                    "Checkpoint {} ({}) saved before the first change.",
                    checkpoint.reference, checkpoint.commit
                ));
                self.say(&saved);
                for hint in checkpoint.restore_hint() {
                    let hint = self.style.dim(&format!("  {hint}"));
                    self.say(&hint);
                }
                self.checkpoint = CheckpointState::Taken;
                true
            }
            Err(reason) => {
                let warning = self.style.yellow(&format!(
                    "No git checkpoint: {reason}. These changes cannot be restored from git."
                ));
                self.say(&warning);
                if self.confirm("Continue without a checkpoint? [y/N] ").await {
                    self.checkpoint = CheckpointState::Skipped;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Reviews one step's proposals. Results of executed actions go back to
    /// the model as untrusted data for the next step.
    pub(super) async fn review(
        &mut self,
        proposals: &[Result<Action, String>],
        approve_all: &mut bool,
        changed: &mut bool,
        checked_after_change: &mut bool,
        last_step: bool,
    ) -> StepEnd {
        let total = proposals.len();
        let mut results = Vec::new();
        let mut overlay = Overlay::new();
        let mut stopped = false;
        for (index, proposal) in proposals.iter().enumerate() {
            let number = index + 1;
            let name = proposal.as_ref().map_or("action", Action::name);
            let prepared = proposal.clone().and_then(|action| {
                actions::prepare(&action, self.root.as_deref(), &self.cwd, &overlay)
            });
            let prepared = match prepared {
                Ok(prepared) => prepared,
                Err(reason) => {
                    let reason = sanitize(&reason);
                    let line = self
                        .style
                        .red(&format!("[{number}/{total}] rejected {name}: {reason}"));
                    self.say(&line);
                    results.push(format!("{number}. {name}: rejected: {reason}"));
                    continue;
                }
            };
            let text = actions::preview(&prepared, number, total, self.style);
            let _ = write!(self.out, "{text}");
            if let Mode::PlanOnly(_) = self.mode {
                actions::plan(&prepared, &mut overlay);
                let note = self.style.dim("  (not executed)");
                self.say(&note);
                continue;
            }
            let summary = prepared.summary();
            if stopped {
                results.push(format!(
                    "{number}. {summary}: skipped (the user stopped this turn)"
                ));
                continue;
            }
            let decision = if *approve_all && !prepared.always_confirm() {
                Decision::Yes
            } else {
                self.decide().await
            };
            match decision {
                Decision::No => {
                    results.push(format!("{number}. {summary}: declined by the user"));
                    continue;
                }
                Decision::Quit => {
                    stopped = true;
                    results.push(format!(
                        "{number}. {summary}: skipped (the user stopped this turn)"
                    ));
                    continue;
                }
                Decision::All => *approve_all = true,
                Decision::Yes => {}
            }
            if prepared.mutates() && !self.ensure_checkpoint().await {
                results.push(format!("{number}. {summary}: not applied (no checkpoint)"));
                continue;
            }
            let root = self.root.clone();
            let applied = actions::apply(&prepared, root.as_deref(), &mut self.out, self.style);
            if prepared.mutates() && applied.success {
                *changed = true;
                *checked_after_change = false;
            } else if prepared.is_check() && applied.success && *changed {
                *checked_after_change = true;
            }
            let mut line = format!("{number}. {summary}: {}", applied.result);
            if let Some(output) = applied.output {
                line.push_str("\n   output:\n");
                line.push_str(&indent(&prompt::cap(&output, 8 * 1024)));
            }
            results.push(line);
            if let Some(new_root) = applied.new_root {
                self.enter_project(new_root);
            }
        }
        if let Mode::PlanOnly(reason) = self.mode {
            let summary = self.style.yellow(&format!(
                "Plan only ({reason}): {total} proposed action(s) were not executed. Run `cargo rullst ai` in an interactive terminal to review and apply them."
            ));
            self.say(&summary);
            self.notes.push(format!(
                "Note: the actions you proposed last time were shown but not executed ({reason})."
            ));
            return StepEnd::Done;
        }
        let message = results_message(&results);
        if stopped || last_step || self.finished {
            if last_step && !stopped {
                let notice = self
                    .style
                    .yellow("Step limit reached for this goal; send another message to continue.");
                self.say(&notice);
            }
            // The next message carries what happened, so nothing is lost.
            self.notes.push(message);
            return StepEnd::Stop;
        }
        self.history.push(Message::user(message));
        StepEnd::Continue
    }
}
