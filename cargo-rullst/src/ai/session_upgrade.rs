//! The `cargo rullst ai upgrade` turn: the plan, then one reviewed goal.

use super::super::upgrade::{Brief, GOAL};
use super::*;

impl<W: Write + Send> Session<'_, W> {
    /// Shows the dry-run plan and works on its findings. The trusted
    /// instructions extend the system prompt; the findings and file excerpts
    /// travel as untrusted data with the goal.
    pub(in crate::ai) async fn upgrade(&mut self, summary: &str, brief: Brief) {
        let Brief {
            instructions,
            attachments,
            notes,
        } = brief;
        self.system = Message::system(format!("{}\n\n{instructions}", self.system.content));
        self.banner();
        for line in summary.lines() {
            let line = sanitize(line);
            self.say(&line);
        }
        for note in notes {
            let note = self.style.dim(&sanitize(&note));
            self.say(&note);
        }
        self.attachments.extend(attachments);
        self.turn(GOAL).await;
        self.finish();
    }
}
