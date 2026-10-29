//! The action protocol between the model and the CLI.
//!
//! The built-in `rullst-ai` transports do not implement provider-native tool
//! calling, so the model proposes actions as fenced JSON blocks:
//!
//! ````text
//! ```rullst-action
//! {"action": "write_file", "path": "src/models/post.rs", "content": "..."}
//! ```
//! ````
//!
//! Each block is parsed independently and defensively: it must be one JSON
//! object with exactly the keys of a known action and correctly typed,
//! bounded values. A malformed block is rejected with a reason (fed back to
//! the model) and never executed. Parsing only produces proposals; paths and
//! commands are validated again by [`super::paths`] and [`super::commands`].

use serde_json::{Map, Value};

pub(super) const FENCE_START: &str = "```rullst-action";
const FENCE_END: &str = "```";
pub(super) const MAX_ACTIONS: usize = 8;
const MAX_BLOCK_BYTES: usize = 512 * 1024;
pub(super) const MAX_CONTENT_BYTES: usize = 256 * 1024;
const MAX_FIND_BYTES: usize = 64 * 1024;

/// A proposed action, before path or command validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Action {
    WriteFile {
        path: String,
        content: String,
    },
    EditFile {
        path: String,
        find: String,
        replace: String,
    },
    RunRullst {
        args: Vec<String>,
    },
    Cargo {
        args: Vec<String>,
    },
}

impl Action {
    pub(super) fn name(&self) -> &'static str {
        match self {
            Self::WriteFile { .. } => "write_file",
            Self::EditFile { .. } => "edit_file",
            Self::RunRullst { .. } => "run_rullst",
            Self::Cargo { .. } => "cargo",
        }
    }
}

/// Classification of one response line for both parsing and display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Fence {
    Start,
    End,
    None,
}

pub(super) fn fence(line: &str) -> Fence {
    match line.trim() {
        FENCE_START => Fence::Start,
        FENCE_END => Fence::End,
        _ => Fence::None,
    }
}

/// Splits a complete response into action proposals (in order). Every block
/// yields either an action or the reason it was rejected.
pub(super) fn parse(response: &str) -> Vec<Result<Action, String>> {
    let mut results = Vec::new();
    let mut block: Option<String> = None;
    for line in response.lines() {
        match (&mut block, fence(line)) {
            (None, Fence::Start) => block = Some(String::new()),
            (None, _) => {}
            (Some(body), Fence::End) => {
                let body = std::mem::take(body);
                block = None;
                if results.len() == MAX_ACTIONS {
                    results.push(Err(format!(
                        "more than {MAX_ACTIONS} actions in one reply; the rest were ignored"
                    )));
                    return results;
                }
                results.push(parse_block(&body));
            }
            (Some(body), _) => {
                if body.len() + line.len() + 1 > MAX_BLOCK_BYTES {
                    results.push(Err("an action block exceeds 512 KiB".to_string()));
                    return results;
                }
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    if block.is_some() {
        results.push(Err("an action block was not closed with ```".to_string()));
    }
    results
}

fn string_field(object: &Map<String, Value>, key: &str, max: usize) -> Result<String, String> {
    match object.get(key) {
        Some(Value::String(value)) if value.len() <= max => Ok(value.clone()),
        Some(Value::String(_)) => Err(format!("`{key}` exceeds {} KiB", max / 1024)),
        Some(_) => Err(format!("`{key}` must be a string")),
        None => Err(format!("`{key}` is required")),
    }
}

fn args_field(object: &Map<String, Value>) -> Result<Vec<String>, String> {
    let Some(Value::Array(values)) = object.get("args") else {
        return Err("`args` must be an array of strings".to_string());
    };
    if values.is_empty() || values.len() > 9 {
        return Err("`args` must hold between 1 and 9 strings".to_string());
    }
    values
        .iter()
        .map(|value| match value {
            Value::String(value) if value.len() <= 256 => Ok(value.clone()),
            _ => Err("`args` must be an array of short strings".to_string()),
        })
        .collect()
}

fn parse_block(body: &str) -> Result<Action, String> {
    let value: Value = serde_json::from_str(body.trim())
        .map_err(|_| "the action is not valid JSON".to_string())?;
    let Value::Object(object) = value else {
        return Err("the action must be a JSON object".to_string());
    };
    let Some(Value::String(kind)) = object.get("action") else {
        return Err("`action` must name an action".to_string());
    };
    let allowed: &[&str] = match kind.as_str() {
        "write_file" => &["action", "path", "content"],
        "edit_file" => &["action", "path", "find", "replace"],
        "run_rullst" | "cargo" => &["action", "args"],
        _ => {
            let shown: String = kind.chars().take(32).collect();
            return Err(format!(
                "unknown action `{}`; allowed: write_file, edit_file, run_rullst, cargo",
                super::term::sanitize(&shown)
            ));
        }
    };
    if let Some(extra) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        let shown: String = extra.chars().take(32).collect();
        return Err(format!(
            "unexpected field `{}` for {kind}",
            super::term::sanitize(&shown)
        ));
    }
    match kind.as_str() {
        "write_file" => Ok(Action::WriteFile {
            path: string_field(&object, "path", 512)?,
            content: string_field(&object, "content", MAX_CONTENT_BYTES)?,
        }),
        "edit_file" => {
            let find = string_field(&object, "find", MAX_FIND_BYTES)?;
            if find.is_empty() {
                return Err("`find` must not be empty".to_string());
            }
            Ok(Action::EditFile {
                path: string_field(&object, "path", 512)?,
                find,
                replace: string_field(&object, "replace", MAX_CONTENT_BYTES)?,
            })
        }
        "run_rullst" => Ok(Action::RunRullst {
            args: args_field(&object)?,
        }),
        _ => Ok(Action::Cargo {
            args: args_field(&object)?,
        }),
    }
}

/// Streams model text to the terminal while hiding action blocks, which are
/// rendered separately after review. Text is sanitised before display.
#[derive(Default)]
pub(super) struct DisplayFilter {
    line: String,
    /// Byte length of the leading whitespace of a held-back line, tracked
    /// as characters arrive so each one costs constant time.
    indent: usize,
    /// The current partial line was already printed.
    flushed: bool,
    in_block: bool,
    /// Blank lines right after a hidden block are dropped too.
    after_block: bool,
    pub(super) hidden_blocks: usize,
}

impl DisplayFilter {
    /// Feeds one chunk and returns the text to print now.
    pub(super) fn push(&mut self, chunk: &str) -> String {
        let mut output = String::new();
        for character in chunk.chars() {
            if character == '\n' {
                self.end_line(&mut output);
                continue;
            }
            self.line.push(character);
            if self.in_block || self.flushed {
                if !self.in_block {
                    output.push(character);
                }
                continue;
            }
            // Hold a line back while it could still become an action fence.
            if self.indent + character.len_utf8() == self.line.len() && character.is_whitespace() {
                self.indent = self.line.len();
            }
            let trimmed = &self.line[self.indent..];
            if !FENCE_START.starts_with(trimmed) && !trimmed.starts_with(FENCE_START) {
                output.push_str(&self.line);
                self.flushed = true;
            }
        }
        super::term::sanitize(&output)
    }

    fn end_line(&mut self, output: &mut String) {
        let line = std::mem::take(&mut self.line);
        self.indent = 0;
        let flushed = std::mem::replace(&mut self.flushed, false);
        if self.in_block {
            if fence(&line) == Fence::End {
                self.in_block = false;
                self.after_block = true;
            }
            return;
        }
        if !flushed && fence(&line) == Fence::Start {
            self.in_block = true;
            self.hidden_blocks += 1;
            return;
        }
        if !flushed && self.after_block && line.trim().is_empty() {
            return;
        }
        self.after_block = false;
        if !flushed {
            output.push_str(&line);
        }
        output.push('\n');
    }

    /// Flushes a trailing partial line at the end of a response.
    pub(super) fn finish(&mut self) -> String {
        let line = std::mem::take(&mut self.line);
        self.indent = 0;
        if self.in_block || self.flushed || fence(&line) == Fence::Start {
            self.flushed = false;
            return String::new();
        }
        super::term::sanitize(&line)
    }
}

#[cfg(test)]
#[path = "tests/protocol.rs"]
mod tests;
