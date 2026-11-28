//! `V13-CSRF-STATIC-PREFIXES` compares paths with the `[security]`
//! `csrf_static_prefixes` and `csrf_static_paths` of the project's
//! `Rullst.toml`, read as `Server` reads them (#895).

use super::super::alpha6_defaults::StaticPaths;
use super::files::plan;
use super::*;
use rullst_core::config::SecurityConfig;

const STATIC: &str = "V13-CSRF-STATIC-PREFIXES";
const PACKAGE: &str = "[package]\nname = \"app\"\n";

/// A file service, a route that serves a file and `GET` routes whose handler
/// renders the response, at paths the lists may cover.
const ROUTER: &str = r#"use tower_http::services::ServeDir;
fn router() -> axum::Router {
    axum::Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        .route("/assets/app.css", get(css))
        .route("/robots.txt", get(robots))
        .route("/.well-known/security.txt", get(security_txt))
}
fn api() -> Router {
    routes![
        get("/api/x.json" => x),
        get("/feed.xml" => feed),
    ]
}
"#;

/// The `V13-CSRF-STATIC-PREFIXES` lines of [`ROUTER`] and their notes, in a
/// project with `rullst_toml` as its `Rullst.toml` (none without).
fn reviewed(rullst_toml: Option<&str>) -> Vec<(usize, Option<String>)> {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), ROUTER).unwrap();
    std::fs::write(root.join("Cargo.toml"), PACKAGE).unwrap();
    if let Some(text) = rullst_toml {
        std::fs::write(root.join("Rullst.toml"), text).unwrap();
    }
    let plans = [plan(root.join("Cargo.toml"), PACKAGE)];
    scan_workspace(root, &plans, TARGET_MAJOR)
        .unwrap()
        .findings
        .into_iter()
        .filter(|finding| finding.rule.code == STATIC)
        .map(|finding| (finding.line, finding.note))
        .collect()
}

fn lines(found: &[usize]) -> Vec<(usize, Option<String>)> {
    found.iter().map(|line| (*line, None)).collect()
}

#[test]
fn without_project_lists_the_defaults_apply() {
    // `/robots.txt` and `/.well-known/` are in the defaults.
    assert_eq!(reviewed(None), lines(&[4, 5, 11, 12]));
    assert_eq!(
        reviewed(Some("[app]\nenv = \"development\"\n")),
        lines(&[4, 5, 11, 12])
    );
    assert_eq!(
        reviewed(Some("[security]\ncsrf_same_site = \"Strict\"\n")),
        lines(&[4, 5, 11, 12])
    );
}

#[test]
fn the_paths_the_project_lists_are_not_reported() {
    // The exact path of a route whose handler renders the response, beside
    // the default prefixes, as in the issue.
    assert_eq!(
        reviewed(Some(
            "[security]\ncsrf_static_paths = [\"/robots.txt\", \"/api/x.json\"]\n"
        )),
        lines(&[4, 5, 12])
    );
    // A prefix covers the service mounted at it and the routes below it.
    assert_eq!(
        reviewed(Some(
            "[security]\ncsrf_static_prefixes = [\"/static/\", \"/.well-known/\", \"/assets/\"]\n"
        )),
        lines(&[11, 12])
    );
    // A list replaces its default, so the defaults it leaves out are
    // reported, as `Server` treats them.
    assert_eq!(
        reviewed(Some(
            "[security]\ncsrf_static_prefixes = [\"/assets/\"]\ncsrf_static_paths = [\"/api/x.json\", \"/feed.xml\"]\n"
        )),
        lines(&[6, 7])
    );
}

/// `Server` checks the request path, so an exact path covers no file below
/// a `nest_service`, even one equal to the mount point.
#[test]
fn only_a_prefix_covers_a_mounted_service() {
    for exact in ["/assets/", "/assets"] {
        assert_eq!(
            reviewed(Some(&format!(
                "[security]\ncsrf_static_paths = [\"{exact}\"]\n"
            ))),
            lines(&[4, 5, 6, 11, 12]),
            "{exact}"
        );
    }
    // A `route_service` serves its own path only: `/assets` is not below
    // `/assets/`, while a wildcard route below it is.
    let facts = WorkspaceFacts {
        csrf_static: StaticPaths::project(
            SecurityConfig::default().with_csrf_static_prefix("/assets/"),
        ),
        ..WorkspaceFacts::default()
    };
    let source = r#"use tower_http::services::ServeDir;
fn router() -> axum::Router {
    axum::Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        .route_service("/assets", ServeDir::new("assets"))
        .route_service("/assets/{*path}", ServeDir::new("assets"))
        .route("/assets", get_service(ServeDir::new("assets")))
}
"#;
    assert_eq!(
        scan_with(source, "lib.rs", &facts),
        [(STATIC, 5), (STATIC, 7)]
    );
}

#[test]
fn a_rullst_toml_server_refuses_keeps_the_defaults_with_a_note() {
    let note = |reason: &str| {
        Some(format!(
            "`Rullst.toml` was not used ({reason}), so this path is compared with the default `csrf_static_prefixes` and `csrf_static_paths`; `Server` refuses to start with it"
        ))
    };
    let parse =
        note("failed to parse Rullst configuration: invalid configuration at line 1, column 10");
    assert_eq!(
        reviewed(Some(
            "[security\ncsrf_static_paths = [\"/api/x.json\", \"/feed.xml\"]\n"
        )),
        [4, 5, 11, 12].map(|line| (line, parse.clone()))
    );
    // `Server` validates the lists: a prefix must end with `/`.
    let found = reviewed(Some(
        "[security]\ncsrf_static_prefixes = [\"/assets\"]\ncsrf_static_paths = [\"/api/x.json\"]\n",
    ));
    assert_eq!(
        found.iter().map(|(line, _)| *line).collect::<Vec<_>>(),
        [4, 5, 11, 12]
    );
    let reason = found[0].1.as_deref().unwrap();
    assert!(
        reason.starts_with("`Rullst.toml` was not used (invalid security configuration: "),
        "{reason}"
    );
    assert!(found.iter().all(|(_, note)| note == &found[0].1));
}

#[test]
fn a_rendered_route_is_described_as_a_route() {
    let rule = &catalog::CSRF_STATIC_PREFIXES;
    assert!(!rule.message.contains("serves files"), "{}", rule.message);
    assert!(
        rule.message.contains(
            "a `GET` route whose path ends in an extension such as `.xml` or `.json` that alpha.5 treated as static, whether its handler serves a file or renders the response"
        ),
        "{}",
        rule.message
    );
    assert!(
        rule.message
            .contains("as the project's `Rullst.toml` sets them or by default"),
        "{}",
        rule.message
    );
}
