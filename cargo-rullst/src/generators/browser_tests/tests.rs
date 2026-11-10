use super::*;
use crate::generators::add::{self, Failure};
use crate::generators::project::ci::assert_pinned_like_rullst;
use std::fs;

const MANIFEST: &str = "[package]\nname = \"shop\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = { version = \"13.0.0\", default-features = false }\n";

fn file(path: &str) -> &'static str {
    FILES
        .iter()
        .find_map(|(candidate, content)| (*candidate == path).then_some(*content))
        .unwrap_or_else(|| panic!("no generated {path}"))
}

/// The Blank starter's generated `src/security_tests.rs`.
fn starter_security_tests() -> String {
    crate::blueprints::blank::file_manifest(
        "shop",
        "shop",
        false,
        false,
        true,
        "Active Record",
        "Zero-Bundle HTMX",
    )
    .into_iter()
    .find_map(|(path, source)| (path == "src/security_tests.rs").then_some(source))
    .expect("the starter's security tests")
}

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("temporary project");
    for (path, content) in files {
        let target = root.path().join(path);
        fs::create_dir_all(target.parent().expect("parent")).expect("directories");
        fs::write(target, content).expect("project file");
    }
    root
}

#[test]
fn the_pins_are_rullsts_own_playwright_and_axe_core() {
    let package: serde_json::Value =
        serde_json::from_str(file("tests/browser/package.json")).expect("package.json");
    assert_eq!(package["private"], true);
    assert_eq!(package["devDependencies"]["playwright"], "1.64.1");
    assert_eq!(package["devDependencies"]["axe-core"], "4.14.0");
    let lock: serde_json::Value =
        serde_json::from_str(file("tests/browser/package-lock.json")).expect("lockfile");
    assert_eq!(lock["lockfileVersion"], 3);
    assert_eq!(lock["name"], package["name"]);
    let packages = lock["packages"].as_object().expect("packages");
    for (name, version) in [
        ("node_modules/playwright", "1.64.1"),
        ("node_modules/playwright-core", "1.64.1"),
        ("node_modules/axe-core", "4.14.0"),
    ] {
        assert_eq!(packages[name]["version"], version, "{name}");
        let integrity = packages[name]["integrity"].as_str().expect("integrity");
        assert!(integrity.starts_with("sha512-"), "{name}");
        assert!(
            packages[name]["resolved"]
                .as_str()
                .is_some_and(|url| url.starts_with("https://registry.npmjs.org/")),
            "{name}"
        );
    }
    // Same bytes as Rullst's own starter-browser lock, apart from the name.
    let own = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/starter-browsers/package-lock.json");
    if let Ok(own) = fs::read_to_string(own) {
        assert_eq!(
            own.replace("rullst-starter-browser-checks", "browser-tests"),
            file("tests/browser/package-lock.json")
        );
    }
}

#[test]
fn the_workflow_is_pinned_and_runs_the_page_checks() {
    let workflow = file(".github/workflows/browser-tests.yml");
    assert_pinned_like_rullst("browser-tests.yml", workflow);
    assert!(workflow.contains("\npermissions:\n  contents: read\n"));
    assert_eq!(workflow.matches("permissions:").count(), 1);
    for step in [
        "npm ci --prefix tests/browser --ignore-scripts --no-audit --no-fund",
        "install --with-deps chromium firefox webkit",
        "cp .env.example .env",
        "if [ -d src/migrations ]; then",
        "--retry-all-errors",
        "run: node tests/browser/pages.mjs --base-url http://127.0.0.1:3000",
        "persist-credentials: false",
    ] {
        assert!(workflow.contains(step), "{step}");
    }
    assert!(!workflow.contains('\t') && !workflow.contains('\r'));
}

#[test]
fn the_script_reads_the_generated_page_list() {
    let script = file("tests/browser/pages.mjs");
    let between = |start: &str, end: &str| {
        let (_, rest) = script.split_once(start).expect(start);
        rest.split_once(end).expect(end).0.to_owned()
    };
    // The JavaScript patterns, compiled as Rust regexes, find the starter's list.
    let list = regex::Regex::new(&between("source.match(/", "/);")).expect("list pattern");
    let entry = regex::Regex::new(&between("code.matchAll(/", "/g)")).expect("entry pattern");
    let source = starter_security_tests();
    assert!(source.contains(PAGE_LIST));
    let captured = list.captures(&source).expect("PAGES list")[1].to_owned();
    let pages = entry
        .captures_iter(&captured)
        .map(|entry| (entry[1].to_owned(), entry[2].parse::<u16>().expect("status")))
        .collect::<Vec<_>>();
    assert_eq!(pages, [("/".to_owned(), 200), ("/health".to_owned(), 200)]);
    for check in [
        "\"serious\", \"critical\"",
        "securitypolicyviolation",
        "message.type() === \"error\"",
        "pageerror",
        "response.status() >= 400",
        "colorScheme: \"dark\"",
        "prefers-color-scheme",
        "chromium,firefox,webkit",
        "resolve(HERE, \"..\", \"..\", \"src\", \"security_tests.rs\")",
    ] {
        assert!(script.contains(check), "{check}");
    }
    // The application's own policy applies in every browser.
    assert!(!script.contains("bypassCSP: true"));
}

#[test]
fn files_are_created_once_and_never_overwritten() {
    let source = starter_security_tests();
    let root = project(&[("Cargo.toml", MANIFEST), ("src/security_tests.rs", &source)]);
    let first = execute(root.path(), false, Style::PLAIN).expect("first run");
    for (path, content) in FILES {
        assert_eq!(fs::read_to_string(root.path().join(path)).expect(path), content);
        assert!(first.contains(&format!("✓ {path:<36} created")), "{first}");
    }
    assert!(!first.contains("declares no"), "{first}");
    assert!(first.contains("node tests/browser/pages.mjs"), "{first}");

    let second = execute(root.path(), false, Style::PLAIN).expect("second run");
    assert_eq!(second.matches(" unchanged").count(), FILES.len(), "{second}");

    let script = root.path().join("tests/browser/pages.mjs");
    fs::write(&script, "// my version\n").expect("edit");
    let third = execute(root.path(), false, Style::PLAIN).expect("third run");
    assert_eq!(fs::read_to_string(&script).expect("kept"), "// my version\n");
    assert!(
        third.contains("! tests/browser/pages.mjs") && third.contains("kept: it differs"),
        "{third}"
    );
}

#[test]
fn a_dry_run_writes_nothing() {
    let root = project(&[("Cargo.toml", MANIFEST)]);
    let report = execute(root.path(), true, Style::PLAIN).expect("dry run");
    assert!(!root.path().join("tests").exists());
    assert!(!root.path().join(".github").exists());
    assert_eq!(report.matches("would create").count(), FILES.len(), "{report}");
    assert!(report.contains("Dry run: nothing was written."), "{report}");
    // Without a page list the report says what the checks fall back to.
    assert!(report.contains("only `/` is visited"), "{report}");
}

#[test]
fn only_rullst_projects_are_changed() {
    let empty = project(&[]);
    assert_eq!(plan(empty.path()), Err(Error::NotAProject));
    let other = project(&[("Cargo.toml", "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\n")]);
    assert_eq!(plan(other.path()), Err(Error::NotAProject));
    let broken = project(&[("Cargo.toml", "[package\n")]);
    assert_eq!(plan(broken.path()), Err(Error::InvalidManifest));
    assert!(!other.path().join("tests").exists());
}

#[test]
fn add_dispatches_the_capability_and_lists_it() {
    let root = project(&[("Cargo.toml", MANIFEST)]);
    let report = add::execute(root.path(), NAME, false, Style::PLAIN).expect("add browser-tests");
    assert!(report.contains("cargo rullst add browser-tests"), "{report}");
    assert!(root.path().join("tests/browser/pages.mjs").is_file());
    let empty = project(&[]);
    assert_eq!(
        add::execute(empty.path(), NAME, false, Style::PLAIN),
        Err(Failure::Project)
    );
    let broken = project(&[("Cargo.toml", "[package\n")]);
    assert!(matches!(
        add::execute(broken.path(), NAME, false, Style::PLAIN),
        Err(Failure::Add(_))
    ));

    let matches = add::command()
        .try_get_matches_from(["add", NAME, "--dry-run"])
        .expect("the CLI accepts browser-tests");
    assert_eq!(
        matches.get_one::<String>("capability").map(String::as_str),
        Some(NAME)
    );
    let help = add::command().render_long_help().to_string();
    assert!(help.contains(NAME) && help.contains(SUMMARY), "{help}");
}
