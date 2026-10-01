//! Line input for the chat: standard input read on a helper thread (so Ctrl+C
//! can be observed while waiting) or a fixed script for tests.

use std::collections::VecDeque;
use std::io::{BufRead, Read};
use tokio::sync::mpsc;

/// Longest accepted input line; longer lines are truncated.
const MAX_LINE_BYTES: usize = 64 * 1024;

/// One read result.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Line {
    Text(String),
    /// End of input (Ctrl+D on a terminal).
    Eof,
    /// Ctrl+C while waiting for input.
    Interrupted,
}

pub(super) enum Input {
    Stdin(mpsc::UnboundedReceiver<Option<String>>),
    /// Fixed lines (tests) or none at all (a one-shot plan that never asks).
    Script(VecDeque<String>),
}

/// Reads one line of at most `MAX_LINE_BYTES`, discarding the remainder.
fn read_bounded(reader: &mut impl BufRead) -> Option<String> {
    let mut buffer = Vec::new();
    let read = Read::take(&mut *reader, MAX_LINE_BYTES as u64)
        .read_until(b'\n', &mut buffer)
        .ok()?;
    if read == 0 {
        return None;
    }
    if !buffer.ends_with(b"\n") && buffer.len() >= MAX_LINE_BYTES {
        // Skip the rest of an oversized line.
        let mut rest = Vec::new();
        loop {
            rest.clear();
            match Read::take(&mut *reader, 8192).read_until(b'\n', &mut rest) {
                Ok(0) | Err(_) => break,
                Ok(_) if rest.ends_with(b"\n") => break,
                Ok(_) => {}
            }
        }
    }
    let text = String::from_utf8_lossy(&buffer);
    Some(text.trim_end_matches(['\n', '\r']).to_string())
}

/// Resolves on Ctrl+C when `enabled`; otherwise (or when the handler cannot
/// be installed) never resolves, so it can sit in a `select!` safely.
pub(super) async fn interrupt(enabled: bool) {
    if enabled && tokio::signal::ctrl_c().await.is_ok() {
        return;
    }
    std::future::pending::<()>().await
}

impl Input {
    /// Whether Ctrl+C should be observed (only for real standard input).
    pub(super) fn watches_interrupts(&self) -> bool {
        matches!(self, Self::Stdin(_))
    }

    /// Starts the reader thread. It ends with the process; a blocked read
    /// never delays exit because the thread is detached.
    pub(super) fn stdin() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            let mut handle = stdin.lock();
            loop {
                let line = read_bounded(&mut handle);
                let done = line.is_none();
                if sender.send(line).is_err() || done {
                    break;
                }
            }
        });
        Self::Stdin(receiver)
    }

    /// No input: every read is end of input. Used when nothing will be asked,
    /// so a terminal's keystrokes are never consumed in the background.
    pub(super) fn closed() -> Self {
        Self::Script(VecDeque::new())
    }

    #[cfg(test)]
    pub(super) fn script(lines: &[&str]) -> Self {
        Self::Script(lines.iter().map(|line| (*line).to_string()).collect())
    }

    /// Waits for the next line, observing Ctrl+C on the terminal.
    pub(super) async fn next_line(&mut self) -> Line {
        match self {
            Self::Script(lines) => lines.pop_front().map_or(Line::Eof, Line::Text),
            Self::Stdin(receiver) => {
                tokio::select! {
                    line = receiver.recv() => match line {
                        Some(Some(text)) => Line::Text(text),
                        _ => Line::Eof,
                    },
                    () = interrupt(true) => Line::Interrupted,
                }
            }
        }
    }

    /// Discards lines typed ahead (for example during a long command) so
    /// they can never answer a later confirmation prompt.
    pub(super) fn drain_typeahead(&mut self) {
        if let Self::Stdin(receiver) = self {
            while let Ok(Some(_)) = receiver.try_recv() {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_lines_are_truncated_and_the_next_line_survives() {
        let mut input = format!("{}\nnext\n", "x".repeat(MAX_LINE_BYTES + 100)).into_bytes();
        input.extend_from_slice(b"last");
        let mut reader = std::io::Cursor::new(input);
        assert_eq!(
            read_bounded(&mut reader).map(|line| line.len()),
            Some(MAX_LINE_BYTES)
        );
        assert_eq!(read_bounded(&mut reader).as_deref(), Some("next"));
        assert_eq!(read_bounded(&mut reader).as_deref(), Some("last"));
        assert_eq!(read_bounded(&mut reader), None);
    }

    #[tokio::test]
    async fn scripts_end_with_eof() {
        let mut input = Input::script(&["one"]);
        assert_eq!(input.next_line().await, Line::Text("one".to_string()));
        assert_eq!(input.next_line().await, Line::Eof);
    }
}
