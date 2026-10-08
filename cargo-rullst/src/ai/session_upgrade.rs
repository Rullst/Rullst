//! A grounded turn (`cargo rullst ai upgrade` and `cargo rullst ai fix`): a
//! summary, then one reviewed goal.

use super::super::upgrade::Brief;
use super::*;

impl<W: Write + Send> Session<'_, W> {
    /// Shows the summary (the dry-run plan or the recorded error) and works on
    /// `goal`. The trusted instructions extend the system prompt; findings,
    /// error contexts and file excerpts travel as untrusted data with the goal.
    pub(in crate::ai) async fn briefed(&mut self, summary: &str, brief: Brief, goal: &str) {
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
        self.turn(goal).await;
        self.finish();
    }
}
