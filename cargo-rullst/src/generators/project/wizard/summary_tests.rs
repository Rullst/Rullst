use super::*;
use crate::generators::project::PolyglotIntegration;
use crate::generators::project::wizard::plan::Database;

fn files(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| path.to_string()).collect()
}

fn texts(lines: Vec<Line>) -> Vec<String> {
    lines.iter().map(Line::text).collect()
}

const BLANK_LIKE: [&str; 16] = [
    ".cargo/config.toml",
    ".env",
    ".env.example",
    ".gitignore",
    ".llms.txt",
    ".rullst/context-map.json",
    "Cargo.toml",
    "rullst.db",
    "src/main.rs",
    "src/migrations/m1.rs",
    "src/migrations/mod.rs",
    "src/rpc.rs",
    "static/HTMX-LICENSE",
    "static/htmx.js",
    "static/rullst.css",
    "static/rullst.png",
];

#[test]
fn compact_tree_opens_source_folders_and_collapses_the_rest() {
    assert_eq!(
        texts(tree_lines("my_app", &files(&BLANK_LIKE), 200)),
        [
            "my_app/  16 files",
            "├── src/",
            "│   ├── migrations/  2 files",
            "│   └── main.rs  rpc.rs",
            "├── static/  4 files",
            "└── Cargo.toml  rullst.db  .cargo/config.toml  .env  .env.example  .gitignore  .llms.txt  .rullst/context-map.json",
        ]
    );
}

#[test]
fn compact_tree_fits_a_width_and_counts_what_it_hides() {
    let lines = texts(tree_lines("my_app", &files(&BLANK_LIKE), 56));
    assert_eq!(
        lines.last().map(String::as_str),
        Some("└── Cargo.toml  rullst.db  .cargo/config.toml  +5 more")
    );
    assert!(lines.iter().all(|line| line.chars().count() <= 56));

    // The last directory closes the tree when there are no root files.
    let only_dirs = texts(tree_lines(
        "x",
        &files(&[
            "src/a.rs",
            "src/b/c.rs",
            "src/b/d.rs",
            "assets/one.css",
            "assets/two.css",
        ]),
        80,
    ));
    assert_eq!(
        only_dirs,
        [
            "x/  5 files",
            "├── assets/  2 files",
            "└── src/",
            "    ├── b/  2 files",
            "    └── a.rs",
        ]
    );
    assert_eq!(texts(tree_lines("empty", &[], 80)), ["empty/  0 files"]);
    assert_eq!(
        texts(tree_lines("one", &files(&["deep/er/file.rs"]), 80)),
        ["one/  1 file", "└── deep/er/file.rs"]
    );
}

#[test]
fn shell_words_quote_only_when_needed() {
    assert_eq!(shell_word("my_app"), "my_app");
    assert_eq!(shell_word("../apps/my-app"), "../apps/my-app");
    assert_eq!(shell_word("my app"), "'my app'");
    assert_eq!(shell_word("it's"), r"'it'\''s'");
    assert_eq!(shell_word(""), "''");
}

fn plan() -> ProjectPlan {
    ProjectPlan {
        name: "my_app".to_string(),
        blueprint: BLANK_BLUEPRINT_ID,
        api: false,
        database: Database::Provider("Sqlite"),
        ai: false,
        redis: false,
        docker: false,
        nix: false,
        buildah: false,
        requested: Vec::new(),
        add_ons: Vec::new(),
        skip_initial_migration: false,
    }
}

#[test]
fn summary_lists_answers_files_commands_and_the_first_url() {
    let files = files(&BLANK_LIKE);
    let lines = texts(summary(&plan(), Some(&files), 3000).lines());
    assert_eq!(lines[0], "Project      my_app  in ./my_app");
    assert_eq!(
        lines[1],
        "Blueprint    Blank · Minimal HTMX page or JSON API; Nexus CMS not included"
    );
    assert_eq!(lines[2], "Application  Full-stack web app (html! + HTMX)");
    assert_eq!(lines[3], "Database     SQLite");
    assert_eq!(lines[4], "Features     none");
    assert_eq!(lines[5], "Files        16 files will be written");
    assert_eq!(lines[6], "             my_app/  16 files");
    let tail = &lines[lines.len() - 4..];
    assert_eq!(
        tail,
        [
            "Runs         cargo run -q -- db:migrate",
            "             first build and initial migration; a cold build can take minutes",
            "Then         cd my_app && cargo rullst dev",
            "             open http://127.0.0.1:3000",
        ]
    );
}

#[test]
fn summary_reflects_api_database_free_features_and_unavailable_previews() {
    let mut lean = plan();
    lean.name = "../apps/lean-api".to_string();
    lean.api = true;
    lean.database = Database::None;
    lean.ai = true;
    lean.buildah = true;
    lean.add_ons = vec![PolyglotIntegration::Qdrant];
    let lines = texts(summary(&lean, None, 8080).lines());
    let text = lines.join("\n");
    assert!(
        text.contains("Application  JSON API (headless, no HTML)"),
        "{text}"
    );
    assert!(text.contains("Database     none"), "{text}");
    assert!(
        text.contains("Features     AI features, Dockerfile, Qdrant, Buildah script"),
        "{text}"
    );
    assert!(text.contains("Files        preview unavailable"), "{text}");
    assert!(
        text.contains("Runs         no commands; files only"),
        "{text}"
    );
    assert!(
        text.contains("Project      lean-api  in ../apps/lean-api"),
        "{text}"
    );
    assert!(
        text.contains("cd ../apps/lean-api && cargo rullst dev"),
        "{text}"
    );
    assert!(text.contains("curl http://127.0.0.1:8080/"), "{text}");

    let mut edge = plan();
    edge.database = Database::Provider("Turso");
    edge.requested = vec![PolyglotIntegration::Turso];
    let text = texts(summary(&edge, None, 3000).lines()).join("\n");
    assert!(text.contains("Database     Turso / libSQL"), "{text}");
    assert!(
        text.contains("Features     none"),
        "the primary is not an add-on: {text}"
    );
}
