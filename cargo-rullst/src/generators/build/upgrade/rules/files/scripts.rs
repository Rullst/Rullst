//! Scripts, workflows, Makefiles and justfiles that run CLI commands which
//! later releases removed or changed. Each predicate reads one line.

use super::super::catalog::{
    BLUEPRINTS_REMOVED, CLI_AI_REMOVED, CLI_SELF_UPDATE, DEPLOY_GENERATORS_REMOVED,
    DEPLOY_TARGETS_REMOVED, MIGRATION_AUTO_STATUS, OMNI_REMOVED,
};
use super::{Rule, line_of};

/// A predicate over one script line.
type LineCheck = fn(&str) -> bool;

/// Each script rule with the line predicate that locates it.
const CHECKS: [(&Rule, LineCheck); 7] = [
    (&OMNI_REMOVED, runs_omni),
    (&CLI_AI_REMOVED, runs_removed_cli_command),
    (&CLI_SELF_UPDATE, runs_removed_update_command),
    (&DEPLOY_TARGETS_REMOVED, runs_removed_deploy_target),
    (&DEPLOY_GENERATORS_REMOVED, runs_removed_generator),
    (&MIGRATION_AUTO_STATUS, runs_migration_auto),
    (&BLUEPRINTS_REMOVED, passes_removed_blueprint),
];

/// Each rule a script matches, at its first matching line.
pub(super) fn script_findings(text: &str) -> Vec<(&'static Rule, usize)> {
    CHECKS
        .into_iter()
        .filter_map(|(rule, matches)| line_of(text, matches).map(|line| (rule, line)))
        .collect()
}

/// A script line running the Omni shell commands removed in 13.0.0-alpha.3.
fn runs_omni(line: &str) -> bool {
    line.contains("rullst omni") || line.contains("make:omni")
}

/// A script line running a command that 13.0.0-alpha.4 removed (`rullst ai`)
/// or renamed (`rullst footprint`, now `green`), as a whole word.
pub(in super::super) fn runs_removed_cli_command(line: &str) -> bool {
    ["rullst footprint", "rullst ai"].iter().any(|command| {
        line.match_indices(command).any(|(index, _)| {
            line[index + command.len()..]
                .chars()
                .next()
                .is_none_or(|next| {
                    !(next.is_ascii_alphanumeric() || matches!(next, '-' | '_' | ':'))
                })
        })
    })
}

/// A script line running the self-updater removed in 13.0.0-alpha.4:
/// `rullst update check|verify|stage|install`, or `update guided` with its
/// CLI-installation flags.
pub(in super::super) fn runs_removed_update_command(line: &str) -> bool {
    ["check", "verify", "stage", "install"]
        .iter()
        .any(|command| line.contains(&format!("rullst update {command}")))
        || (line.contains("rullst update guided")
            && (line.contains("--scope") || line.contains("--root")))
}

/// Whether `line` passes `flag` as a whole word (`--nix`, not `--nixpkgs`).
fn has_flag(line: &str, flag: &str) -> bool {
    line.split(|character: char| character.is_whitespace() || character == '=')
        .any(|word| word == flag)
}

/// A script line running `deploy` with a platform target removed in
/// 13.0.0-alpha.4 (Fly.io, Railway or Render).
pub(in super::super) fn runs_removed_deploy_target(line: &str) -> bool {
    line.contains("rullst deploy")
        && ["fly", "railway", "render"].iter().any(|platform| {
            ["--platform ", "--platform=", "-p "]
                .iter()
                .any(|option| line.contains(&format!("{option}{platform}")))
        })
}

/// A script line running a generator removed in 13.0.0-alpha.4: Foundry,
/// `nixify`, Buildah, `make:grpc`, or `new --nix|--buildah`.
pub(in super::super) fn runs_removed_generator(line: &str) -> bool {
    [
        "rullst foundry:",
        "rullst nixify",
        "generate:buildah",
        "make:grpc",
    ]
    .iter()
    .any(|command| line.contains(command))
        || (line.contains("rullst new") && (has_flag(line, "--nix") || has_flag(line, "--buildah")))
}

/// A script line running `make:migration:auto`, which exits with status 1
/// since 13.0.0-alpha.4 when there is no SQLite database to compare.
pub(in super::super) fn runs_migration_auto(line: &str) -> bool {
    line.contains("make:migration:auto")
}

/// A script line passing a `--blueprint` that 13.0.0-alpha.4 moved to the
/// repository's examples (`lms`, `blog`, `portfolio`, `erp`), also as
/// `--blueprint=lms` or quoted, and on a continuation line of `new`.
pub(in super::super) fn passes_removed_blueprint(line: &str) -> bool {
    let words: Vec<&str> = line
        .split(|character: char| character.is_whitespace() || character == '=')
        .filter(|word| !word.is_empty())
        .collect();
    words.windows(2).any(|pair| {
        pair[0] == "--blueprint"
            && matches!(
                pair[1].trim_matches(['"', '\'']),
                "lms" | "blog" | "portfolio" | "erp"
            )
    })
}
