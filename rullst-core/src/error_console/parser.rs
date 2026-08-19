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

/// Parses the stack trace to find the developer's source file and line.
#[cfg_attr(mutants, mutants::skip)]
pub fn find_source_location(bt_str: &str) -> Option<(String, u32)> {
    for line in bt_str.lines() {
        let trimmed = line.trim();
        if trimmed.contains("at ")
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
                if let Some((file, line_str)) = path_part.rsplit_once(':')
                    && let Ok(line_num) = line_str.trim().parse::<u32>()
                {
                    return Some((file.trim().to_string(), line_num));
                }
            }
        }
    }
    None
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
