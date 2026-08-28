use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

pub(super) const RULE_CATALOG_VERSION: &str = "rullst-upgrade-rules-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum FindingSeverity {
    Blocker,
    Review,
}

impl std::fmt::Display for FindingSeverity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blocker => formatter.write_str("BLOCKER"),
            Self::Review => formatter.write_str("REVIEW"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct SourceFinding {
    pub path: PathBuf,
    pub line: usize,
    pub code: &'static str,
    pub severity: FindingSeverity,
    pub message: &'static str,
}

struct Rule {
    needle: &'static str,
    code: &'static str,
    severity: FindingSeverity,
    message: &'static str,
    source_major_at_most: u64,
    target_major_at_least: u64,
}

const RULES: &[Rule] = &[
    Rule {
        needle: "#[routes]",
        code: "V5-ROUTES-ATTRIBUTE",
        severity: FindingSeverity::Blocker,
        message: "replace attribute-style registration with the explicit routes! macro",
        source_major_at_most: 5,
        target_major_at_least: 12,
    },
    Rule {
        needle: "Server::new()",
        code: "V5-SERVER-CONSTRUCTOR",
        severity: FindingSeverity::Blocker,
        message: "v12 Server::new requires an explicit Router",
        source_major_at_most: 5,
        target_major_at_least: 12,
    },
    Rule {
        needle: ".run()",
        code: "V5-SERVER-PORT",
        severity: FindingSeverity::Blocker,
        message: "v12 Server::run requires an explicit port",
        source_major_at_most: 5,
        target_major_at_least: 12,
    },
    Rule {
        needle: "Nexus::build(",
        code: "V5-NEXUS-POLICY",
        severity: FindingSeverity::Review,
        message: "rebuild Nexus with try_build and an explicit fail-closed access policy",
        source_major_at_most: 11,
        target_major_at_least: 12,
    },
    Rule {
        needle: "STUDIO_PASSWORD",
        code: "V5-STUDIO-BOUNDARY",
        severity: FindingSeverity::Review,
        message: "STUDIO_PASSWORD is not built-in network authentication; keep Studio debug-only and loopback-bound",
        source_major_at_most: 11,
        target_major_at_least: 12,
    },
    Rule {
        needle: "APP_ENV",
        code: "V5-ENV-ALIAS",
        severity: FindingSeverity::Review,
        message: "APP_ENV remains a legacy alias; migrate deployments to validated RULLST_ENV",
        source_major_at_most: 11,
        target_major_at_least: 12,
    },
];

pub(super) fn scan_workspace(
    package_roots: &[PathBuf],
    source_majors: &std::collections::BTreeSet<u64>,
    target_major: u64,
) -> Result<Vec<SourceFinding>, Box<dyn std::error::Error>> {
    let mut findings = Vec::new();
    for package_root in package_roots {
        for entry in WalkDir::new(package_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(included_entry)
        {
            let entry = entry?;
            let is_rust = entry.path().extension().and_then(|value| value.to_str()) == Some("rs");
            if is_rust && entry.file_type().is_symlink() {
                return Err(format!(
                    "refusing to scan symlinked Rust source {}",
                    entry.path().display()
                )
                .into());
            }
            if !is_rust || !entry.file_type().is_file() {
                continue;
            }
            let content = std::fs::read_to_string(entry.path())?;
            for (index, line) in content.lines().enumerate() {
                for rule in RULES {
                    let applies = target_major >= rule.target_major_at_least
                        && source_majors
                            .iter()
                            .any(|major| *major <= rule.source_major_at_most);
                    if applies && line.contains(rule.needle) {
                        findings.push(SourceFinding {
                            path: entry.path().to_path_buf(),
                            line: index + 1,
                            code: rule.code,
                            severity: rule.severity,
                            message: rule.message,
                        });
                    }
                }
            }
        }
    }
    findings.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.code.cmp(right.code))
    });
    Ok(findings)
}

fn included_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return true;
    }
    !entry.path().join("Cargo.toml").is_file()
        && !matches!(
            entry.file_name().to_str(),
            Some(".git" | ".rullst" | "node_modules" | "target" | "vendor")
        )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn reports_known_v5_markers_with_lines_and_ignores_target() {
        let root =
            std::env::temp_dir().join(format!("rullst-upgrade-scan-{}", rand::random::<u64>()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("target/generated")).unwrap();
        std::fs::write(
            root.join("src/main.rs"),
            "#[routes]\nfn home() {}\nfn boot() { Server::new(); }\n",
        )
        .unwrap();
        std::fs::write(root.join("target/generated/old.rs"), "APP_ENV").unwrap();

        let findings = scan_workspace(
            std::slice::from_ref(&root),
            &std::collections::BTreeSet::from([5]),
            12,
        )
        .unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].line, 1);
        assert_eq!(findings[0].severity, FindingSeverity::Blocker);

        let future_findings = scan_workspace(
            std::slice::from_ref(&root),
            &std::collections::BTreeSet::from([12]),
            13,
        )
        .unwrap();
        assert!(
            future_findings.is_empty(),
            "v12 rules must not guess v13 fixes"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
