// cargo-rullst/src/generators/project/command.rs — Runtime extensions of `cargo rullst new`,
// attached in `crate::run` so the published `Commands` enum stays unchanged.

use clap::builder::Resettable;
use clap::{Arg, ArgAction, Command};

use super::create;
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

/// `cargo rullst new … --dry-run`: the same plan as a real run, printed.
pub(crate) fn run_dry_run(command: &Commands) -> Result<(), Box<dyn std::error::Error>> {
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
        return Err("--dry-run applies only to `cargo rullst new`".into());
    };
    create::run_new(&NewProjectRequest {
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
        dry_run: true,
    })
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
