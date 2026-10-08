//! Collects the change set for `cargo rullst ai review`.
//!
//! Git produces the diff without external diff drivers or text conversion
//! filters, so no repository-configured program runs. Each file is then
//! judged on its own: protected paths (the assistant's path policy: `.env*`,
//! keys and certificate stores, credentials, databases, `Cargo.lock`, `.git/`,
//! `target/`), binary files and untracked files that are not regular text
//! files are omitted and listed; the rest have high-signal secrets redacted
//! (the audit report's patterns) and every added or context line numbered.

use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Most bytes of `git diff` output read.
const MAX_GIT_OUTPUT: usize = 8 * 1024 * 1024;
/// Most untracked files considered.
const MAX_UNTRACKED: usize = 200;
/// Largest untracked file read.
const MAX_UNTRACKED_BYTES: u64 = 256 * 1024;

/// Which changes are reviewed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    /// Staged and unstaged changes against `HEAD`.
    WorkingTree,
    /// Staged changes only.
    Staged,
    /// `git diff <base>...HEAD`.
    Base(String),
}

/// One file's numbered, redacted diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileDiff {
    pub path: String,
    pub text: String,
}

/// A file that is never sent, and why.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct Omitted {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Default)]
pub(crate) struct Collected {
    pub files: Vec<FileDiff>,
    pub omitted: Vec<Omitted>,
    pub redactions: usize,
    /// The git output exceeded [`MAX_GIT_OUTPUT`] and its end was not read.
    pub output_cut: bool,
}

fn git_command(git: &OsStr, root: &Path) -> Command {
    let mut command = Command::new(git);
    command
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_EXTERNAL_DIFF")
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Runs git and returns at most [`MAX_GIT_OUTPUT`] bytes of its output and
/// whether more existed.
fn run(git: &OsStr, root: &Path, args: &[&str]) -> Result<(Vec<u8>, bool), String> {
    // A repository-configured fsmonitor hook would run a program.
    let mut child = git_command(git, root)
        .args(["-c", "core.fsmonitor=false"])
        .args(args)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| "git is not available".to_string())?;
    let mut output = Vec::new();
    let read = child
        .stdout
        .take()
        .map(|stdout| {
            stdout
                .take(MAX_GIT_OUTPUT as u64 + 1)
                .read_to_end(&mut output)
        })
        .transpose()
        .map_err(|_| "the git output could not be read".to_string())?;
    let cut = output.len() > MAX_GIT_OUTPUT;
    if cut {
        output.truncate(MAX_GIT_OUTPUT);
        let _ = child.kill();
    }
    let status = child.wait().map_err(|_| "git did not finish".to_string())?;
    if read.is_none() || (!cut && !status.success()) {
        return Err(format!("`git {}` failed", args.join(" ")));
    }
    Ok((output, cut))
}

/// The top level of the work tree containing `cwd`.
pub(crate) fn work_tree(git: &OsStr, cwd: &Path) -> Result<PathBuf, String> {
    let (output, _) = run(git, cwd, &["rev-parse", "--show-toplevel"]).map_err(|_| {
        "`cargo rullst ai review` needs a git work tree; run it inside one".to_string()
    })?;
    let top =
        String::from_utf8(output).map_err(|_| "the work tree path is not UTF-8".to_string())?;
    let top = top.trim();
    if top.is_empty() {
        return Err(
            "`cargo rullst ai review` needs a git work tree; run it inside one".to_string(),
        );
    }
    Ok(PathBuf::from(top))
}

/// A revision a user may name with `--base`: no option-like or spaced text.
pub(crate) fn valid_base(base: &str) -> bool {
    !base.is_empty()
        && base.len() <= 200
        && !base.starts_with('-')
        && !base.contains("..")
        && base.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '.' | '~' | '^' | '@')
        })
}

const DIFF_ARGS: [&str; 8] = [
    "-c",
    "core.quotePath=false",
    "diff",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "--find-renames",
    "--unified=3",
];

fn diff(git: &OsStr, root: &Path, extra: &[&str]) -> Result<(String, bool), String> {
    let mut args: Vec<&str> = DIFF_ARGS.to_vec();
    args.extend_from_slice(extra);
    args.push("--");
    let (output, cut) = run(git, root, &args)?;
    Ok((String::from_utf8_lossy(&output).into_owned(), cut))
}

fn has_head(git: &OsStr, root: &Path) -> bool {
    run(git, root, &["rev-parse", "--verify", "--quiet", "HEAD"]).is_ok()
}

/// The raw diff text for `scope`.
fn raw_diff(git: &OsStr, root: &Path, scope: &Scope) -> Result<(String, bool), String> {
    match scope {
        Scope::Staged => diff(git, root, &["--cached"]),
        Scope::Base(base) => {
            if !valid_base(base) {
                return Err("`--base` takes a branch, tag or commit name".to_string());
            }
            let commit = format!("{base}^{{commit}}");
            run(git, root, &["rev-parse", "--verify", "--quiet", &commit])
                .map_err(|_| format!("`{base}` is not a commit in this repository"))?;
            diff(git, root, &[&format!("{base}...HEAD")])
        }
        Scope::WorkingTree if has_head(git, root) => diff(git, root, &["HEAD"]),
        Scope::WorkingTree => {
            // No commit yet: staged files, then unstaged changes to them.
            let (mut staged, cut) = diff(git, root, &["--cached"])?;
            if cut {
                return Ok((staged, cut));
            }
            let (unstaged, cut) = diff(git, root, &[])?;
            staged.push_str(&unstaged);
            Ok((staged, cut))
        }
    }
}

/// Splits a diff into its `diff --git` sections.
pub(crate) fn sections(diff: &str) -> Vec<&str> {
    let mut starts: Vec<usize> = diff
        .match_indices("diff --git ")
        .filter(|(index, _)| *index == 0 || diff.as_bytes()[index - 1] == b'\n')
        .map(|(index, _)| index)
        .collect();
    starts.push(diff.len());
    starts
        .windows(2)
        .map(|pair| &diff[pair[0]..pair[1]])
        .collect()
}

fn unquote(path: &str) -> &str {
    path.strip_prefix('"')
        .and_then(|path| path.strip_suffix('"'))
        .unwrap_or(path)
}

/// The paths a section touches (new path first; a rename adds the old one).
pub(crate) fn section_paths(section: &str) -> Vec<String> {
    let mut new = None;
    let mut old = None;
    for line in section.lines() {
        if line.starts_with("@@") {
            break;
        }
        if let Some(path) = line.strip_prefix("+++ ") {
            let path = unquote(path);
            new = Some(path.strip_prefix("b/").unwrap_or(path).to_string());
        } else if let Some(path) = line.strip_prefix("--- ") {
            let path = unquote(path);
            old = Some(path.strip_prefix("a/").unwrap_or(path).to_string());
        } else if let Some(path) = line.strip_prefix("rename to ") {
            new = Some(unquote(path).to_string());
        } else if let Some(path) = line.strip_prefix("rename from ") {
            old = Some(unquote(path).to_string());
        }
    }
    if new.is_none() && old.is_none() {
        // Binary or mode-only change: `diff --git a/<path> b/<path>`.
        let header = section.lines().next().unwrap_or("");
        if let Some((_, path)) = header.rsplit_once(" b/") {
            new = Some(unquote(path).to_string());
        }
    }
    let mut paths: Vec<String> = [new, old]
        .into_iter()
        .flatten()
        .filter(|path| path != "/dev/null")
        .collect();
    paths.dedup();
    paths
}

fn binary(section: &str) -> bool {
    section.lines().any(|line| {
        line == "GIT binary patch"
            || (line.starts_with("Binary files ") && line.ends_with(" differ"))
    })
}

/// Numbers added and context lines with their new line number and redacts
/// secrets in every line. `None` when the redaction patterns are unusable.
pub(crate) fn annotate(section: &str, redactions: &mut usize) -> Option<String> {
    let mut output = String::with_capacity(section.len() + section.len() / 4);
    let mut line_number: Option<u64> = None;
    for line in section.lines() {
        let (prefix, rest) = match line.chars().next() {
            Some(first @ ('+' | '-' | ' ')) if line_number.is_some() => (Some(first), &line[1..]),
            _ => (None, line),
        };
        let (rest, found) = crate::generators::audit_report::redact_secrets(rest)?;
        *redactions += found;
        match (prefix, line_number.as_mut()) {
            (Some('-'), _) => output.push_str(&format!("-     | {rest}")),
            (Some(first), Some(number)) => {
                output.push_str(&format!("{first}{number:>5}| {rest}"));
                *number += 1;
            }
            _ => {
                if let Some(start) = hunk_start(&rest) {
                    line_number = Some(start);
                }
                output.push_str(&rest);
            }
        }
        output.push('\n');
    }
    Some(output)
}

/// The new-file start line of a `@@ -a,b +c,d @@` header.
fn hunk_start(line: &str) -> Option<u64> {
    let rest = line.strip_prefix("@@ ")?;
    let new = rest.split_whitespace().find(|part| part.starts_with('+'))?;
    let start = new.trim_start_matches('+').split(',').next()?;
    start.parse().ok()
}

fn omit(collected: &mut Collected, path: &str, reason: impl Into<String>) {
    if !collected.omitted.iter().any(|omitted| omitted.path == path) {
        collected.omitted.push(Omitted {
            path: path.to_string(),
            reason: reason.into(),
        });
    }
}

fn protected_reason(path: &str) -> Option<String> {
    super::super::paths::protected(path).map(|error| match error {
        super::super::paths::PathError::Denied(_) => {
            "protected path (secrets, keys, certificates, databases or managed files)".to_string()
        }
        _ => "unusual path name".to_string(),
    })
}

/// A synthetic "new file" diff for an untracked text file.
fn untracked_section(root: &Path, path: &str) -> Result<String, &'static str> {
    let absolute = root.join(path);
    let metadata = std::fs::symlink_metadata(&absolute).map_err(|_| "unreadable")?;
    if !metadata.is_file() {
        return Err("not a regular file");
    }
    if metadata.len() > MAX_UNTRACKED_BYTES {
        return Err("larger than 256 KiB");
    }
    let bytes = std::fs::read(&absolute).map_err(|_| "unreadable")?;
    let text = String::from_utf8(bytes).map_err(|_| "binary")?;
    if text.contains('\0') {
        return Err("binary");
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut section = format!(
        "diff --git a/{path} b/{path}\nnew file (untracked)\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1,{} @@\n",
        lines.len()
    );
    for line in lines {
        section.push('+');
        section.push_str(line);
        section.push('\n');
    }
    Ok(section)
}

fn add_section(collected: &mut Collected, section: &str) -> Result<(), String> {
    let paths = section_paths(section);
    let Some(path) = paths.first().cloned() else {
        return Ok(());
    };
    if let Some(reason) = paths.iter().find_map(|path| protected_reason(path)) {
        omit(collected, &path, reason);
        return Ok(());
    }
    if binary(section) {
        omit(collected, &path, "binary file");
        return Ok(());
    }
    let text = annotate(section, &mut collected.redactions)
        .ok_or_else(|| "the secret redaction patterns could not be compiled".to_string())?;
    collected.files.push(FileDiff { path, text });
    Ok(())
}

/// Collects the change set of the work tree at `root`.
pub(crate) fn collect(
    git: &OsStr,
    root: &Path,
    scope: &Scope,
    include_untracked: bool,
) -> Result<Collected, String> {
    let (text, cut) = raw_diff(git, root, scope)?;
    let mut collected = Collected {
        output_cut: cut,
        ..Collected::default()
    };
    for section in sections(&text) {
        add_section(&mut collected, section)?;
    }
    if include_untracked && *scope == Scope::WorkingTree {
        let (output, _) = run(
            git,
            root,
            &["ls-files", "--others", "--exclude-standard", "-z"],
        )?;
        let paths: Vec<String> = output
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
            .map(|path| String::from_utf8_lossy(path).into_owned())
            .collect();
        for (index, path) in paths.iter().enumerate() {
            if index >= MAX_UNTRACKED {
                omit(&mut collected, path, "over the 200 untracked file limit");
                continue;
            }
            if let Some(reason) = protected_reason(path) {
                omit(&mut collected, path, reason);
                continue;
            }
            match untracked_section(root, path) {
                Ok(section) => add_section(&mut collected, &section)?,
                Err(reason) => omit(&mut collected, path, reason),
            }
        }
    }
    Ok(collected)
}
