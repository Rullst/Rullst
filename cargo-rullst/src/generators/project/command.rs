// cargo-rullst/src/generators/project/command.rs — Runtime extensions of `cargo rullst new`,
// attached in `crate::run` so the published `Commands` enum stays unchanged.

use clap::builder::Resettable;
use clap::{Arg, ArgAction, Command};

use super::create;
use super::vcs::Vcs;
use super::wizard::NewProjectRequest;
use crate::cli::{Commands, DatabaseChoice};

/// The wizard skips every question a profile flag answers, so those flags
/// no longer require `--default`; `--dry-run` previews without writing.
pub(crate) fn new_command(command: Command) -> Command {
    command
        .mut_arg("blueprint", |arg| {
            arg.requires(Resettable::Reset)
                .help("Selects the starter blueprint; the wizard skips that question")
        })
        .mut_arg("database", |arg| {
            arg.requires(Resettable::Reset)
                .help("Selects the primary relational backend; the wizard skips that question")
        })
        .mut_arg("no_database", |arg| {
            arg.requires(Resettable::Reset)
                .help("Generates the Blank starter without a primary relational database")
        })
        .mut_arg("ai", |arg| {
            arg.requires(Resettable::Reset)
                .help("Enables the Rullst AI facade; the wizard skips that question")
        })
        .mut_arg("redis", |arg| {
            arg.requires(Resettable::Reset)
                .help("Enables the Redis-backed adapters; the wizard skips that question")
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
        buildah,
        nix,
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
    let request = NewProjectRequest {
        name: name.as_deref(),
        options: super::ProjectScaffoldOptions {
            api: *api,
            docker: *docker,
            buildah: *buildah,
            nix: *nix,
            use_defaults: *default,
            turso: *turso,
            mongodb: *mongodb,
            duckdb: *duckdb,
            surrealdb: *surrealdb,
            qdrant: *qdrant,
            database: database.map(provider),
            no_database: *no_database,
            hot_reload: *hot_reload,
            wants_ai: *ai,
            wants_redis: *redis,
        },
        blueprint: blueprint.map(|choice| choice.id()),
        skip_initial_migration: *skip_initial_migration,
        dry_run,
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

    #[test]
    fn database_choices_map_to_generator_providers() {
        assert_eq!(provider(DatabaseChoice::Sqlite), "Sqlite");
        assert_eq!(provider(DatabaseChoice::Postgres), "Postgres");
        assert_eq!(provider(DatabaseChoice::Mysql), "MySQL");
        assert_eq!(provider(DatabaseChoice::Mariadb), "MariaDB");
        assert_eq!(provider(DatabaseChoice::Turso), "Turso");
    }
}
