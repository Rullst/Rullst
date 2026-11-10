use super::*;

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("src/controllers")).unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        r#"[package]
name = "context-fixture"
version = "0.1.0"
[dependencies]
rullst = { version = "13", git = "https://token-must-not-appear@example.invalid/private" }
serde = { version = "1", features = ["derive"] }
[features]
zebra = []
alpha = []
[target.'cfg(unix)'.dependencies]
libc = "0.2"
[package.metadata]
secret = "manifest-value-must-not-appear"
"#,
    )
    .unwrap();
    fs::write(root.path().join("Rullst.toml"), "[app]\nname = 'config-value-must-not-appear'\n[database]\nurl = 'private-database-value'\n").unwrap();
    fs::write(root.path().join(".env.example"), "# comment-must-not-appear\nAPP_KEY=key-value-must-not-appear\nexport DATABASE_URL=database-value-must-not-appear\n").unwrap();
    fs::write(
        root.path().join(".env"),
        "LIVE_SECRET=live-secret-must-not-appear",
    )
    .unwrap();
    fs::write(
        root.path().join("src/main.rs"),
        "fn main() { /* source-body-must-not-appear */ }\n",
    )
    .unwrap();
    fs::write(
        root.path().join("src/controllers/account.rs"),
        "pub async fn account() {}\n",
    )
    .unwrap();
    root
}

fn read(root: &Path, file: &str) -> String {
    fs::read_to_string(root.join(file)).unwrap()
}
fn map(root: &Path) -> serde_json::Value {
    serde_json::from_str(&read(root, ".rullst/context-map.json")).unwrap()
}

#[test]
fn inventory_is_deterministic_and_does_not_embed_private_values() {
    let root = fixture();
    generate(root.path()).unwrap();
    let initial = read(root.path(), ".llms.txt");
    for output in [&initial, &read(root.path(), ".rullst/context-map.json")] {
        for private in [
            "must-not-appear",
            "private-database-value",
            "example.invalid",
            root.path().to_str().unwrap(),
        ] {
            assert!(!output.contains(private), "private data in inventory");
        }
    }
    let value = map(root.path());
    assert_eq!(value["schema"], SCHEMA);
    assert_eq!(
        value["dependency_features"]["serde"],
        serde_json::json!(["derive"])
    );
    assert_eq!(value["project"], "context-fixture");
    assert_eq!(
        value["dependencies"],
        serde_json::json!(["libc", "rullst", "serde"])
    );
    assert_eq!(
        value["declared_features"],
        serde_json::json!(["alpha", "zebra"])
    );
    assert_eq!(
        value["configuration_keys"][".env.example"],
        serde_json::json!(["APP_KEY", "DATABASE_URL"])
    );
    assert_eq!(
        value["configuration_keys"]["Rullst.toml"],
        serde_json::json!(["app", "app.name", "database", "database.url"])
    );
    assert_eq!(value["files"].as_array().unwrap().len(), 2);
    assert!(read(root.path(), "AGENTS.md").contains("tenant membership"));
    check_ai_context(Some(root.path())).unwrap();
    generate(root.path()).unwrap();
    assert_eq!(initial, read(root.path(), ".llms.txt"));
    // Values are deliberately neither copied nor hashed; only declared key names matter.
    fs::write(
        root.path().join(".env.example"),
        "APP_KEY=changed\nDATABASE_URL=changed\n",
    )
    .unwrap();
    check_ai_context(Some(root.path())).unwrap();
}

#[test]
fn changes_to_code_manifest_keys_and_output_are_detected_without_writes() {
    let root = fixture();
    assert!(matches!(
        check_ai_context(Some(root.path())),
        Err(ContextError::Stale)
    ));
    generate(root.path()).unwrap();
    let initial = read(root.path(), ".llms.txt");
    fs::write(
        root.path().join("src/controllers/account.rs"),
        "pub async fn renamed() {}\n",
    )
    .unwrap();
    assert!(matches!(
        check_ai_context(Some(root.path())),
        Err(ContextError::Stale)
    ));
    assert_eq!(initial, read(root.path(), ".llms.txt"));
    generate(root.path()).unwrap();
    fs::write(root.path().join(".env.example"), "NEW_SETTING=private\n").unwrap();
    assert!(matches!(
        check_ai_context(Some(root.path())),
        Err(ContextError::Stale)
    ));
    generate(root.path()).unwrap();
    fs::write(root.path().join(".llms.txt"), "manually replaced output").unwrap();
    assert!(matches!(
        check_ai_context(Some(root.path())),
        Err(ContextError::Stale)
    ));
    assert!(matches!(
        generate(root.path()),
        Err(ContextError::Refused { path, .. }) if path == ".llms.txt"
    ));
    assert_eq!(read(root.path(), ".llms.txt"), "manually replaced output");
}

#[test]
fn instructions_are_user_owned_and_recognized_legacy_output_migrates() {
    let root = fixture();
    fs::write(
        root.path().join("AGENTS.md"),
        "project-specific rules; preserve exactly",
    )
    .unwrap();
    fs::write(
        root.path().join(".llms.txt"),
        format!("{LEGACY_MARKER} legacy generated source body"),
    )
    .unwrap();
    generate(root.path()).unwrap();
    assert_eq!(
        read(root.path(), "AGENTS.md"),
        "project-specific rules; preserve exactly"
    );
    assert!(!read(root.path(), ".llms.txt").contains("legacy generated source body"));
    fs::write(root.path().join("AGENTS.md"), "new custom rules").unwrap();
    generate(root.path()).unwrap();
    assert_eq!(read(root.path(), "AGENTS.md"), "new custom rules");
}

#[test]
fn all_output_conflicts_are_found_before_replacing_any_file() {
    let root = fixture();
    generate(root.path()).unwrap();
    let initial = read(root.path(), ".llms.txt");
    fs::write(root.path().join("src/main.rs"), "changed input").unwrap();
    fs::write(
        root.path().join(".rullst/context-map.json"),
        "{\"user\":\"owned\"}",
    )
    .unwrap();
    assert!(matches!(
        generate(root.path()),
        Err(ContextError::Refused { path, .. }) if path == ".rullst/context-map.json"
    ));
    assert_eq!(initial, read(root.path(), ".llms.txt"));
    assert_eq!(
        read(root.path(), ".rullst/context-map.json"),
        "{\"user\":\"owned\"}"
    );
}

#[test]
fn excluded_files_are_not_embedded_and_input_budgets_fail_closed() {
    let root = fixture();
    for name in [
        "src/.hidden/key.rs",
        "src/vendor/key.rs",
        "src/target/key.rs",
        "src/node_modules/key.rs",
    ] {
        fs::create_dir_all(root.path().join(name).parent().unwrap()).unwrap();
        fs::write(root.path().join(name), "excluded source marker").unwrap();
    }
    fs::write(root.path().join("src/private.key"), "excluded key marker").unwrap();
    generate(root.path()).unwrap();
    assert_eq!(map(root.path())["files"].as_array().unwrap().len(), 2);
    assert_eq!(map(root.path())["excluded_entries"], 5);
    let before = read(root.path(), ".llms.txt");
    fs::write(
        root.path().join("src/large.rs"),
        vec![b'x'; 1024 * 1024 + 1],
    )
    .unwrap();
    assert!(matches!(generate(root.path()), Err(ContextError::Limit)));
    assert_eq!(before, read(root.path(), ".llms.txt"));
    fs::remove_file(root.path().join("src/large.rs")).unwrap();
    let mut directory = root.path().join("src");
    for _ in 0..18 {
        directory = directory.join("deep");
    }
    fs::create_dir_all(directory).unwrap();
    assert!(matches!(generate(root.path()), Err(ContextError::Limit)));
}

#[test]
fn malformed_or_oversized_configuration_never_echoes_its_contents() {
    for (file, source) in [
        ("Cargo.toml", "[package\nsecret-must-not-leak"),
        ("Rullst.toml", "[broken\nsecret-must-not-leak"),
        (".env.example", "INVALID KEY=secret-must-not-leak"),
    ] {
        let root = fixture();
        fs::write(root.path().join(file), source).unwrap();
        let error = generate(root.path()).unwrap_err();
        assert!(!format!("{error:?} {error}").contains("secret-must-not-leak"));
        assert!(!root.path().join(".llms.txt").exists());
    }
    let root = fixture();
    fs::write(root.path().join(".env.example"), vec![b'x'; 256 * 1024 + 1]).unwrap();
    assert!(matches!(generate(root.path()), Err(ContextError::Limit)));
}

#[test]
fn source_file_count_and_aggregate_bytes_are_bounded() {
    let root = fixture();
    for n in 0..512 {
        fs::write(root.path().join(format!("src/f{n}.rs")), "").unwrap();
    }
    assert!(matches!(generate(root.path()), Err(ContextError::Limit)));
    let root = fixture();
    for n in 0..9 {
        fs::write(
            root.path().join(format!("src/f{n}.rs")),
            vec![b'x'; 1024 * 1024],
        )
        .unwrap();
    }
    assert!(matches!(generate(root.path()), Err(ContextError::Limit)));
}

#[cfg(unix)]
#[test]
fn linked_sources_are_excluded_and_linked_outputs_or_config_are_rejected() {
    use std::os::unix::fs::symlink;
    let root = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("private.rs"), "outside secret marker").unwrap();
    symlink(
        outside.path().join("private.rs"),
        root.path().join("src/linked.rs"),
    )
    .unwrap();
    symlink(outside.path(), root.path().join("src/linked_dir")).unwrap();
    generate(root.path()).unwrap();
    assert_eq!(map(root.path())["excluded_entries"], 2);
    assert_eq!(map(root.path())["files"].as_array().unwrap().len(), 2);
    fs::remove_file(root.path().join(".llms.txt")).unwrap();
    symlink(
        outside.path().join("private.rs"),
        root.path().join(".llms.txt"),
    )
    .unwrap();
    assert!(matches!(
        generate(root.path()),
        Err(ContextError::Refused { path, reason: "it is a link or lies below one" })
            if path == ".llms.txt"
    ));
    assert_eq!(read(outside.path(), "private.rs"), "outside secret marker");
    fs::remove_file(root.path().join(".llms.txt")).unwrap();
    fs::remove_file(root.path().join(".env.example")).unwrap();
    symlink(
        outside.path().join("private.rs"),
        root.path().join(".env.example"),
    )
    .unwrap();
    assert!(matches!(
        generate(root.path()),
        Err(ContextError::UnsafePath)
    ));
}

#[test]
fn workspace_metadata_is_not_mistaken_for_dependencies() {
    let data = metadata::manifest(
        br#"[workspace]
members = []
[workspace.dependencies]
rullst = "13"
[workspace.metadata]
secret = "do-not-render"
dependencies = "this-is-not-a-cargo-dependency-table"
"#,
    )
    .unwrap();
    assert_eq!(data.project, None);
    assert_eq!(data.dependencies, ["rullst"]);
}

#[test]
fn dependency_requirements_participate_in_freshness() {
    let root = fixture();
    generate(root.path()).unwrap();
    assert_eq!(
        map(root.path())["dependency_requirements"]["rullst"],
        serde_json::json!(["^13"])
    );
    let path = root.path().join("Cargo.toml");
    let source = fs::read_to_string(&path)
        .unwrap()
        .replace("version = \"13\"", "version = \"12\"");
    fs::write(path, source).unwrap();
    assert!(matches!(
        check_ai_context(Some(root.path())),
        Err(ContextError::Stale)
    ));
}

#[test]
fn dotenv_values_are_not_reinterpreted_as_keys_or_interpolated() {
    for value in [
        b"KEY=\"first\nsecret_value=second\n\"\n".as_slice(),
        b"KEY='first\nsecret_value=second\n'\n",
        b"KEY=first\\\nsecond",
    ] {
        assert!(matches!(
            metadata::env_keys(value),
            Err(ContextError::Configuration)
        ));
    }
    assert_eq!(metadata::env_keys(b"KEY=\"quoted = value # text\" # comment\nOTHER='single = value'\nREF=${PRIVATE_ENV}\n").unwrap(), ["KEY", "OTHER", "REF"]);
}

fn crlf(root: &Path, file: &str) {
    let text = read(root, file).replace('\n', "\r\n");
    fs::write(root.join(file), text).unwrap();
}

/// Git for Windows checks the outputs out with CRLF (`core.autocrlf=true`):
/// they are still recognized, current and left as they are.
#[test]
fn outputs_checked_out_with_crlf_are_recognized_and_current() {
    let root = fixture();
    generate(root.path()).unwrap();
    for file in [".llms.txt", ".rullst/context-map.json"] {
        crlf(root.path(), file);
    }
    let checked_out = read(root.path(), ".llms.txt");
    assert!(checked_out.starts_with(&TEXT_MARKER.replace('\n', "\r\n")));
    check_ai_context(Some(root.path())).unwrap();
    assert!(!generate(root.path()).unwrap());
    assert_eq!(read(root.path(), ".llms.txt"), checked_out);

    // A stale CRLF output is replaced with the current LF inventory.
    fs::write(
        root.path().join("src/main.rs"),
        "fn main() { changed(); }\n",
    )
    .unwrap();
    assert!(matches!(
        check_ai_context(Some(root.path())),
        Err(ContextError::Stale)
    ));
    generate(root.path()).unwrap();
    assert!(!read(root.path(), ".llms.txt").contains('\r'));
    check_ai_context(Some(root.path())).unwrap();

    // The legacy header is recognized with CRLF too.
    fs::write(
        root.path().join(".llms.txt"),
        format!("{LEGACY_MARKER} legacy body").replace('\n', "\r\n"),
    )
    .unwrap();
    generate(root.path()).unwrap();
    assert!(read(root.path(), ".llms.txt").starts_with(TEXT_MARKER));
}

#[test]
fn a_refusal_names_the_file_and_the_reason() {
    let root = fixture();
    fs::write(root.path().join(".llms.txt"), "my own notes\n").unwrap();
    let error = generate(root.path()).unwrap_err().to_string();
    assert_eq!(
        error,
        "project context refuses `.llms.txt`: it does not start with the generated-inventory marker, so it may be your own file; review it, move it aside and run the command again"
    );
    fs::remove_file(root.path().join(".llms.txt")).unwrap();
    fs::create_dir_all(root.path().join(".rullst")).unwrap();
    fs::write(root.path().join(".rullst/context-map.json"), "{}").unwrap();
    assert!(
        generate(root.path())
            .unwrap_err()
            .to_string()
            .starts_with("project context refuses `.rullst/context-map.json`: it is not a `rullst.project-context.v1` map")
    );
    fs::remove_file(root.path().join(".rullst/context-map.json")).unwrap();
    fs::write(root.path().join(".llms.txt"), [0xff, 0xfe]).unwrap();
    assert_eq!(
        generate(root.path()).unwrap_err().to_string(),
        "project context refuses `.llms.txt`: it is not UTF-8 text"
    );
    fs::remove_file(root.path().join(".llms.txt")).unwrap();
    fs::create_dir(root.path().join(".llms.txt")).unwrap();
    assert_eq!(
        generate(root.path()).unwrap_err().to_string(),
        "project context refuses `.llms.txt`: it is not a regular file"
    );
}

const ATTRIBUTES: &str = "# cargo rullst generate:ai-context output: LF on every checkout, so --check matches\n.llms.txt text eol=lf\n.rullst/context-map.json text eol=lf\n";

/// The outputs' LF entries are added once, after any existing content;
/// nothing is written when the outputs are already current.
#[test]
fn gitattributes_keeps_the_outputs_lf_without_replacing_user_lines() {
    let root = fixture();
    assert!(generate(root.path()).unwrap());
    assert_eq!(read(root.path(), ".gitattributes"), ATTRIBUTES);
    fs::write(root.path().join("src/main.rs"), "fn main() { a(); }\n").unwrap();
    assert!(!generate(root.path()).unwrap());
    assert_eq!(read(root.path(), ".gitattributes"), ATTRIBUTES);

    // Current outputs write nothing, so no entry comes back.
    fs::remove_file(root.path().join(".gitattributes")).unwrap();
    assert!(!generate(root.path()).unwrap());
    assert!(!root.path().join(".gitattributes").exists());

    // Existing lines stay; a file without a final newline gets one.
    fs::write(root.path().join(".gitattributes"), "*.png binary").unwrap();
    fs::write(root.path().join("src/main.rs"), "fn main() { b(); }\n").unwrap();
    assert!(generate(root.path()).unwrap());
    assert_eq!(
        read(root.path(), ".gitattributes"),
        format!("*.png binary\n\n{ATTRIBUTES}")
    );

    // A file the user already names keeps that choice; CRLF files get CRLF.
    fs::write(
        root.path().join(".gitattributes"),
        "*.png binary\r\n/.llms.txt -text\r\n",
    )
    .unwrap();
    fs::write(root.path().join("src/main.rs"), "fn main() { c(); }\n").unwrap();
    assert!(generate(root.path()).unwrap());
    assert_eq!(
        read(root.path(), ".gitattributes"),
        "*.png binary\r\n/.llms.txt -text\r\n\r\n# cargo rullst generate:ai-context output: LF on every checkout, so --check matches\r\n.rullst/context-map.json text eol=lf\r\n"
    );
    assert_eq!(
        attributes_note(true),
        "\n.gitattributes now keeps both files LF on every checkout."
    );
    assert_eq!(attributes_note(false), "");
}

#[cfg(unix)]
#[test]
fn a_linked_gitattributes_is_refused_before_any_write() {
    let root = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("attributes"), "*.rs text\n").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("attributes"),
        root.path().join(".gitattributes"),
    )
    .unwrap();
    assert!(matches!(
        generate(root.path()),
        Err(ContextError::Refused { path, .. }) if path == ".gitattributes"
    ));
    assert!(!root.path().join(".llms.txt").exists());
    assert_eq!(read(outside.path(), "attributes"), "*.rs text\n");
}
