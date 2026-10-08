//! High-signal secret patterns in files tracked by Git.
//!
//! Only the file, the line and a redacted preview (the first four characters
//! and `…`) leave this module; a matched value is never stored or printed.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;

use crate::generators::audit_compliance::EvidenceStatus;

use super::catalog::SECRETS;
use super::model::{Check, Finding};
use super::sources::{MAX_FILE_BYTES, read_bounded};

/// At most this many tracked files are read; more makes the scan incomplete.
const MAX_TRACKED_FILES: usize = 100_000;
/// A generic `.env` assignment needs a value at least this long.
const MIN_ENV_VALUE: usize = 20;

/// Why the tracked-file list is unavailable.
pub(super) type Tracked = Result<Vec<PathBuf>, &'static str>;

/// `git ls-files` in `root`, relative paths. `git` is the program to run.
pub(super) fn tracked_files(git: &OsStr, root: &Path) -> Tracked {
    let output = Command::new(git)
        .args(["ls-files", "-z", "--cached"])
        .current_dir(root)
        .output()
        .map_err(|_| "git is unavailable, so tracked files could not be listed")?;
    if !output.status.success() {
        return Err("the project is not a Git work tree, so tracked files could not be listed");
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| PathBuf::from(String::from_utf8_lossy(path).into_owned()))
        .collect())
}

/// The token patterns with their description, or `None` when one does not
/// compile (the check then reports `ERROR` instead of a clean scan).
static PATTERNS: LazyLock<Option<Vec<(&'static str, Regex)>>> = LazyLock::new(|| {
    [
        (
            "private key header",
            r"-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY(?: BLOCK)?-----",
        ),
        ("AWS access key ID", r"\bAKIA[0-9A-Z]{16}\b"),
        ("Stripe live secret key", r"\bsk_live_[0-9A-Za-z]{10,}"),
        ("GitHub personal access token", r"\bghp_[0-9A-Za-z]{36}\b"),
        (
            "GitHub fine-grained token",
            r"\bgithub_pat_[0-9A-Za-z_]{22,}",
        ),
        ("Slack token", r"\bxox[bp]-[0-9A-Za-z-]{10,}"),
    ]
    .into_iter()
    .map(|(kind, pattern)| Regex::new(pattern).ok().map(|regex| (kind, regex)))
    .collect()
});

static ENV_ASSIGNMENT: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:export\s+)?([A-Za-z0-9_]*(?:_SECRET|_KEY))\s*=\s*(.*?)\s*$").ok()
});

/// The first four characters followed by `…`.
pub(super) fn redact(value: &str) -> String {
    let mut preview: String = value.chars().take(4).collect();
    preview.push('…');
    preview
}

/// A committed `.env` file (`.env`, `.env.production`, ...), not a template.
fn committed_env(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };
    let template = [".example", ".sample", ".template", ".dist", ".defaults"]
        .iter()
        .any(|suffix| name.ends_with(suffix));
    (name == ".env" || name.starts_with(".env.")) && !template
}

fn placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.starts_with("${")
        || value.starts_with('<')
        || [
            "change",
            "example",
            "placeholder",
            "your_",
            "your-",
            "xxxx",
            "replace",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Findings in one tracked text file.
pub(super) fn scan_text(display: &str, path: &Path, content: &str) -> Option<Vec<Finding>> {
    let patterns = PATTERNS.as_ref()?;
    let env = ENV_ASSIGNMENT.as_ref()?;
    let env_file = committed_env(path);
    let mut findings = Vec::new();
    for (index, line) in content.lines().enumerate() {
        for (kind, regex) in patterns {
            for found in regex.find_iter(line) {
                findings.push(
                    Finding::new(
                        display,
                        Some(index + 1),
                        format!("{kind} in a tracked file"),
                    )
                    .with_preview(redact(found.as_str())),
                );
            }
        }
        if !env_file || line.trim_start().starts_with('#') {
            continue;
        }
        if let Some(captures) = env.captures(line) {
            let name = captures.get(1).map_or("", |name| name.as_str());
            let value = captures
                .get(2)
                .map_or("", |value| value.as_str())
                .trim_matches(['"', '\'']);
            if value.chars().count() >= MIN_ENV_VALUE && !placeholder(value) {
                findings.push(
                    Finding::new(
                        display,
                        Some(index + 1),
                        format!("`{name}` is assigned a long value in a committed .env file"),
                    )
                    .with_preview(redact(value)),
                );
            }
        }
    }
    Some(findings)
}

/// `scheme://user:password@` credentials in a URL (redaction only).
static URL_PASSWORD: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)\b[a-z][a-z0-9+.-]*://[^/\s:@'\x22]+:([^/\s@'\x22]+)@").ok());

/// Text with every high-signal secret of [`scan_text`] replaced by
/// `[redacted: <kind>]`, a long non-placeholder `*_SECRET`/`*_KEY`
/// assignment value on any line and a URL password replaced as well, and the
/// number of replacements. `None` when a pattern does not compile: callers
/// must then send nothing.
pub(crate) fn redact_secrets(text: &str) -> Option<(String, usize)> {
    let patterns = PATTERNS.as_ref()?;
    let env = ENV_ASSIGNMENT.as_ref()?;
    let url = URL_PASSWORD.as_ref()?;
    let mut count = 0usize;
    let mut output = String::with_capacity(text.len());
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            output.push('\n');
        }
        let mut line = line.to_string();
        for (kind, regex) in patterns {
            let found = regex.find_iter(&line).count();
            if found > 0 {
                count += found;
                line = regex
                    .replace_all(&line, format!("[redacted: {kind}]").as_str())
                    .into_owned();
            }
        }
        let found = url.captures_iter(&line).count();
        if found > 0 {
            count += found;
            line = url
                .replace_all(&line, |captures: &regex::Captures<'_>| {
                    let whole = captures.get(0).map_or("", |m| m.as_str());
                    let secret = captures.get(1).map_or("", |m| m.as_str());
                    whole.replacen(secret, "[redacted]", 1)
                })
                .into_owned();
        }
        if let Some(captures) = env.captures(&line)
            && let Some(value) = captures.get(2)
        {
            let bare = value.as_str().trim_matches(['"', '\'']);
            if bare.chars().count() >= MIN_ENV_VALUE && !placeholder(bare) {
                count += 1;
                line = format!("{}[redacted]", &line[..value.start()]);
            }
        }
        output.push_str(&line);
    }
    Some((output, count))
}

fn skipped_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "target")
}

pub(super) fn check(root: &Path, tracked: Tracked) -> Check {
    let paths = match tracked {
        Ok(paths) => paths,
        Err(reason) => return Check::not_checked(&SECRETS, reason),
    };
    if paths.len() > MAX_TRACKED_FILES {
        let reason = format!(
            "{} tracked files exceed the {MAX_TRACKED_FILES}-file bound; the scan is incomplete",
            paths.len()
        );
        return Check::with_status(
            &SECRETS,
            EvidenceStatus::Error(reason.clone()),
            Vec::new(),
            reason,
        );
    }
    let (mut scanned, mut skipped) = (0usize, 0usize);
    let mut findings = Vec::new();
    for path in paths {
        if skipped_path(&path) {
            continue;
        }
        // Symlinks, special files, binaries and files over the size bound
        // are not read.
        let Some(content) = read_bounded(&root.join(&path)) else {
            skipped += 1;
            continue;
        };
        if content.contains('\0') {
            skipped += 1;
            continue;
        }
        let display = path.to_string_lossy().replace('\\', "/");
        let Some(found) = scan_text(&display, &path, &content) else {
            let reason = "a secret pattern could not be compiled".to_string();
            return Check::with_status(
                &SECRETS,
                EvidenceStatus::Error(reason.clone()),
                Vec::new(),
                reason,
            );
        };
        scanned += 1;
        findings.extend(found);
    }
    let detail = format!(
        "{scanned} tracked text file(s) scanned; {skipped} skipped (binary, non-UTF-8, symlink or larger than {} MiB); `target/` excluded. Patterns: private-key headers, AWS AKIA key IDs, Stripe sk_live_, GitHub ghp_/github_pat_, Slack xoxb-/xoxp-, and long *_SECRET/*_KEY values in committed .env files. Untracked files and Git history are not scanned.",
        MAX_FILE_BYTES / (1024 * 1024)
    );
    Check::from_findings(&SECRETS, findings, detail)
}

#[cfg(test)]
#[path = "secrets_tests.rs"]
mod tests;
