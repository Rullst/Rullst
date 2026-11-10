#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::generators::frontend::tailwind::{Platform, binary_path};
use crate::generators::frontend::tailwind_release::{self, Asset};
use crate::generators::frontend::{FetchError, Fixture, TAILWIND_INPUT, TAILWIND_OUTPUT, docker};
use std::fs;

pub(super) const MANIFEST: &str = "[package]\nname = \"shop\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nrullst = \"13.0.0-alpha.5\"\n";

pub(super) const LAYOUT: &str = r#"use rullst::html;
use rullst::html::Markup;

pub fn page(title: &str, main: Markup) -> Markup {
    html! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>{title}</title>
            </head>
            <body>{main}</body>
        </html>
    }
}
"#;

const RENDER_PAGE_MAIN: &str = r#"use rullst::htmx::{HtmxRequest, render_page_with_lang};
async fn home(htmx: HtmxRequest) -> impl rullst::response::IntoResponse {
    render_page_with_lang(&htmx, "en", "Welcome", rullst::html! { <main>"hi"</main> })
}
fn main() {}
"#;

pub(super) fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Cargo.toml"), MANIFEST).unwrap();
    for (path, text) in files {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    root
}

pub(super) fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path)).unwrap()
}

pub(super) const CLI: &[u8] =
    b"#!/bin/sh\nmkdir -p static && printf '/*! tailwindcss fixture */' > static/app.css\n";
const CLI_SHA256: &str = "d355dbd6f0726e91a07f76456ca9f90efa28bc0b2d5cf7a1668904537c9913a2";
pub(super) const ASSET: Asset = Asset {
    platform: Platform::LinuxX64,
    name: "tailwindcss-fixture",
    sha256: CLI_SHA256,
};

/// A per-user cache and a fixture "network".
pub(super) struct Harness {
    pub(super) cache: tempfile::TempDir,
    pub(super) fetcher: Fixture,
    /// Runs the first build (the fixture CLI is a shell script).
    pub(super) build: bool,
}

impl Harness {
    pub(super) fn serving(body: &[u8]) -> Self {
        Self {
            cache: tempfile::tempdir().unwrap(),
            fetcher: Fixture::serving(body),
            build: cfg!(unix),
        }
    }

    pub(super) fn run(
        &self,
        root: &Path,
        kit: &str,
        dry_run: bool,
        alpine_integrity: &str,
    ) -> Result<String, Failure> {
        let toolchain = Toolchain {
            fetcher: &self.fetcher,
            tailwind: Some(&ASSET),
            cache_base: Some(self.cache.path().to_path_buf()),
            build: self.build,
            alpine_integrity,
        };
        execute(root, find(kit).unwrap(), dry_run, Style::PLAIN, &toolchain)
    }

    pub(super) fn tailwind(&self, root: &Path, dry_run: bool) -> Result<String, Failure> {
        self.run(root, "tailwind", dry_run, "unused")
    }
}

pub(super) fn frontend_error(result: Result<String, Failure>) -> FrontendError {
    match result {
        Err(Failure::Frontend(error)) => error,
        Err(Failure::Project) => panic!("not a project"),
        Ok(report) => panic!("unexpected success: {report}"),
    }
}

fn generated_dockerfile(root: &Path) -> String {
    crate::generators::project::generate_docker_files(root, "shop", None, None).unwrap();
    read(root, "Dockerfile")
}

#[test]
fn the_kit_table_matches_the_command_values() {
    assert_eq!(
        NAMES.to_vec(),
        KITS.iter().map(|kit| kit.name).collect::<Vec<_>>()
    );
    assert_eq!(KITS[0].name, "tailwind");
    assert_eq!(KITS[1].name, "alpine");
    for kit in &KITS {
        assert_eq!(find(kit.name), Some(kit));
        assert!((1..=3).contains(&kit.hints.len()), "{}", kit.name);
    }
    assert_eq!(find("mail"), None);
}

#[test]
fn tailwind_dry_run_downloads_and_writes_nothing() {
    let root = project(&[("src/pages/layout.rs", LAYOUT), (".gitignore", "/target\n")]);
    let harness = Harness::serving(CLI);
    let report = harness.tailwind(root.path(), true).unwrap();
    assert_eq!(harness.fetcher.calls.get(), 0);
    assert!(!root.path().join(TAILWIND_INPUT).exists());
    assert_eq!(read(root.path(), ".gitignore"), "/target\n");
    assert_eq!(read(root.path(), "src/pages/layout.rs"), LAYOUT);
    for expected in [
        "◆ cargo rullst add tailwind",
        &format!("~ tailwindcss v{}", tailwind_release::VERSION),
        "would download the pinned CLI into the user cache and verify its SHA-256",
        "~ assets/css/app.css",
        "would create the input",
        "~ src/pages/layout.rs",
        "+++ assets/css/app.css (new file)",
        "+++ .gitignore (planned)",
        "Dry run: nothing was downloaded or written.",
    ] {
        assert!(report.contains(expected), "{expected}\n{report}");
    }
}

#[test]
fn tailwind_writes_the_input_links_the_layout_and_is_idempotent() {
    let root = project(&[
        ("src/pages/layout.rs", LAYOUT),
        (".gitignore", "/target"),
        ("static/rullst.css", "body{}"),
    ]);
    let dockerfile = generated_dockerfile(root.path());
    let harness = Harness::serving(CLI);
    let report = harness.tailwind(root.path(), false).unwrap();
    assert_eq!(harness.fetcher.calls.get(), 1);
    assert_eq!(read(root.path(), TAILWIND_INPUT), tailwind::INPUT_CSS);
    assert!(tailwind::INPUT_CSS.contains("@import \"tailwindcss\" source(none);\n"));
    assert!(tailwind::INPUT_CSS.contains("@source \"../../src/**/*.rs\";\n"));
    assert!(crate::generators::frontend::has_managed_tailwind(
        root.path()
    ));
    assert_eq!(
        read(root.path(), ".gitignore"),
        "/target\n\n# Built from assets/css/app.css by the Tailwind CSS CLI (`cargo rullst add tailwind`)\n/static/app.css\n"
    );
    let layout = read(root.path(), "src/pages/layout.rs");
    assert!(
        layout.contains(
            "                <title>{title}</title>\n                <link rel=\"stylesheet\" href={rullst::assets::url(\"app.css\")} />\n            </head>\n"
        ),
        "{layout}"
    );
    let edited = read(root.path(), "Dockerfile");
    assert_eq!(Some(edited.clone()), docker::add_to(&dockerfile));
    let cached = binary_path(harness.cache.path(), &ASSET);
    assert_eq!(fs::read(&cached).unwrap(), CLI);
    assert!(
        report.contains("downloaded and SHA-256 verified"),
        "{report}"
    );
    assert!(report.contains("✓ Dockerfile"), "{report}");
    if cfg!(unix) {
        assert_eq!(
            read(root.path(), TAILWIND_OUTPUT),
            "/*! tailwindcss fixture */"
        );
        assert!(report.contains("built by tailwindcss"), "{report}");
    }

    let before: Vec<String> = [
        TAILWIND_INPUT,
        ".gitignore",
        "src/pages/layout.rs",
        "Dockerfile",
    ]
    .iter()
    .map(|path| read(root.path(), path))
    .collect();
    let again = harness.tailwind(root.path(), false).unwrap();
    let after: Vec<String> = [
        TAILWIND_INPUT,
        ".gitignore",
        "src/pages/layout.rs",
        "Dockerfile",
    ]
    .iter()
    .map(|path| read(root.path(), path))
    .collect();
    assert_eq!(before, after);
    assert_eq!(harness.fetcher.calls.get(), 1, "the cached CLI is reused");
    for expected in [
        "Nothing to change",
        "already imports tailwindcss",
        "src/pages/layout.rs already links the stylesheet",
        "Dockerfile already builds the stylesheet",
        "cached, SHA-256 verified",
    ] {
        assert!(again.contains(expected), "{expected}\n{again}");
    }
    assert!(!again.contains("built by tailwindcss"), "{again}");
    let dry = harness.tailwind(root.path(), true).unwrap();
    assert!(dry.contains("cached in the user cache"), "{dry}");
    assert!(dry.contains("Nothing to change"), "{dry}");
}

#[test]
fn render_page_projects_get_the_head_snippet_instead_of_an_edit() {
    let root = project(&[("src/main.rs", RENDER_PAGE_MAIN)]);
    let harness = Harness::serving(CLI);
    let report = harness.tailwind(root.path(), false).unwrap();
    assert_eq!(read(root.path(), "src/main.rs"), RENDER_PAGE_MAIN);
    assert!(
        report.contains("Wire it in (not edited automatically)"),
        "{report}"
    );
    assert!(
        report.contains("no html! in src/ closes <head>"),
        "{report}"
    );
    assert!(
        report.contains("rullst::htmx::render_page_with_head(&htmx, \"en\", title,"),
        "{report}"
    );
    assert!(
        report.contains(&format!("    rullst::html! {{ {} }},", tailwind::LINK_LINE)),
        "{report}"
    );
    // No .gitignore and no Dockerfile: neither is created.
    assert!(!root.path().join(".gitignore").exists());
    assert!(!root.path().join("Dockerfile").exists());
}

#[test]
fn several_layouts_and_an_old_rullst_are_not_edited() {
    let second = LAYOUT.replace("pub fn page", "pub fn bare");
    let root = project(&[
        ("src/pages/layout.rs", LAYOUT),
        ("src/pages/error.rs", &second),
    ]);
    let harness = Harness::serving(CLI);
    let report = harness.tailwind(root.path(), true).unwrap();
    assert!(
        report.contains("the <head> of each layout (src/pages/error.rs, src/pages/layout.rs)"),
        "{report}"
    );

    let root = project(&[
        ("src/pages/layout.rs", LAYOUT),
        (
            "Cargo.lock",
            "version = 4\n\n[[package]]\nname = \"rullst\"\nversion = \"13.0.0-alpha.4\"\n",
        ),
    ]);
    let report = harness.tailwind(root.path(), false).unwrap();
    assert_eq!(read(root.path(), "src/pages/layout.rs"), LAYOUT);
    assert!(
        report.contains("src/pages/layout.rs, inside <head> (after `cargo rullst upgrade`: rullst 13.0.0-alpha.4 has no rullst::assets"),
        "{report}"
    );
}

#[test]
fn conflicting_files_stop_before_any_download_or_write() {
    let harness = Harness::serving(CLI);
    let root = project(&[(TAILWIND_INPUT, "body { color: red }\n")]);
    let error = frontend_error(harness.tailwind(root.path(), false));
    assert!(
        matches!(
            error,
            FrontendError::Conflict {
                file: TAILWIND_INPUT,
                ..
            }
        ),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        "assets/css/app.css exists and does not import tailwindcss"
    );
    let root = project(&[(TAILWIND_OUTPUT, "main { margin: 0 }\n")]);
    let error = frontend_error(harness.tailwind(root.path(), false));
    assert!(
        matches!(
            error,
            FrontendError::Conflict {
                file: TAILWIND_OUTPUT,
                ..
            }
        ),
        "{error}"
    );
    assert!(!root.path().join(TAILWIND_INPUT).exists());
    assert_eq!(harness.fetcher.calls.get(), 0);
    // A stylesheet Tailwind built earlier is fine.
    let root = project(&[(TAILWIND_OUTPUT, "/*! tailwindcss v4.3.5 */\n")]);
    assert!(harness.tailwind(root.path(), true).is_ok());
}

#[test]
fn a_failed_download_or_an_unsupported_host_writes_nothing() {
    let root = project(&[("src/pages/layout.rs", LAYOUT), (".gitignore", "")]);
    let wrong = Harness::serving(b"other bytes");
    let error = frontend_error(wrong.tailwind(root.path(), false));
    assert!(
        matches!(
            error,
            FrontendError::Tool(ToolError::ChecksumMismatch { .. })
        ),
        "{error}"
    );
    let offline = Harness {
        fetcher: Fixture::failing(FetchError::Status {
            url: "https://github.com/x".into(),
            status: 503,
        }),
        ..Harness::serving(b"")
    };
    let error = frontend_error(offline.tailwind(root.path(), false));
    assert!(
        matches!(error, FrontendError::Tool(ToolError::Download(_))),
        "{error}"
    );
    assert!(!root.path().join(TAILWIND_INPUT).exists());
    assert_eq!(read(root.path(), ".gitignore"), "");
    assert_eq!(read(root.path(), "src/pages/layout.rs"), LAYOUT);

    let toolchain = Toolchain {
        fetcher: &offline.fetcher,
        tailwind: None,
        cache_base: None,
        build: false,
        alpine_integrity: "",
    };
    let error = frontend_error(execute(
        root.path(),
        find("tailwind").unwrap(),
        true,
        Style::PLAIN,
        &toolchain,
    ));
    assert!(
        matches!(
            error,
            FrontendError::Tool(ToolError::UnsupportedPlatform { .. })
        ),
        "{error}"
    );
    let no_cache = Toolchain {
        tailwind: Some(&ASSET),
        ..toolchain
    };
    let error = frontend_error(execute(
        root.path(),
        find("tailwind").unwrap(),
        false,
        Style::PLAIN,
        &no_cache,
    ));
    assert!(
        matches!(error, FrontendError::Tool(ToolError::NoCacheDirectory)),
        "{error}"
    );
    assert!(!root.path().join(TAILWIND_INPUT).exists());
}

#[test]
fn outside_a_project_the_kits_fail_with_the_shared_report() {
    let root = tempfile::tempdir().unwrap();
    let harness = Harness::serving(CLI);
    assert!(matches!(
        harness.tailwind(root.path(), true),
        Err(Failure::Project)
    ));
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"x\"\n[dependencies]\nserde = \"1\"\n",
    )
    .unwrap();
    assert!(matches!(
        harness.run(root.path(), "alpine", true, ""),
        Err(Failure::Project)
    ));
    fs::write(root.path().join("Cargo.toml"), "not toml [").unwrap();
    assert!(matches!(
        harness.tailwind(root.path(), true),
        Err(Failure::Project)
    ));
}

#[test]
fn a_tracked_or_already_ignored_output_keeps_the_gitignore() {
    for ignored in ["/static/app.css", "static/app.css", "static/", "/static"] {
        assert!(
            tailwind::ignores_output(&format!("/target\n  {ignored}  \n")),
            "{ignored}"
        );
    }
    assert!(!tailwind::ignores_output(
        "/target\n/static/other.css\n# /static/app.css\n"
    ));
    let root = project(&[(".gitignore", "/target\nstatic/app.css\n")]);
    let report = Harness::serving(CLI).tailwind(root.path(), true).unwrap();
    assert!(!report.contains(".gitignore"), "{report}");

    // A project that commits static/app.css keeps committing it.
    let root = project(&[
        (".gitignore", "/target\n"),
        (TAILWIND_OUTPUT, "/*! tailwindcss v4 */\n"),
    ]);
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(root.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    };
    if git(&["init", "-q"]) && git(&["add", TAILWIND_OUTPUT]) {
        let report = Harness::serving(CLI).tailwind(root.path(), true).unwrap();
        assert!(
            report.contains("static/app.css is tracked by Git, so it stays committed"),
            "{report}"
        );
        assert!(!report.contains("+++ .gitignore"), "{report}");
    }
}

#[test]
fn a_project_with_its_own_tailwind_build_is_told_to_keep_one() {
    let root = project(&[("tailwind.config.js", "module.exports = {}\n")]);
    let report = Harness::serving(CLI).tailwind(root.path(), true).unwrap();
    assert!(
        report.contains("the project also builds Tailwind itself"),
        "{report}"
    );
    let root = project(&[]);
    let report = Harness::serving(CLI).tailwind(root.path(), true).unwrap();
    assert!(!report.contains("builds Tailwind itself"), "{report}");
}

#[test]
fn an_unrecognised_dockerfile_gets_the_stages_to_paste() {
    let root = project(&[("Dockerfile", "FROM debian\nCOPY . /app\n")]);
    let report = Harness::serving(CLI).tailwind(root.path(), true).unwrap();
    assert!(
        report.contains("Dockerfile (not the generated one)"),
        "{report}"
    );
    assert!(
        report.contains("FROM scratch AS tailwind-cli-amd64"),
        "{report}"
    );
    assert!(report.contains(docker::COPY_LINE.trim_end()), "{report}");
}
