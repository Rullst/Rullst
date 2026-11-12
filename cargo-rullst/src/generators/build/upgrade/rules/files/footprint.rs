//! `V13-CLI-GREEN-RENAMED`: `cargo rullst green` became `cargo rullst
//! footprint`, and its `rullst.green.v1` report `rullst.footprint.v1`, in
//! 13.0.0-alpha.5, without an alias. Scripts, CI configuration, container
//! files, Makefiles and justfiles and documentation that still run the old
//! command or read the old schema are found by line, up to four directories
//! deep; Rust sources and build output are not read.

use super::super::Rule;
use super::super::catalog::CLI_GREEN_RENAMED;
use super::super::reviewed::first_unreviewed;
use super::read_small;
use walkdir::WalkDir;

/// Directory entries visited at most.
const MAX_ENTRIES: usize = 5_000;
/// File names read whatever their extension.
const NAMES: &[&str] = &[
    "Makefile",
    "makefile",
    "GNUmakefile",
    "justfile",
    "Justfile",
    ".justfile",
    "Dockerfile",
    "Containerfile",
    "Jenkinsfile",
    "Procfile",
];
/// Extensions of scripts, CI and container configuration and documentation.
const EXTENSIONS: &[&str] = &[
    "sh", "bash", "zsh", "fish", "ps1", "psm1", "bat", "cmd", "py", "rb", "js", "mjs", "cjs", "ts",
    "mk", "yml", "yaml", "toml", "json", "nix", "md", "markdown", "mdx", "rst", "adoc", "txt",
];
/// Directories never read: build output, dependencies and version control.
const SKIPPED: &[&str] = &["target", ".git", "node_modules", "vendor", ".rullst"];

/// Whether `line` runs the old command (`cargo rullst green`, `rullst green`)
/// as a whole word or names the old report schema.
pub(in super::super) fn mentions_green(line: &str) -> bool {
    const COMMAND: &str = "rullst green";
    line.contains("rullst.green.v1")
        || line.match_indices(COMMAND).any(|(index, _)| {
            let before = line[..index].chars().next_back();
            let after = line[index + COMMAND.len()..].chars().next();
            before.is_none_or(|character| !(character.is_ascii_alphanumeric() || character == '_'))
                && after.is_none_or(|character| {
                    !(character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':'))
                })
        })
}

/// Whether the file called `name` can hold such a line.
fn readable(name: &str, in_workflows: bool) -> bool {
    if in_workflows || NAMES.contains(&name) {
        return true;
    }
    if ["Dockerfile.", "Containerfile."]
        .iter()
        .any(|prefix| name.starts_with(prefix))
    {
        return true;
    }
    name.rsplit_once('.').is_some_and(|(stem, extension)| {
        !stem.is_empty()
            && (EXTENSIONS.contains(&extension)
                || extension.eq_ignore_ascii_case("dockerfile")
                || extension == "Containerfile")
    })
}

/// The relative path and first unmarked line of each file that mentions the
/// old command; covered lines add the rule to `reviewed`.
pub(in super::super) fn green_mentions(
    root: &std::path::Path,
    reviewed: &mut Vec<&'static Rule>,
) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    for entry in WalkDir::new(root)
        .max_depth(4)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| SKIPPED.contains(&name))
        })
        .filter_map(Result::ok)
        .take(MAX_ENTRIES)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let name = entry.file_name().to_string_lossy();
        if !readable(&name, relative.starts_with(".github/workflows/")) {
            continue;
        }
        let Some(text) = read_small(entry.path()) else {
            continue;
        };
        if !text.contains("green") {
            continue;
        }
        let lines: Vec<&str> = text.lines().collect();
        if let Some(line) = first_unreviewed(&lines, &CLI_GREEN_RENAMED, mentions_green, reviewed) {
            found.push((relative, line));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_command_and_schema_are_found_as_whole_words() {
        for line in [
            "cargo rullst green --bench --json",
            "run: cargo rullst green",
            "`rullst green`",
            "RUN cargo-rullst green --path /health",
            "jq -e '.schema == \"rullst.green.v1\"' report.json",
            "npm run footprint # was: rullst green;",
        ] {
            assert!(mentions_green(line), "{line}");
        }
        for line in [
            "cargo rullst footprint --bench",
            "rullst.footprint.v1",
            "cargo rullst greenhouse",
            "rullst green-report",
            "myrullst green",
            "rullst green_x",
            "rullst green:x",
            "Rullst green software",
            "the green software roadmap",
        ] {
            assert!(!mentions_green(line), "{line}");
        }
    }

    #[test]
    fn scripts_ci_container_files_and_docs_are_read() {
        for (name, workflow) in [
            ("Makefile", false),
            ("justfile", false),
            ("Dockerfile", false),
            ("Dockerfile.prod", false),
            ("app.Dockerfile", false),
            ("Containerfile", false),
            ("ci.yml", false),
            ("release", true),
            ("measure.sh", false),
            ("README.md", false),
            ("package.json", false),
            ("Makefile.toml", false),
            ("deploy.ps1", false),
        ] {
            assert!(readable(name, workflow), "{name}");
        }
        for name in [
            "main.rs",
            "logo.png",
            "Cargo.lock",
            ".md",
            "notes",
            "app.Containerfile.bak",
        ] {
            assert!(!readable(name, false), "{name}");
        }
    }
}
