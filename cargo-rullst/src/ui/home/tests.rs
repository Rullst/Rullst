use super::project::{
    find_project, migrations, parse_manifest, read_small_file, relative_root, resolve_database,
};
use super::*;
use std::fs;
use std::path::PathBuf;

fn no_env(_: &str) -> Option<String> {
    None
}

fn fixture_project(relative_root: Option<&str>) -> Project {
    Project {
        root: PathBuf::from("/work/shop"),
        relative_root: relative_root.map(str::to_string),
        name: "shop".to_string(),
        features: vec!["orm".to_string(), "studio".to_string()],
        database: Database::Configured {
            kind: "SQLite",
            source: ".env",
        },
        migrations: Some(Migrations {
            count: 2,
            latest: Some("m20260101000000_create_users".to_string()),
        }),
        git_branch: Some("main".to_string()),
    }
}

#[test]
fn manifests_are_projects_only_when_they_depend_on_rullst() {
    assert_eq!(
        parse_manifest("[package]\nname = \"app\"\n[dependencies]\nrullst = \"13\"\n"),
        Some(("app".to_string(), vec!["default".to_string()]))
    );
    assert_eq!(
        parse_manifest(
            "[package]\nname = \"shop\"\n[dependencies]\nrullst = { version = \"13\", \
             default-features = false, features = [\"orm\", \"strict-sqlite\", \"studio\"] }\n"
        ),
        Some((
            "shop".to_string(),
            ["orm", "strict-sqlite", "studio"]
                .map(str::to_string)
                .to_vec()
        ))
    );
    assert_eq!(
        parse_manifest(
            "[package]\nname = \"renamed\"\n[dependencies]\nweb = { package = \"rullst\", \
             features = [\"ai\"] }\n"
        ),
        Some((
            "renamed".to_string(),
            ["default", "ai"].map(str::to_string).to_vec()
        ))
    );
    for manifest in [
        "[package]\nname = \"core-only\"\n[dependencies]\nrullst-core = \"13\"\n",
        "[package]\nname = \"other\"\n[dependencies]\nrullst = { package = \"not-rullst\" }\n",
        "[workspace]\n[workspace.dependencies]\nrullst = \"13\"\n",
        "[package\nname = \"broken\"",
        "[package]\nname = \"plain\"\n",
    ] {
        assert_eq!(parse_manifest(manifest), None, "{manifest}");
    }
}

#[test]
fn untrusted_names_cannot_inject_terminal_sequences() {
    let (name, features) = parse_manifest(
        "[package]\nname = \"evil\\u001b]0;title\\u0007\"\n[dependencies]\n\
         rullst = { version = \"13\", features = [\"a\\u001b[2J\"] }\n",
    )
    .unwrap();
    assert_eq!(name, "evil?]0;title?");
    assert_eq!(features, ["default", "a?[2J"]);

    let long = "x".repeat(200);
    let (name, _) = parse_manifest(&format!(
        "[package]\nname = \"{long}\"\n[dependencies]\nrullst = \"13\"\n"
    ))
    .unwrap();
    assert_eq!(name.chars().count(), 65);
    assert!(name.ends_with('…'));
}

#[test]
fn the_nearest_rullst_ancestor_is_the_project_root() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("shop");
    let nested = root.join("src").join("controllers");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"shop\"\n[dependencies]\nrullst = \"13\"\n",
    )
    .unwrap();

    let (found, name, _) = find_project(&nested).unwrap();
    assert_eq!(found, root);
    assert_eq!(name, "shop");
    assert_eq!(relative_root(&nested, &root).as_deref(), Some("../.."));
    assert_eq!(relative_root(&root, &root), None);
    assert!(find_project(directory.path()).is_none());

    let project = Project::detect(&nested, no_env).unwrap();
    assert_eq!(project.relative_root.as_deref(), Some("../.."));
    assert_eq!(project.database, Database::NotConfigured);
    assert_eq!(project.migrations, None);
}

#[test]
fn oversized_and_special_files_are_not_read() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("big");
    fs::write(&file, "x".repeat(32)).unwrap();
    assert_eq!(read_small_file(&file, 16).unwrap(), None);
    assert_eq!(read_small_file(&file, 32).unwrap().unwrap().len(), 32);
    assert_eq!(read_small_file(directory.path(), 1024).unwrap(), None);
    assert_eq!(
        read_small_file(&directory.path().join("missing"), 16).unwrap(),
        None
    );
}

#[test]
fn database_configuration_follows_the_runtime_precedence() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let secret = "postgres://admin:hunter2@db.internal/app";
    fs::write(root.join(".env"), "DATABASE_URL=sqlite://app.db\n").unwrap();
    fs::write(
        root.join("Rullst.toml"),
        "[database]\nurl = \"mysql://u:p@h/db\"\n",
    )
    .unwrap();

    let environment = resolve_database(root, |name| {
        (name == "DATABASE_URL").then(|| secret.to_string())
    });
    assert_eq!(
        environment,
        Database::Configured {
            kind: "PostgreSQL",
            source: "environment"
        }
    );
    assert!(!format!("{environment:?}").contains("hunter2"));

    assert_eq!(
        resolve_database(root, no_env),
        Database::Configured {
            kind: "SQLite",
            source: ".env"
        }
    );

    fs::write(root.join(".env"), "APP_ENV=development\n").unwrap();
    assert_eq!(
        resolve_database(root, no_env),
        Database::Configured {
            kind: "MySQL/MariaDB",
            source: "Rullst.toml"
        }
    );

    fs::write(root.join("Rullst.toml"), "[app]\nname = \"shop\"\n").unwrap();
    assert_eq!(resolve_database(root, no_env), Database::NotConfigured);

    fs::write(
        root.join(".env"),
        "TURSO_DATABASE_URL=libsql://x.turso.io\n",
    )
    .unwrap();
    assert_eq!(
        resolve_database(root, no_env),
        Database::Configured {
            kind: "Turso/libSQL",
            source: ".env"
        }
    );

    fs::write(root.join(".env"), "DATABASE_URL='unterminated\n").unwrap();
    assert_eq!(
        resolve_database(root, no_env),
        Database::Unreadable(".env could not be parsed")
    );

    fs::write(root.join(".env"), "").unwrap();
    fs::write(root.join("Rullst.toml"), "[database\nurl = 1").unwrap();
    assert_eq!(
        resolve_database(root, no_env),
        Database::Unreadable("Rullst.toml is not valid TOML")
    );
}

#[test]
fn migration_files_are_counted_without_the_registry() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    assert_eq!(migrations(root), None);

    let folder = root.join("src").join("migrations");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("mod.rs"), "").unwrap();
    assert_eq!(
        migrations(root),
        Some(Migrations {
            count: 0,
            latest: None
        })
    );

    for name in [
        "m20260101000000_create_users.rs",
        "m20260301000000_add_status.rs",
        "notes.txt",
        "helper.rs",
    ] {
        fs::write(folder.join(name), "").unwrap();
    }
    assert_eq!(
        migrations(root),
        Some(Migrations {
            count: 2,
            latest: Some("m20260301000000_add_status".to_string())
        })
    );
}

#[test]
fn git_heads_name_branches_or_detached_commits() {
    assert_eq!(
        git::describe_head("ref: refs/heads/feat/v13-home\n").as_deref(),
        Some("feat/v13-home")
    );
    assert_eq!(
        git::describe_head("ref: refs/remotes/origin/main").as_deref(),
        Some("remotes/origin/main")
    );
    assert_eq!(
        git::describe_head("0123456789abcdef0123456789abcdef01234567").as_deref(),
        Some("detached at 0123456")
    );
    assert_eq!(git::describe_head("not a head"), None);
    assert_eq!(git::describe_head("ref: "), None);

    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join("repo");
    let nested = repository.join("app");
    fs::create_dir_all(repository.join(".git")).unwrap();
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        repository.join(".git").join("HEAD"),
        "ref: refs/heads/main\n",
    )
    .unwrap();
    assert_eq!(git::branch(&nested).as_deref(), Some("main"));

    // A linked worktree points at its private Git directory.
    let worktree = directory.path().join("worktree");
    let private = directory.path().join("git-private");
    fs::create_dir_all(&worktree).unwrap();
    fs::create_dir_all(&private).unwrap();
    fs::write(private.join("HEAD"), "ref: refs/heads/feature\n").unwrap();
    fs::write(worktree.join(".git"), "gitdir: ../git-private\n").unwrap();
    assert_eq!(git::branch(&worktree).as_deref(), Some("feature"));
}

#[test]
fn the_plain_summary_lists_only_real_project_data() {
    let mut out = Vec::new();
    let home = Home::Project(Box::new(fixture_project(None)));
    write_summary(&mut out, &home, ColorDepth::None).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Project     shop\n\
         Features    orm · studio\n\
         Database    SQLite · .env\n\
         Migrations  2 defined · latest m20260101000000_create_users\n\
         Git branch  main\n\n"
    );

    let mut sparse = fixture_project(Some(".."));
    sparse.features.clear();
    sparse.database = Database::NotConfigured;
    sparse.migrations = None;
    sparse.git_branch = None;
    let mut out = Vec::new();
    write_summary(&mut out, &Home::Project(Box::new(sparse)), ColorDepth::None).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Project   shop\nRoot      ..\nFeatures  none\nDatabase  not configured\n\n"
    );

    let mut out = Vec::new();
    write_summary(&mut out, &Home::Outside, ColorDepth::None).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        format!(
            "Project        none in this directory or its parents\n\
             Start here     {START_HERE_URL}\n\
             CLI reference  {CLI_REFERENCE_URL}\n\n"
        )
    );
}

#[test]
fn the_coloured_summary_aligns_with_the_wordmark() {
    let mut out = Vec::new();
    let home = Home::Project(Box::new(fixture_project(None)));
    write_summary(&mut out, &home, ColorDepth::TrueColor).unwrap();
    let text = String::from_utf8(out).unwrap();
    let first = text.lines().next().unwrap();
    assert_eq!(
        first,
        "  \x1b[38;2;150;155;175mProject     \x1b[0m\x1b[1m\x1b[38;2;240;240;248mshop\x1b[0m"
    );
}

#[test]
fn next_steps_are_the_non_interactive_menu_equivalent() {
    let mut out = Vec::new();
    write_next_steps(&mut out, &Home::Outside).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Get started\n  cargo rullst new <name>  Create a new Rullst application\n\n\
         Run `cargo rullst --help` to list every command.\n"
    );

    let mut out = Vec::new();
    let home = Home::Project(Box::new(fixture_project(Some("../.."))));
    write_next_steps(&mut out, &home).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(text.starts_with("Next steps (from ../..)\n"));
    for command in [
        "dev",
        "dash",
        "make:model",
        "db:migrate",
        "doctor",
        "deploy",
    ] {
        assert!(
            text.contains(&format!("cargo rullst {command}")),
            "{command}"
        );
    }
}

#[test]
fn project_commands_run_at_the_root_and_new_runs_in_place() {
    let args = |command: &str| vec!["cargo-rullst".to_string(), command.to_string()];
    let nested = Home::Project(Box::new(fixture_project(Some(".."))));
    assert_eq!(
        nested.command_directory(&args("dev")),
        Some(std::path::Path::new("/work/shop"))
    );
    assert_eq!(nested.command_directory(&args("new")), None);
    let at_root = Home::Project(Box::new(fixture_project(None)));
    assert_eq!(at_root.command_directory(&args("dev")), None);
    assert_eq!(Home::Outside.command_directory(&args("dev")), None);
}

#[test]
fn home_entries_lead_with_the_context_and_run_real_commands() {
    let outside: Vec<_> = home_entries(&Home::Outside)
        .iter()
        .map(|entry| entry.action)
        .collect();
    assert_eq!(
        outside,
        [
            HomeAction::NewProject,
            HomeAction::ProjectOperations,
            HomeAction::Palette,
            HomeAction::Help,
            HomeAction::Exit
        ]
    );

    let project = Home::Project(Box::new(fixture_project(None)));
    let actions: Vec<_> = home_entries(&project)
        .iter()
        .map(|entry| entry.action)
        .collect();
    for expected in [
        HomeAction::Command("dev"),
        HomeAction::Command("dash"),
        HomeAction::Scaffold,
        HomeAction::Database,
        HomeAction::Command("doctor"),
        HomeAction::Deploy,
        HomeAction::ProjectOperations,
        HomeAction::NewProject,
        HomeAction::Palette,
        HomeAction::Help,
        HomeAction::Exit,
    ] {
        assert!(actions.contains(&expected), "{expected:?}");
    }
    assert_eq!(actions.first(), Some(&HomeAction::Command("dev")));

    for entry in home_entries(&project) {
        assert!(!entry.label().is_empty());
        if let HomeAction::Command(command) = entry.action {
            <crate::cli::Cli as clap::Parser>::try_parse_from(["cargo-rullst", command])
                .unwrap_or_else(|error| panic!("{command}: {error}"));
        }
    }
}

#[test]
fn every_home_title_uses_one_wide_emoji_and_the_same_width() {
    for home in [
        Home::Outside,
        Home::Project(Box::new(fixture_project(None))),
    ] {
        for entry in home_entries(&home) {
            let emoji = entry.title.chars().next().unwrap();
            assert!(u32::from(emoji) >= 0x2700, "{}", entry.title);
            assert!(entry.title[emoji.len_utf8()..].starts_with("  "));
            assert_eq!(entry.title.chars().count(), 28, "{}", entry.title);
        }
    }
}
