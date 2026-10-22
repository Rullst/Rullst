//! `cargo rullst make:pwa` on generated starters: the command line, the
//! idempotent second run, refusals, and the routes of a compiled Blank
//! application answering with the headers a service worker needs.
use cargo_rullst::{
    blueprints::{self, BLANK_BLUEPRINT_ID},
    generators::project::cargo_toml::build_cargo_toml,
};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Output},
};

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn materialize(root: &Path, workspace: &Path, api: bool) {
    fs::create_dir_all(root).unwrap();
    let mut manifest = build_cargo_toml(
        "pwa-consumer",
        false,
        false,
        "Sqlite",
        &[],
        false,
        false,
        BLANK_BLUEPRINT_ID,
        "Zero-Bundle HTMX",
        workspace,
    )
    .unwrap();
    manifest.push_str("\n[profile.test]\ndebug = 0\nincremental = false\n");
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    fs::copy(workspace.join("Cargo.lock"), root.join("Cargo.lock")).unwrap();
    blueprints::apply(
        BLANK_BLUEPRINT_ID,
        root,
        "pwa-consumer",
        "pwa_consumer",
        api,
        false,
        false,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .unwrap();
}

fn make_pwa(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rullst"))
        .current_dir(root)
        .args(["make:pwa", "--name", "Field Notes", "--lang", "en"])
        .output()
        .unwrap()
}

/// Every file of the project except Cargo's own outputs.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.ends_with("target") {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else {
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(name, fs::read(&path).unwrap());
            }
        }
    }
    files
}

#[test]
fn make_pwa_is_idempotent_refuses_edits_and_the_api_starter() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("app");
    materialize(&project, workspace, false);
    let first = success(make_pwa(&project));
    let stdout = String::from_utf8_lossy(&first.stdout);
    assert!(
        stdout.starts_with("Progressive Web App added.\n"),
        "{stdout}"
    );
    assert!(stdout.contains("cargo rullst dev"), "{stdout}");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(project.join("static/manifest.webmanifest")).unwrap())
            .unwrap();
    assert_eq!(manifest["name"], "Field Notes");
    assert_eq!(manifest["short_name"], "Field Notes");
    assert_eq!(manifest["start_url"], "/");
    assert_eq!(manifest["scope"], "/");
    assert_eq!(manifest["id"], "/");
    assert_eq!(manifest["display"], "standalone");
    for icon in manifest["icons"].as_array().unwrap() {
        let path = icon["src"].as_str().unwrap().trim_start_matches('/');
        assert!(project.join(path).is_file(), "{path}");
    }

    let wired = snapshot(&project);
    let second = success(make_pwa(&project));
    assert!(
        String::from_utf8_lossy(&second.stdout)
            .starts_with("The PWA files and wiring are already in place; nothing changed.")
    );
    assert_eq!(snapshot(&project), wired);

    fs::write(project.join("static/pwa.js"), "// application-owned\n").unwrap();
    let edited = snapshot(&project);
    let refused = make_pwa(&project);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("static/pwa.js already exists with other content")
    );
    assert_eq!(snapshot(&project), edited);

    let api = temporary.path().join("api");
    materialize(&api, workspace, true);
    let before = snapshot(&api);
    let refused = make_pwa(&api);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr)
            .contains("the JSON API starter serves no pages to install")
    );
    assert_eq!(snapshot(&api), before);
}

#[test]
fn a_generated_blank_application_serves_the_pwa_routes() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("app");
    materialize(&project, workspace, false);
    let format = |check: bool| {
        let mut command = Command::new("cargo");
        command.current_dir(&project).args(["fmt", "--all"]);
        if check {
            command.args(["--", "--check"]);
        }
        success(command.output().unwrap())
    };
    format(false);
    success(make_pwa(&project));
    // The wiring keeps rustfmt's layout of a formatted starter.
    format(true);
    fs::write(
        project.join("src/pwa_contract.rs"),
        include_str!("fixtures/pwa_contract.rs"),
    )
    .unwrap();
    let mut main = fs::read_to_string(project.join("src/main.rs")).unwrap();
    main.push_str("\n#[cfg(test)]\nmod pwa_contract;\n");
    fs::write(project.join("src/main.rs"), main).unwrap();
    success(
        Command::new("cargo")
            .current_dir(&project)
            .args([
                "test",
                "--offline",
                "-j",
                "2",
                "pwa_contract",
                "--",
                "--nocapture",
            ])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .output()
            .unwrap(),
    );
    // All generated Cargo/test child processes have exited. Remove only this
    // suite's package artifacts; retain shared framework/dependency builds.
    success(
        Command::new("cargo")
            .current_dir(&project)
            .args(["clean", "--package", "pwa-consumer"])
            .env("CARGO_TARGET_DIR", workspace.join("target"))
            .env("CARGO_NET_OFFLINE", "true")
            .output()
            .unwrap(),
    );
}
