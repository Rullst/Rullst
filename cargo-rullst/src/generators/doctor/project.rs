//! Project, configuration and security-baseline checks. Secret values are
//! only measured (present, placeholder, length), never displayed.

use super::context::{DotEnv, ProjectContext, Vars, lookup};
use super::probe;
use super::report::Check;
use crate::ui::home::{display_safe, rullst_requirement};
use std::path::Path;
use std::str::FromStr;

const APP_KEY_MIN: usize = 32;

/// The project group outside a project.
pub(crate) fn outside_checks() -> Vec<Check> {
    vec![
        Check::info(
            "project.detected",
            "Rullst project",
            "none in this directory or its parents; project checks were skipped",
        )
        .with_fix("cargo rullst new <name>"),
    ]
}

/// The first number of a version requirement such as `^13.0.0-alpha.1`.
pub(crate) fn requirement_major(requirement: &str) -> Option<u64> {
    let first = requirement.split(',').next()?.trim();
    let digits: String = first
        .trim_start_matches(|character: char| !character.is_ascii_digit())
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

pub(crate) fn version_check(requirement: Option<&str>, cli_version: &str) -> Check {
    const ID: &str = "project.rullst_version";
    const TITLE: &str = "Rullst version";
    let cli_major = semver::Version::parse(cli_version)
        .ok()
        .map(|version| version.major);
    let Some(requirement) = requirement else {
        return Check::info(ID, TITLE, "the rullst requirement could not be read");
    };
    if matches!(requirement, "path" | "git" | "workspace") {
        return Check::info(
            ID,
            TITLE,
            format!("{requirement} dependency (version not compared)"),
        );
    }
    match (requirement_major(requirement), cli_major) {
        (Some(project), Some(cli)) if project == cli => Check::pass(
            ID,
            TITLE,
            format!("rullst {requirement} matches this CLI ({cli_version})"),
        ),
        (Some(project), Some(cli)) if project < cli => Check::warn(
            ID,
            TITLE,
            format!("project uses rullst {requirement}; this CLI is {cli_version}"),
            "cargo rullst upgrade --dry-run, then cargo rullst upgrade",
        ),
        (Some(_), Some(_)) => Check::warn(
            ID,
            TITLE,
            format!("project uses rullst {requirement}, newer than this CLI ({cli_version})"),
            "Install the matching CLI: cargo install cargo-rullst --locked",
        ),
        _ => Check::info(
            ID,
            TITLE,
            format!("requirement {requirement} (not compared)"),
        ),
    }
}

pub(crate) fn project_checks(project: &ProjectContext) -> Vec<Check> {
    let location = project
        .relative_root
        .as_ref()
        .map(|root| format!(" (root {root})"))
        .unwrap_or_default();
    vec![
        Check::pass(
            "project.detected",
            "Rullst project",
            format!("{}{location}", project.name),
        ),
        version_check(
            rullst_requirement(&project.manifest).as_deref(),
            env!("CARGO_PKG_VERSION"),
        ),
    ]
}

fn env_file_check(project: &ProjectContext) -> Check {
    const ID: &str = "config.env_file";
    const TITLE: &str = ".env file";
    match (&project.dotenv, &project.example) {
        (DotEnv::Parsed(values), _) => Check::pass(ID, TITLE, format!("{} keys", values.len())),
        (DotEnv::Invalid, _) => Check::fail(
            ID,
            TITLE,
            ".env could not be read or parsed",
            "Fix .env so every line is KEY=value (quotes allowed)",
        ),
        (DotEnv::Missing, DotEnv::Missing) => Check::info(
            ID,
            TITLE,
            "no .env; configuration comes from the process environment",
        ),
        (DotEnv::Missing, _) => Check::warn(
            ID,
            TITLE,
            ".env is missing but .env.example exists",
            "cp .env.example .env, then fill in the values",
        ),
    }
}

/// Keys listed in `.env.example` but set neither in `.env` nor the environment.
pub(crate) fn missing_keys(project: &ProjectContext, vars: &impl Vars) -> Option<Vec<String>> {
    let DotEnv::Parsed(example) = &project.example else {
        return None;
    };
    let mut missing: Vec<String> = example
        .keys()
        .filter(|key| lookup(project, vars, key).is_none())
        .map(|key| display_safe(key))
        .collect();
    missing.sort();
    Some(missing)
}

fn env_keys_check(project: &ProjectContext, vars: &impl Vars) -> Option<Check> {
    const ID: &str = "config.env_keys";
    const TITLE: &str = "Keys from .env.example";
    if project.dotenv == DotEnv::Invalid {
        return None;
    }
    let missing = missing_keys(project, vars)?;
    Some(if missing.is_empty() {
        Check::pass(ID, TITLE, "every key is set")
    } else {
        let shown: Vec<&str> = missing.iter().take(8).map(String::as_str).collect();
        let more = missing.len().saturating_sub(shown.len());
        let suffix = if more > 0 {
            format!(" (+{more} more)")
        } else {
            String::new()
        };
        Check::warn(
            ID,
            TITLE,
            format!("missing: {}{suffix}", shown.join(", ")),
            "Add the missing keys to .env (see .env.example for their meaning)",
        )
    })
}

fn environment_check(project: &ProjectContext, vars: &impl Vars) -> Check {
    const ID: &str = "config.environment";
    const TITLE: &str = "Environment";
    let value = lookup(project, vars, "RULLST_ENV")
        .map(|(value, source)| ("RULLST_ENV", value, source))
        .or_else(|| {
            lookup(project, vars, "APP_ENV").map(|(value, source)| ("APP_ENV", value, source))
        });
    let Some((name, value, source)) = value else {
        return Check::info(
            ID,
            TITLE,
            "RULLST_ENV not set; [app].env or the default applies",
        );
    };
    match rullst_core::config::Environment::from_str(&value) {
        Ok(environment) => Check::pass(
            ID,
            TITLE,
            format!("{environment} ({name} from {})", source.label()),
        ),
        Err(_) => Check::fail(
            ID,
            TITLE,
            format!(
                "{name}={} from {} is not a known environment",
                display_safe(&value),
                source.label()
            ),
            "Use development, test, staging or production",
        ),
    }
}

fn rullst_toml_check(contents: &str) -> Check {
    const ID: &str = "config.rullst_toml";
    const TITLE: &str = "Rullst.toml";
    match rullst_core::config::RullstConfig::from_toml(contents) {
        Ok(_) => Check::pass(ID, TITLE, "valid"),
        Err(error) => {
            let message = crate::ui::error_report::sanitize(&error.to_string());
            let first = message.lines().next().unwrap_or_default().to_string();
            Check::fail(
                ID,
                TITLE,
                format!("invalid: {first}"),
                "Fix Rullst.toml at the reported line (see the configuration reference)",
            )
        }
    }
}

pub(crate) fn config_checks(project: &ProjectContext, vars: &impl Vars) -> Vec<Check> {
    let mut checks = vec![env_file_check(project)];
    checks.extend(env_keys_check(project, vars));
    checks.push(environment_check(project, vars));
    if let Some(contents) = &project.rullst_toml {
        checks.push(rullst_toml_check(contents));
    }
    checks
}

/// Whether `value` looks like a template placeholder rather than a secret.
fn placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "replace",
        "change",
        "your_",
        "your-",
        "example",
        "placeholder",
        "xxxx",
        "secret",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

pub(crate) fn app_key_check(project: &ProjectContext, vars: &impl Vars) -> Option<Check> {
    const ID: &str = "security.app_key";
    const TITLE: &str = "APP_KEY";
    const FIX: &str =
        "Set APP_KEY in .env to 32+ random characters (for example `openssl rand -hex 32`)";
    let Some((value, source)) = lookup(project, vars, "APP_KEY") else {
        return project
            .example
            .get("APP_KEY")
            .map(|_| Check::warn(ID, TITLE, "not set", FIX));
    };
    let value = value.trim();
    Some(if value.is_empty() {
        Check::warn(ID, TITLE, format!("empty in {}", source.label()), FIX)
    } else if placeholder(value) {
        Check::warn(
            ID,
            TITLE,
            format!("still a placeholder in {}", source.label()),
            FIX,
        )
    } else if value.chars().count() < APP_KEY_MIN {
        Check::warn(
            ID,
            TITLE,
            format!(
                "shorter than {APP_KEY_MIN} characters in {}",
                source.label()
            ),
            FIX,
        )
    } else {
        Check::pass(
            ID,
            TITLE,
            format!("set in {} ({APP_KEY_MIN}+ characters)", source.label()),
        )
    })
}

/// `.env` must never be committed; checked only when it exists and Git works.
pub(crate) fn env_git_check(root: &Path, git_available: bool) -> Option<Check> {
    const ID: &str = "security.env_git";
    const TITLE: &str = ".env in Git";
    if !git_available || !root.join(".env").is_file() {
        return None;
    }
    let root_text = root.to_string_lossy().into_owned();
    let root_text = root_text.as_str();
    let tracked = probe::run(
        "git",
        &["-C", root_text, "ls-files", "--error-unmatch", "--", ".env"],
    );
    if tracked.ok() {
        return Some(Check::fail(
            ID,
            TITLE,
            ".env is tracked by Git; its secrets are in the history",
            "git rm --cached .env, keep .env in .gitignore and rotate the exposed secrets",
        ));
    }
    let ignored = std::process::Command::new("git")
        .args(["-C", root_text, "check-ignore", "-q", ".env"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()
        .and_then(|status| status.code());
    Some(match ignored {
        Some(0) => Check::pass(ID, TITLE, "ignored and untracked"),
        Some(1) => Check::warn(
            ID,
            TITLE,
            ".env is not ignored by Git",
            "Add .env to .gitignore",
        ),
        _ => Check::info(ID, TITLE, "not a Git repository"),
    })
}

pub(crate) fn lockfile_check(root: &Path) -> Check {
    let found = root
        .ancestors()
        .take(8)
        .any(|directory| directory.join("Cargo.lock").is_file());
    if found {
        Check::pass("security.lockfile", "Cargo.lock", "present")
    } else {
        Check::warn(
            "security.lockfile",
            "Cargo.lock",
            "missing; dependency versions are not pinned",
            "cargo generate-lockfile, then commit Cargo.lock",
        )
    }
}
