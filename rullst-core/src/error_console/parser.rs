//! Stack frame extraction and source context inspection for error debugging.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Represents a single frame in the panic stack trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    /// The source file path containing the frame.
    pub file: String,
    /// The line number in the source file.
    pub line: u32,
    /// The name of the function where the panic occurred.
    pub function: String,
}

/// Path fragments of standard-library and Cargo dependency frames.
const DEPENDENCY_FRAME_MARKERS: [&str; 6] = [
    "/rustc/",
    "\\rustc\\",
    ".cargo/registry",
    ".cargo/git",
    ".cargo\\registry",
    ".cargo\\git",
];

/// Parses the stack trace to find the developer's source file and line.
///
/// Accepts `at path:line` and the `at path:line:column` frames of a
/// `std::backtrace::Backtrace` `Display`, and skips standard-library
/// (`/rustc/...`) and Cargo registry or git dependency frames.
#[cfg_attr(mutants, mutants::skip)]
pub fn find_source_location(bt_str: &str) -> Option<(String, u32)> {
    for line in bt_str.lines() {
        let trimmed = line.trim();
        let dependency_frame = DEPENDENCY_FRAME_MARKERS
            .iter()
            .any(|marker| trimmed.contains(marker));
        if trimmed.contains("at ")
            && !dependency_frame
            && (trimmed.contains("/src/")
                || trimmed.contains("\\src\\")
                || trimmed.contains("/examples/")
                || trimmed.contains("\\examples\\")
                || trimmed.contains("/tests/")
                || trimmed.contains("\\tests\\"))
        {
            // Find the location after "at "
            if let Some(pos) = trimmed.find("at ") {
                let path_part = &trimmed[pos + 3..];
                if let Some(location) = split_file_line(path_part) {
                    return Some(location);
                }
            }
        }
    }
    None
}

/// Splits `file:line:column` or `file:line` into the file and line.
fn split_file_line(location: &str) -> Option<(String, u32)> {
    let location = location.trim();
    let numeric = |value: &str| value.parse::<u32>().ok();
    if let Some((rest, column)) = location.rsplit_once(':')
        && numeric(column).is_some()
        && let Some((file, line)) = rest.rsplit_once(':')
        && let Some(line) = numeric(line)
    {
        return Some((file.to_string(), line));
    }
    let (file, line) = location.rsplit_once(':')?;
    Some((file.to_string(), numeric(line)?))
}

/// Reads a file and extracts surrounding context lines.
#[cfg_attr(mutants, mutants::skip)]
pub fn extract_source_context(
    file_path: &str,
    target_line: u32,
    range: u32,
) -> Option<Vec<(u32, String, bool)>> {
    let project_root = std::env::current_dir()
        .map(|cwd| cwd.canonicalize().unwrap_or(cwd))
        .ok()?;

    let target_path = Path::new(file_path);
    if target_path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }

    let absolute_path = project_root.join(target_path);

    let canonical = absolute_path.canonicalize().ok()?;
    if !canonical.starts_with(&project_root) {
        return None;
    }

    let content = fs::read_to_string(&canonical).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    let total_lines = lines.len() as u32;
    let start = if target_line > range {
        target_line - range
    } else {
        1
    };
    let end = std::cmp::min(target_line + range, total_lines);

    let mut context = Vec::new();
    for i in start..=end {
        if i >= 1 && i <= total_lines {
            let line_content = lines[(i - 1) as usize].to_string();
            let is_target = i == target_line;
            context.push((i, line_content, is_target));
        }
    }
    Some(context)
}
