//! `cargo rullst info` (alias `version`): the CLI version, platform, Rust
//! toolchain and the detected project, as text or as the versioned
//! `rullst.cli-info.v1` JSON document. Read-only; no network access.

use crate::generators::doctor::probe;
use crate::ui::home::{Database, Project, rullst_requirement};
use crate::ui::style::{self, Style};
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::Serialize;
use std::io::{self, Write};

pub(crate) const SCHEMA_VERSION: &str = "rullst.cli-info.v1";

pub(crate) fn command() -> Command {
    Command::new("info")
        .visible_alias("version")
        .about("Show the CLI version, platform, Rust toolchain and detected project")
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Print versioned JSON (rullst.cli-info.v1) for automation"),
        )
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Cli {
    name: &'static str,
    version: &'static str,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Platform {
    os: &'static str,
    arch: &'static str,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Toolchain {
    rustc: Option<String>,
    cargo: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum DatabaseInfo {
    Configured {
        kind: &'static str,
        source: &'static str,
    },
    NotConfigured,
    Unreadable {
        reason: &'static str,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Migrations {
    count: usize,
    latest: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct ProjectInfo {
    name: String,
    root: String,
    relative_root: Option<String>,
    rullst: Option<String>,
    features: Vec<String>,
    database: DatabaseInfo,
    migrations: Option<Migrations>,
    git_branch: Option<String>,
}

/// The `rullst.cli-info.v1` document.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Info {
    schema_version: &'static str,
    cli: Cli,
    platform: Platform,
    toolchain: Toolchain,
    project: Option<ProjectInfo>,
}

fn project_info(project: &Project) -> ProjectInfo {
    let manifest = crate::ui::home::read_small_file(&project.root.join("Cargo.toml"), 1024 * 1024)
        .ok()
        .flatten()
        .unwrap_or_default();
    ProjectInfo {
        name: project.name.clone(),
        root: project.root.display().to_string(),
        relative_root: project.relative_root.clone(),
        rullst: rullst_requirement(&manifest),
        features: project.features.clone(),
        database: match &project.database {
            Database::Configured { kind, source } => DatabaseInfo::Configured { kind, source },
            Database::NotConfigured => DatabaseInfo::NotConfigured,
            Database::Unreadable(reason) => DatabaseInfo::Unreadable { reason },
        },
        migrations: project.migrations.as_ref().map(|migrations| Migrations {
            count: migrations.count,
            latest: migrations.latest.clone(),
        }),
        git_branch: project.git_branch.clone(),
    }
}

impl Info {
    pub(crate) fn new(
        rustc: Option<String>,
        cargo: Option<String>,
        project: Option<&Project>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            cli: Cli {
                name: "cargo-rullst",
                version: env!("CARGO_PKG_VERSION"),
            },
            platform: Platform {
                os: std::env::consts::OS,
                arch: std::env::consts::ARCH,
            },
            toolchain: Toolchain { rustc, cargo },
            project: project.map(project_info),
        }
    }

    fn detect() -> Self {
        let project = std::env::current_dir()
            .ok()
            .and_then(|start| Project::detect(&start, |name| std::env::var(name).ok()));
        Self::new(
            probe::tool_version("rustc"),
            probe::tool_version("cargo"),
            project.as_ref(),
        )
    }

    /// Aligned text; values come from local files and are display-safe.
    pub(crate) fn render(&self, style: Style) -> String {
        let mut rows: Vec<(&str, String)> = vec![
            (
                "Platform",
                format!("{} · {}", self.platform.os, self.platform.arch),
            ),
            (
                "rustc",
                self.toolchain
                    .rustc
                    .clone()
                    .unwrap_or_else(|| "not found".into()),
            ),
            (
                "cargo",
                self.toolchain
                    .cargo
                    .clone()
                    .unwrap_or_else(|| "not found".into()),
            ),
        ];
        match &self.project {
            None => rows.push(("Project", "none in this directory or its parents".into())),
            Some(project) => {
                rows.push(("Project", project.name.clone()));
                if let Some(root) = &project.relative_root {
                    rows.push(("Root", root.clone()));
                }
                if let Some(rullst) = &project.rullst {
                    rows.push(("Rullst", rullst.clone()));
                }
                let features = if project.features.is_empty() {
                    "none".to_string()
                } else {
                    project.features.join(" · ")
                };
                rows.push(("Features", features));
                rows.push((
                    "Database",
                    match &project.database {
                        DatabaseInfo::Configured { kind, source } => format!("{kind} · {source}"),
                        DatabaseInfo::NotConfigured => "not configured".into(),
                        DatabaseInfo::Unreadable { reason } => (*reason).into(),
                    },
                ));
                if let Some(migrations) = &project.migrations {
                    rows.push((
                        "Migrations",
                        match &migrations.latest {
                            Some(latest) => {
                                format!("{} defined · latest {latest}", migrations.count)
                            }
                            None => "none yet".into(),
                        },
                    ));
                }
                if let Some(branch) = &project.git_branch {
                    rows.push(("Git branch", branch.clone()));
                }
            }
        }
        let width = rows.iter().map(|(label, _)| label.len()).max().unwrap_or(0) + 2;
        let mut out = format!(
            "{} {}\n",
            style.bold(self.cli.name, style::BRIGHT),
            style.bold(self.cli.version, style::ACCENT)
        );
        for (label, value) in rows {
            out.push_str(&format!(
                "  {}{}\n",
                style.paint(&format!("{label:<width$}"), style::MUTED),
                style.paint(&value, style::BRIGHT)
            ));
        }
        out
    }
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let json = matches
        .try_get_one::<bool>("json")
        .ok()
        .flatten()
        .copied()
        .unwrap_or(false);
    let info = Info::detect();
    let mut stdout = io::stdout().lock();
    if json {
        writeln!(stdout, "{}", serde_json::to_string_pretty(&info)?)?;
    } else {
        write!(stdout, "{}", info.render(Style::stdout()))?;
    }
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::home::Migrations as HomeMigrations;
    use std::path::PathBuf;

    fn project() -> Project {
        Project {
            root: PathBuf::from("/work/shop"),
            relative_root: Some("..".to_string()),
            name: "shop".to_string(),
            features: vec!["orm".to_string()],
            database: Database::Configured {
                kind: "SQLite",
                source: ".env",
            },
            migrations: Some(HomeMigrations {
                count: 2,
                latest: Some("m2_items".to_string()),
            }),
            git_branch: Some("main".to_string()),
        }
    }

    #[test]
    fn the_json_document_has_the_documented_shape() {
        let info = Info::new(Some("1.98.1".into()), None, Some(&project()));
        let value = serde_json::to_value(&info).expect("serializable");
        assert_eq!(value["schema_version"], "rullst.cli-info.v1");
        assert_eq!(value["cli"]["name"], "cargo-rullst");
        assert_eq!(value["cli"]["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["platform"]["os"], std::env::consts::OS);
        assert_eq!(value["toolchain"]["rustc"], "1.98.1");
        assert!(value["toolchain"]["cargo"].is_null());
        let project = &value["project"];
        assert_eq!(project["name"], "shop");
        assert_eq!(project["relative_root"], "..");
        assert_eq!(project["database"]["status"], "configured");
        assert_eq!(project["database"]["kind"], "SQLite");
        assert_eq!(project["migrations"]["count"], 2);
        assert_eq!(project["git_branch"], "main");
        assert!(project.get("rullst").is_some());

        let outside = serde_json::to_value(Info::new(None, None, None)).expect("serializable");
        assert!(outside["project"].is_null());
    }

    #[test]
    fn the_text_form_is_plain_and_aligned() {
        let text = Info::new(
            Some("1.98.1".into()),
            Some("1.98.1".into()),
            Some(&project()),
        )
        .render(Style::PLAIN);
        assert!(text.starts_with(&format!("cargo-rullst {}\n", env!("CARGO_PKG_VERSION"))));
        assert!(text.contains("  Project     shop\n"));
        assert!(text.contains("  Database    SQLite · .env\n"));
        assert!(text.contains("  Migrations  2 defined · latest m2_items\n"));
        assert!(!text.contains('\x1b'));
        let outside = Info::new(None, None, None).render(Style::PLAIN);
        assert!(outside.contains("  rustc     not found\n"));
        assert!(outside.contains("  Project   none in this directory or its parents\n"));
    }
}
