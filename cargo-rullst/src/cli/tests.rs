// src/cli/tests.rs — Parser contract tests for the public CLI definitions.
use super::*;
use clap::CommandFactory;

#[test]
fn parses_nix_and_buildah_flags_without_swapping_them() {
    let nix =
        Cli::try_parse_from(["rullst", "new", "demo", "--default", "--nix"]).expect("Nix CLI");
    assert!(matches!(
        nix.command,
        Commands::New {
            nix: true,
            buildah: false,
            ..
        }
    ));

    let buildah = Cli::try_parse_from(["rullst", "new", "demo", "--default", "--buildah"])
        .expect("Buildah CLI");
    assert!(matches!(
        buildah.command,
        Commands::New {
            nix: false,
            buildah: true,
            ..
        }
    ));
}

#[test]
fn parses_deterministic_persistence_flags_independently() {
    let cli = Cli::try_parse_from([
        "rullst",
        "new",
        "polyglot-app",
        "--default",
        "--database",
        "mariadb",
        "--turso",
        "--mongodb",
        "--duckdb",
        "--surrealdb",
        "--qdrant",
        "--skip-initial-migration",
    ])
    .expect("persistence CLI flags");

    assert!(matches!(
        cli.command,
        Commands::New {
            turso: true,
            mongodb: true,
            duckdb: true,
            surrealdb: true,
            qdrant: true,
            database: Some(DatabaseChoice::Mariadb),
            ..
        }
    ));
}

#[test]
fn parses_turso_as_the_primary_database() {
    let cli = Cli::try_parse_from([
        "rullst",
        "new",
        "edge-primary",
        "--default",
        "--database",
        "turso",
        "--skip-initial-migration",
    ])
    .expect("Turso-primary CLI flags");

    assert!(matches!(
        cli.command,
        Commands::New {
            database: Some(DatabaseChoice::Turso),
            ..
        }
    ));
}

#[test]
fn parses_deterministic_blueprint_generation_flags() {
    let cli = Cli::try_parse_from([
        "rullst",
        "new",
        "release-consumer",
        "--default",
        "--blueprint",
        "erp",
        "--skip-initial-migration",
    ])
    .expect("deterministic blueprint CLI");

    assert!(matches!(
        cli.command,
        Commands::New {
            blueprint: Some(BlueprintChoice::Erp),
            skip_initial_migration: true,
            ..
        }
    ));
}

#[test]
fn parses_streamlined_deterministic_project_profile() {
    let cli = Cli::try_parse_from([
        "rullst",
        "new",
        "release-consumer",
        "--default",
        "--blueprint",
        "erp",
        "--database",
        "postgres",
        "--hot-reload",
        "--ai",
        "--redis",
        "--skip-initial-migration",
    ])
    .expect("complete deterministic project profile");

    assert!(matches!(
        cli.command,
        Commands::New {
            blueprint: Some(BlueprintChoice::Erp),
            database: Some(DatabaseChoice::Postgres),
            hot_reload: true,
            ai: true,
            redis: true,
            skip_initial_migration: true,
            ..
        }
    ));
}

#[test]
fn deterministic_profile_flags_require_default_mode() {
    for flag in ["--no-database", "--hot-reload", "--ai", "--redis"] {
        let mut arguments = vec!["rullst", "new", "profile"];
        arguments.push(flag);
        let error = Cli::try_parse_from(arguments)
            .err()
            .expect("profile flag without deterministic defaults must fail");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument,
            "unexpected parser result for {flag}"
        );
    }
}

#[test]
fn removed_v12_architecture_flags_are_not_advertised_or_accepted() {
    let command = Cli::command();
    let new = command
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "new")
        .expect("new command");
    let argument_ids = new
        .get_arguments()
        .map(|argument| argument.get_id().as_str())
        .collect::<Vec<_>>();
    assert!(!argument_ids.contains(&"orm"));
    assert!(!argument_ids.contains(&"frontend"));

    for removed in [["--orm", "repository"], ["--frontend", "tera"]] {
        let error = Cli::try_parse_from([
            "rullst",
            "new",
            "profile",
            "--default",
            removed[0],
            removed[1],
        ])
        .err()
        .expect("removed v12 selector must fail");
        assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
    }
}

#[test]
fn lms_module_profiles_were_retired() {
    let error = Cli::try_parse_from([
        "rullst",
        "new",
        "demo",
        "--default",
        "--blueprint",
        "lms",
        "--lms-modules",
        "auth,learning",
    ])
    .err()
    .expect("the single LMS starter has no module profiles");

    assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
}

#[test]
fn explicit_blueprint_requires_non_interactive_defaults() {
    let error = Cli::try_parse_from(["rullst", "new", "release-consumer", "--blueprint", "saas"])
        .err()
        .expect("blueprint without deterministic defaults must be rejected");

    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}

#[test]
fn parses_transactional_upgrade_options() {
    let cli = Cli::try_parse_from([
        "rullst",
        "upgrade",
        "--to",
        "12.0.0-rc.1",
        "--dry-run",
        "--json",
        "--keep-on-failure",
    ])
    .expect("upgrade CLI");

    assert!(matches!(
        cli.command,
        Commands::Upgrade {
            to: Some(ref target),
            dry_run: true,
            json: true,
            keep_on_failure: true,
            restore: None,
        } if target == "12.0.0-rc.1"
    ));
}

#[test]
fn parses_deterministic_omni_platforms_and_backend() {
    let cli = Cli::try_parse_from([
        "rullst",
        "make:omni",
        "--platform",
        "desktop,ios",
        "--backend-url",
        "https://api.example.com",
        "--product-name",
        "Acme Chat",
        "--identifier",
        "com.acme.chat",
        "--app-version",
        "1.2.3",
    ])
    .expect("deterministic Omni CLI flags");

    assert!(matches!(
        cli.command,
        Commands::MakeOmni {
            platform,
            backend_url: Some(ref backend_url),
            product_name: Some(ref product_name),
            identifier: Some(ref identifier),
            app_version: Some(ref app_version),
        } if platform == vec![OmniPlatformChoice::Desktop, OmniPlatformChoice::Ios]
            && backend_url == "https://api.example.com"
            && product_name == "Acme Chat"
            && identifier == "com.acme.chat"
            && app_version == "1.2.3"
    ));
}
