// cargo-rullst/src/generators/project/command.rs — Runtime extensions of `cargo rullst new`,
// attached in `crate::run` so the published `Commands` enum stays unchanged.

use clap::builder::Resettable;
use clap::{Arg, ArgAction, Command};

use super::create;
use super::vcs::Vcs;
use super::wizard::NewProjectRequest;
use crate::cli::{BlueprintChoice, Commands, DatabaseChoice};

/// The wizard skips every question a profile flag answers, so those flags
/// no longer require `--default`; `--dry-run` previews without writing.
pub(crate) fn new_command(command: Command) -> Command {
    command
        .mut_arg("blueprint", |arg| {
            arg.requires(Resettable::Reset)
                .help("Selects the starter (blank, api or saas); the wizard skips that question")
        })
        .mut_arg("database", |arg| {
            arg.requires(Resettable::Reset).help(
                "Selects the primary database (--help lists what each one supports); the wizard skips that question",
            )
        })
        .mut_arg("no_database", |arg| {
            arg.requires(Resettable::Reset)
                .help("Generates the Blank starter without a primary relational database")
        })
        .mut_arg("ai", |arg| {
            arg.requires(Resettable::Reset)
                .help("Enables the Rullst AI facade and, in the SaaS starter, the Nexus AI assistant; the wizard skips that question")
        })
        .mut_arg("redis", |arg| {
            arg.requires(Resettable::Reset).help(
                "Enables Redis queues, cache and the Redis-structures side store; the wizard skips that question",
            )
        })
        .arg(
            Arg::new("dry_run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help(
                    "Prints the plan, the file tree and the commands it would run, then exits without creating anything",
                ),
        )
        .arg(
            Arg::new("no_ci")
                .long("no-ci")
                .action(ArgAction::SetTrue)
                .help(
                    "Skips the GitHub CI workflow (.github/workflows/ci.yml) and .github/dependabot.yml",
                ),
        )
        .arg(
            Arg::new("vcs")
                .long("vcs")
                .value_name("VCS")
                .value_parser(Vcs::VALUES)
                .default_value("git")
                .help(
                    "Initializes a Git repository (git) unless the destination is already inside one, or none",
                ),
        )
}

/// `--mongodb`, `--duckdb` and `--qdrant` added a side store until
/// 13.0.0-alpha.5, which removed those adapters. The flags stay hidden so that
/// a script that still passes one learns which recipe replaces it, instead of
/// clap's unknown argument error.
pub(crate) fn reject_removed_side_stores(
    mongodb: bool,
    duckdb: bool,
    qdrant: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let removed = [
        (mongodb, "mongodb", "MongoDB", "mongodb"),
        (duckdb, "duckdb", "DuckDB", "duckdb"),
        (qdrant, "qdrant", "Qdrant", "qdrant-client"),
    ];
    match removed.into_iter().find(|(selected, ..)| *selected) {
        Some((_, flag, name, krate)) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "--{flag} was removed in 13.0.0-alpha.5: Rullst no longer ships a {name} adapter. Generate the project without it and use the official `{krate}` crate in your application, as https://rullst.github.io/Rullst/book/recipes/{flag}.html shows"
            ),
        )
        .into()),
        None => Ok(()),
    }
}

const fn provider(choice: DatabaseChoice) -> &'static str {
    match choice {
        DatabaseChoice::Sqlite => "Sqlite",
        DatabaseChoice::Postgres => "Postgres",
        DatabaseChoice::Mysql => "MySQL",
        DatabaseChoice::Mariadb => "MariaDB",
        DatabaseChoice::Turso => "Turso",
    }
}

/// `cargo rullst new` with its runtime flags: `--dry-run` prints the same
/// plan as a real run; `--vcs` selects the repository set up afterwards.
pub(crate) fn run_new_command(
    command: &Commands,
    matches: &clap::ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let dry_run = matches
        .try_get_one::<bool>("dry_run")
        .ok()
        .flatten()
        .copied()
        .unwrap_or(false);
    let skip_ci = matches
        .try_get_one::<bool>("no_ci")
        .ok()
        .flatten()
        .copied()
        .unwrap_or(false);
    let vcs = matches
        .try_get_one::<String>("vcs")
        .ok()
        .flatten()
        .and_then(|value| Vcs::parse(value))
        .unwrap_or_default();
    let Commands::New {
        name,
        api,
        docker,
        default,
        blueprint,
        database,
        no_database,
        hot_reload,
        ai,
        redis,
        skip_initial_migration,
        turso,
        mongodb,
        duckdb,
        surrealdb,
        qdrant,
    } = command
    else {
        return Err("--dry-run and --vcs apply only to `cargo rullst new`".into());
    };
    // A removed flag or blueprint fails here, before the wizard or any file write.
    reject_removed_side_stores(*mongodb, *duckdb, *qdrant)?;
    let starter = blueprint.map(BlueprintChoice::starter).transpose()?;
    let request = NewProjectRequest {
        name: name.as_deref(),
        options: super::ProjectScaffoldOptions {
            api: *api || blueprint.is_some_and(BlueprintChoice::is_api),
            docker: *docker,
            use_defaults: *default,
            turso: *turso,
            surrealdb: *surrealdb,
            database: database.map(provider),
            no_database: *no_database,
            hot_reload: *hot_reload,
            wants_ai: *ai,
            wants_redis: *redis,
        },
        blueprint: starter,
        skip_initial_migration: *skip_initial_migration,
        dry_run,
        skip_ci,
    };
    create::run_new_with_vcs(&request, vcs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, FromArgMatches};

    fn parse(arguments: &[&str]) -> Result<clap::ArgMatches, clap::Error> {
        crate::cli::Cli::command()
            .mut_subcommand("new", new_command)
            .try_get_matches_from(arguments)
    }

    #[test]
    fn profile_flags_no_longer_require_default_and_still_parse_into_commands() {
        let matches = parse(&[
            "rullst",
            "new",
            "shop",
            "--blueprint",
            "saas",
            "--database",
            "postgres",
            "--ai",
            "--redis",
            "--dry-run",
        ])
        .expect("profile flags without --default");
        assert!(
            matches
                .subcommand_matches("new")
                .is_some_and(|new| new.get_flag("dry_run"))
        );
        let cli = crate::cli::Cli::from_arg_matches(&matches).expect("derive still reads it");
        assert!(matches!(
            cli.command,
            Commands::New {
                default: false,
                ai: true,
                redis: true,
                database: Some(DatabaseChoice::Postgres),
                ..
            }
        ));

        assert!(parse(&["rullst", "new", "lean", "--no-database"]).is_ok());
        let new = parse(&["rullst", "new", "lean"]).expect("default vcs");
        assert_eq!(
            new.subcommand_matches("new")
                .and_then(|new| new.get_one::<String>("vcs"))
                .map(String::as_str),
            Some("git")
        );
        assert!(parse(&["rullst", "new", "lean", "--vcs", "none"]).is_ok());
        let no_ci = parse(&["rullst", "new", "lean", "--no-ci"]).expect("--no-ci");
        assert!(
            no_ci
                .subcommand_matches("new")
                .is_some_and(|new| new.get_flag("no_ci"))
        );
        assert!(
            !new.subcommand_matches("new")
                .is_some_and(|new| new.get_flag("no_ci"))
        );
        assert_eq!(
            parse(&["rullst", "new", "lean", "--vcs", "hg"])
                .expect_err("unknown vcs")
                .kind(),
            clap::error::ErrorKind::InvalidValue
        );
        let conflict = parse(&[
            "rullst",
            "new",
            "x",
            "--no-database",
            "--database",
            "sqlite",
        ])
        .expect_err("--no-database still conflicts with --database");
        assert_eq!(conflict.kind(), clap::error::ErrorKind::ArgumentConflict);
        let legacy = parse(&["rullst", "new", "x", "--hot-reload"])
            .expect_err("the rejected legacy profile still requires --default");
        assert_eq!(
            legacy.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }

    /// `new --help` with its runtime help, whitespace collapsed.
    fn long_help() -> String {
        let mut command = crate::cli::Cli::command().mut_subcommand("new", new_command);
        let help = command
            .find_subcommand_mut("new")
            .expect("new subcommand")
            .render_long_help()
            .to_string();
        help.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn help_states_the_turso_limits_and_names_the_side_stores() {
        let help = long_help();
        for expected in [
            "Selects the primary database (--help lists what each one supports); the wizard skips that question",
            "sqlite: SQLite through SQLx; every starter (the default)",
            "postgres: PostgreSQL through SQLx; every starter",
            "mysql: MySQL through SQLx; every starter",
            "mariadb: MariaDB through SQLx's MySQL driver; every starter",
            "turso: Turso/libSQL (Blank/API only; no Nexus, Studio, Schema or SQLx ORM)",
            "Enables Redis queues, cache and the Redis-structures side store; the wizard skips that question",
            "--turso Adds Turso/libSQL as a side store (edge SQL) beside the primary database",
            "--surrealdb Adds SurrealDB as a side store (documents and read-only graph queries)",
        ] {
            assert!(help.contains(expected), "missing {expected:?} in:\n{help}");
        }
    }

    #[test]
    fn removed_side_store_flags_are_hidden_and_name_their_recipe() {
        let help = long_help();
        for removed in [
            "--mongodb",
            "--duckdb",
            "--qdrant",
            "MongoDB",
            "DuckDB",
            "Qdrant",
        ] {
            assert!(!help.contains(removed), "{removed} in:\n{help}");
        }
        assert!(reject_removed_side_stores(false, false, false).is_ok());
        for (mongodb, duckdb, qdrant, flag, name, krate) in [
            (true, false, false, "mongodb", "MongoDB", "mongodb"),
            (false, true, false, "duckdb", "DuckDB", "duckdb"),
            (false, false, true, "qdrant", "Qdrant", "qdrant-client"),
            (true, true, true, "mongodb", "MongoDB", "mongodb"),
            (false, true, true, "duckdb", "DuckDB", "duckdb"),
        ] {
            let message = reject_removed_side_stores(mongodb, duckdb, qdrant)
                .expect_err("a removed flag is refused")
                .to_string();
            assert!(
                message.starts_with(&format!("--{flag} was removed in 13.0.0-alpha.5: ")),
                "{message}"
            );
            assert!(
                message.contains(&format!("no longer ships a {name} adapter")),
                "{message}"
            );
            assert!(
                message.contains(&format!("the official `{krate}` crate")),
                "{message}"
            );
            assert!(
                message.ends_with(&format!(
                    "https://rullst.github.io/Rullst/book/recipes/{flag}.html shows"
                )),
                "{message}"
            );
        }
        let matches = parse(&[
            "rullst",
            "new",
            "shop",
            "--default",
            "--mongodb",
            "--duckdb",
            "--qdrant",
        ])
        .expect("the hidden flags still parse");
        let cli = crate::cli::Cli::from_arg_matches(&matches).expect("derive reads them");
        assert!(matches!(
            cli.command,
            Commands::New {
                mongodb: true,
                duckdb: true,
                qdrant: true,
                ..
            }
        ));
        let new = matches.subcommand_matches("new").expect("new matches");
        let error = run_new_command(&cli.command, new)
            .expect_err("`new --mongodb` stops before the wizard or any file write");
        assert!(
            error.to_string().starts_with("--mongodb was removed"),
            "{error}"
        );
    }

    #[test]
    fn database_choices_map_to_generator_providers() {
        assert_eq!(provider(DatabaseChoice::Sqlite), "Sqlite");
        assert_eq!(provider(DatabaseChoice::Postgres), "Postgres");
        assert_eq!(provider(DatabaseChoice::Mysql), "MySQL");
        assert_eq!(provider(DatabaseChoice::Mariadb), "MariaDB");
        assert_eq!(provider(DatabaseChoice::Turso), "Turso");
    }
}
