//! 13.0.0-alpha.6: the health probes no longer report the framework version,
//! and the CSRF middleware decides static files by path instead of by
//! extension.

use super::super::alpha6_defaults::reads_probe_version;
use super::files::Project;
use super::*;

const VERSION: &str = "V13-HEALTH-FRAMEWORK-VERSION";
const FIELD: &str = "V13-HEALTH-PROBE-VERSION-FIELD";
const STATIC: &str = "V13-CSRF-STATIC-PREFIXES";
const PACKAGE: &str = "[package]\nname = \"app\"\n";

fn found(source: &str) -> Vec<(&'static str, usize)> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
}

#[test]
fn reading_version_next_to_a_probe_path_is_reviewed_at_every_location() {
    let source = r#"
#[tokio::test]
async fn probe() {
    let body = json(get("/health")).await;
    assert_eq!(body["version"], "13.0.0-alpha.5");
    let ready = json(get("/ready?x=1")).await;
    println!("{:?}", ready.get("version"));
}
"#;
    assert_eq!(found(source), [(VERSION, 5), (VERSION, 7)]);
    // `app_version`, `version` away from any probe path and a comment are not.
    assert_only(
        "fn f(body: Value) { let _ = (get(\"/health\"), &body[\"app_version\"]); }\n",
        &[],
    );
    assert_only(
        "fn f(body: Value) -> Value { body[\"version\"].clone() }\n",
        &[],
    );
    assert_only(
        "// GET /health returns \"version\"\nfn f() { let _ = \"/health\"; }\n",
        &[],
    );
}

#[test]
fn the_probe_payload_version_field_must_change() {
    let source = r#"
use rullst::health::HealthProbeResponse;
fn read(probe: &HealthProbeResponse) -> &str {
    probe.version
}
fn build() -> HealthProbeResponse {
    rullst::health::HealthProbeResponse { status: "UP", uptime_seconds: 0, timestamp: 0 }
}
"#;
    assert_eq!(found(source), [(FIELD, 4), (FIELD, 7)]);
    assert_eq!(
        found(
            "fn r(p: &rullst::health::ReadinessProbeResponse) -> u64 {\n    p.version.len() as u64 + p.uptime_seconds\n}\n"
        ),
        [(FIELD, 2)]
    );
    // `.version` of another type, a `version()` method and other fields.
    assert_only("fn v(p: &Package) -> &str { p.version }\n", &[]);
    assert_only(
        "use rullst::health::HealthProbeResponse;\nfn v(p: &Package, h: &HealthProbeResponse) -> (String, u64) { (p.version(), h.uptime_seconds) }\n",
        &[],
    );
    assert_only("fn s() -> Probe { Probe { version: 1 } }\n", &[]);
}

#[test]
fn file_services_outside_the_default_static_paths_are_reviewed() {
    let source = r#"
use tower_http::services::{ServeDir, ServeFile};
fn router() -> axum::Router {
    axum::Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        .route_service("/humans.txt", ServeFile::new("humans.txt"))
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/static/images/", ServeDir::new("images"))
        .nest_service("/.well-known", ServeDir::new("well-known"))
        .route_service("/favicon.ico", ServeFile::new("favicon.ico"))
        .route_service("/sw.js", ServeFile::new("sw.js"))
        .nest_service("/", ServeDir::new("public"))
}
"#;
    assert_eq!(found(source), [(STATIC, 5), (STATIC, 6), (STATIC, 12)]);
    // Without a file service in the file, and for other methods, nothing.
    assert_only(
        "fn r(api: Svc) -> Router { Router::new().nest_service(\"/api\", api) }\n",
        &[],
    );
    assert_only(
        "use tower_http::services::ServeDir;\nfn r() -> Router { Router::new().nest(\"/assets\", other()).nest_service(path, ServeDir::new(\"a\")) }\n",
        &[],
    );
    assert_only(
        "use tower_http::services::ServeDir;\nfn r() -> Router { Router::new().nest_service(\"/staticx\", ServeDir::new(\"a\")) }\n",
        &[STATIC],
    );
}

#[test]
fn scripts_reading_version_from_a_probe_are_reviewed() {
    let project = Project::new();
    project
        .write(
            ".github/workflows/deploy.yml",
            "steps:\n  - run: curl -fsS https://app.example/health\n  - run: curl -fsS https://app.example/health | jq -r .version\n",
        )
        .write(
            "Makefile",
            "status:\n\tcurl -s $(URL)/ready | jq '.app_version'\n\tcurl -s $(URL)/ready | grep '\"version\"'\n",
        );
    let mut found = project.findings(PACKAGE);
    found.sort();
    assert_eq!(
        found,
        [
            (".github/workflows/deploy.yml".to_string(), VERSION, 3),
            ("Makefile".to_string(), VERSION, 3),
        ]
    );
    for line in [
        "curl -s $URL/health | jq -r .version",
        "curl -s $URL/ready | jq '.version'",
        "http GET :3000/health | jq .version,.status",
        "curl $URL/health | python3 -c 'import json,sys; print(json.load(sys.stdin)[\"version\"])'",
        "curl $URL/health | jq \"['version']\" # 'version'",
    ] {
        assert!(reads_probe_version(line), "{line}");
    }
    for line in [
        "curl -s $URL/health | jq -r .app_version",
        "curl -s $URL/health | jq -r .versions",
        "curl -s $URL/health",
        "curl -s $URL/api/info | jq -r .version",
        "cargo rullst --version",
    ] {
        assert!(!reads_probe_version(line), "{line}");
    }
}

#[test]
fn the_alpha6_default_rules_name_their_rows_kinds_and_release() {
    let alpha5 = Version::parse("13.0.0-alpha.5").unwrap();
    let alpha6 = Version::parse("13.0.0-alpha.6").unwrap();
    for (rule, kind, row) in [
        (
            &catalog::HEALTH_FRAMEWORK_VERSION,
            FindingKind::Review,
            "Health probes without the framework version",
        ),
        (
            &catalog::HEALTH_PROBE_VERSION_FIELD,
            FindingKind::MustChange,
            "Health probes without the framework version",
        ),
        (
            &catalog::CSRF_STATIC_PREFIXES,
            FindingKind::Review,
            "CSRF static files by path",
        ),
    ] {
        assert_eq!((rule.kind, rule.row), (kind, row), "{}", rule.code);
        assert!(rule.introduced_after(&alpha5), "{}", rule.code);
        assert!(!rule.introduced_after(&alpha6), "{}", rule.code);
    }
}
