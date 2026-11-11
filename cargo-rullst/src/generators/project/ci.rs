//! The GitHub files `cargo rullst new` writes unless `--no-ci` is given:
//! a CI workflow (formatting, strict Clippy, tests, a daily `cargo audit` and
//! a TruffleHog secret scan) and a monthly, grouped Dependabot configuration.
//! The files are static so they can be linted and pin-checked in this
//! repository exactly as they are written.

use std::fs;
use std::io;
use std::path::Path;

/// Each generated file and its content.
pub(crate) const FILES: [(&str, &str); 2] = [
    (
        ".github/workflows/ci.yml",
        include_str!("ci/.github/workflows/ci.yml"),
    ),
    (".github/dependabot.yml", include_str!("ci/dependabot.yml")),
];

/// Writes [`FILES`] below `root`, creating `.github/` as needed.
pub(crate) fn write(root: &Path) -> io::Result<()> {
    for (relative, content) in FILES {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
    }
    Ok(())
}

/// Every `uses:` value of a workflow with the comment after it, for the
/// pin checks of generated workflows.
#[cfg(test)]
pub(crate) fn action_references(workflow: &str) -> Vec<(String, String)> {
    workflow
        .lines()
        .filter_map(|line| {
            let line = line.trim_start().trim_start_matches("- ").trim_start();
            let rest = line.strip_prefix("uses:")?;
            let (value, comment) = rest.split_once(" # ").unwrap_or((rest, ""));
            Some((value.trim().to_owned(), comment.trim().to_owned()))
        })
        .collect()
}

/// Asserts that every action of `workflow` is pinned to a full commit SHA
/// with a version comment, and that each pin is one Rullst's own workflows
/// use when this repository's `.github/workflows` is present.
#[cfg(test)]
pub(crate) fn assert_pinned_like_rullst(name: &str, workflow: &str) {
    let references = action_references(workflow);
    assert!(!references.is_empty(), "{name}: no actions");
    assert_eq!(
        references.len(),
        workflow.matches("uses:").count(),
        "{name}: every `uses:` must be a plain `uses: owner/repo@sha # version` line"
    );
    let own = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows");
    let own = fs::read_dir(&own).ok().map(|entries| {
        entries
            .filter_map(Result::ok)
            .filter_map(|entry| fs::read_to_string(entry.path()).ok())
            .flat_map(|source| action_references(&source))
            .collect::<Vec<_>>()
    });
    for (value, comment) in references {
        let (action, sha) = value
            .split_once('@')
            .unwrap_or_else(|| panic!("{name}: `{value}` names no ref"));
        assert!(
            action.split('/').count() >= 2 && !action.starts_with('.'),
            "{name}: `{value}` is not a remote action"
        );
        assert!(
            sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "{name}: `{value}` is not pinned to a full commit SHA"
        );
        assert!(
            !comment.is_empty(),
            "{name}: `{value}` has no version comment"
        );
        if let Some(own) = &own {
            // When a Rullst workflow bumps a pin, update the generated one too.
            assert!(
                own.iter()
                    .any(|(used, used_comment)| *used == value && *used_comment == comment),
                "{name}: `{value} # {comment}` is not a pin of Rullst's own .github/workflows"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_workflow_pins_rullsts_own_actions_with_minimal_permissions() {
        let (_, workflow) = FILES[0];
        assert_pinned_like_rullst("ci.yml", workflow);
        let actions = action_references(workflow)
            .into_iter()
            .map(|(value, _)| value.split_once('@').map(|(action, _)| action.to_owned()))
            .collect::<Vec<_>>();
        for expected in [
            "actions/checkout",
            "actions-rust-lang/setup-rust-toolchain",
            "taiki-e/install-action",
            "trufflesecurity/trufflehog",
        ] {
            assert!(actions.contains(&Some(expected.to_owned())), "{expected}");
        }
        // Read-only token at the top; no job asks for more.
        assert!(workflow.contains("\npermissions:\n  contents: read\n"));
        assert_eq!(workflow.matches("permissions:").count(), 1);
        assert_eq!(
            workflow.matches("persist-credentials: false").count(),
            workflow.matches("uses: actions/checkout@").count()
        );
        for step in [
            "run: cargo fmt --all -- --check",
            "run: cargo clippy --all-targets -- -D warnings",
            "run: cargo test --all-targets",
            "run: cargo audit",
            "- cron: \"17 6 * * *\"",
            "fetch-depth: 0",
            "extra_args: --only-verified",
        ] {
            assert!(workflow.contains(step), "{step}");
        }
        // Only the advisory audit runs on the daily schedule.
        assert_eq!(
            workflow
                .matches("    if: github.event_name != 'schedule'\n")
                .count(),
            3
        );
        assert!(!workflow.contains('\t') && !workflow.contains('\r'));
    }

    #[test]
    fn dependabot_updates_both_ecosystems_monthly_in_groups() {
        let (path, config) = FILES[1];
        assert_eq!(path, ".github/dependabot.yml");
        assert!(config.contains("\nversion: 2\n"));
        for ecosystem in ["cargo", "github-actions"] {
            let entry = format!(
                "  - package-ecosystem: \"{ecosystem}\"\n    directory: \"/\"\n    schedule:\n      interval: \"monthly\"\n    open-pull-requests-limit: 5\n    groups:\n      {ecosystem}:\n        patterns:\n          - \"*\"\n"
            );
            assert!(config.contains(&entry), "{ecosystem}");
        }
        assert_eq!(config.matches("package-ecosystem").count(), 2);
    }

    #[test]
    fn writes_both_files_below_github() {
        let root = tempfile::tempdir().expect("temporary project");
        write(root.path()).expect("CI files");
        for (relative, content) in FILES {
            assert_eq!(
                fs::read_to_string(root.path().join(relative)).expect("written file"),
                content
            );
        }
        // Writing again replaces them identically.
        write(root.path()).expect("rewrite");
    }

    #[test]
    fn references_are_read_from_plain_and_list_lines() {
        let workflow =
            "steps:\n  - uses: a/b@c # v1\n    uses: d/e@f\n  - name: x\n    run: echo uses\n";
        assert_eq!(
            action_references(workflow),
            [
                ("a/b@c".to_owned(), "v1".to_owned()),
                ("d/e@f".to_owned(), String::new())
            ]
        );
    }
}
