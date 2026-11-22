//! 13.0.0-alpha.6: the health probes no longer report the framework version,
//! the CSRF middleware decides static files by path instead of by extension,
//! and pages embed the CSRF token masked per response, which the generated
//! Caddyfile compresses.

use super::super::alpha6_defaults::reads_probe_version;
use super::files::Project;
use super::*;

const VERSION: &str = "V13-HEALTH-FRAMEWORK-VERSION";
const FIELD: &str = "V13-HEALTH-PROBE-VERSION-FIELD";
const STATIC: &str = "V13-CSRF-STATIC-PREFIXES";
const COOKIE: &str = "V13-CSRF-COOKIE-IN-PAGE";
const ENCODE: &str = "V13-DEPLOY-CADDY-ENCODE";
const LAYERS: &str = "V13-ROUTER-BASELINE-LAYERS";
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
    // An application's own `/healthz` probe next to a quiz answer named
    // `version`; a probe path built with `format!` still counts.
    assert_only(
        "fn f() { let _ = (get(\"/healthz\"), json!({\"choice\": \"version\"})); }\n",
        &[],
    );
    assert_only(
        "fn f(base: &str, body: Value) { let _ = (format!(\"{base}/ready\"), &body[\"version\"]); }\n",
        &["V13-HEALTH-FRAMEWORK-VERSION"],
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

/// `GET` routes that serve files outside the static prefixes: a
/// `get_service` file service, and a `get` route at a path alpha.5 treated as
/// static by its extension (#826).
#[test]
fn get_routes_that_serve_files_outside_the_static_paths_are_reviewed() {
    let source = r#"
use tower_http::services::{ServeDir, ServeFile};
fn router() -> axum::Router {
    axum::Router::new()
        .route("/feed.xml", get(feed))
        .route("/data/report.json", axum::routing::get(report).post(save))
        .route("/downloads/{*path}", get_service(ServeDir::new("downloads")))
        .route("/terms", get_service(ServeFile::new("terms.html")))
        .route("/robots.txt", get(robots))
        .route("/static/app.css", get(css))
        .route("/.well-known/security.txt", get(security_txt))
        .route("/api/users", get(users))
        .route("/upload.json", post(upload))
        .route("/assets/app.CSS", get(css))
}
fn pages() -> Router {
    routes![
        get("/sitemap.xml" => sitemap),
        get("/manifest.json" => manifest),
        get("/about" => about),
        post("/hooks.json" => hook),
    ]
}
"#;
    assert_eq!(
        found(source),
        [
            (STATIC, 5),
            (STATIC, 6),
            (STATIC, 7),
            (STATIC, 8),
            (STATIC, 19)
        ]
    );
    // `get(handler)` alone, a path that is not absolute, a template string
    // and other methods are not routes of a file.
    assert_only(
        "fn r() -> Router { Router::new().route(PATH, get(feed)).route(\"feed.xml\", get(feed)).merge(get(\"/x.json\")) }\n",
        &[],
    );
    assert_only(
        "fn r() -> Router { Router::new().route(\"/files\", get_service(other_service())) }\n",
        &[],
    );
    assert_only(
        "fn r() -> Router { routes![get(\"/feed.xml\" => feed)] }\n",
        &[STATIC],
    );
}

/// The `get_csrf_token` helper `make:auth` generated before 13.0.0-alpha.6.
const RAW_COOKIE_HELPER: &str = r#"
pub(crate) fn get_csrf_token(headers: &HeaderMap) -> String {
    headers
        .get(rullst::server::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie_header| {
            cookie_header.split(';').find_map(|cookie| {
                cookie
                    .trim()
                    .strip_prefix("rullst_csrf=")
                    .map(ToOwned::to_owned)
            })
        })
        .unwrap_or_default()
}
"#;

#[test]
fn reading_the_raw_csrf_cookie_outside_tests_is_reviewed() {
    let found = |file: &str, source: &str| scan_with(source, file, &WorkspaceFacts::default());
    assert_eq!(
        found("src/controllers/auth_controller.rs", RAW_COOKIE_HELPER),
        [(COOKIE, 10)]
    );
    // The helper generated since 13.0.0-alpha.6 masks the value.
    let masked = RAW_COOKIE_HELPER.replace(
        ".map(ToOwned::to_owned)",
        ".and_then(rullst::security::mask_csrf_token)",
    );
    assert_eq!(found("src/controllers/auth_controller.rs", &masked), []);
    // Tests read the cookie a response sets; other prefixes are not it.
    for test_file in [
        "tests/security.rs",
        "tests/common/mod.rs",
        "src/tests.rs",
        "src/auth_tests.rs",
        "src/auth_test.rs",
    ] {
        assert_eq!(found(test_file, RAW_COOKIE_HELPER), [], "{test_file}");
    }
    for source in [
        "fn f(c: &str) -> Option<&str> { c.strip_prefix(\"rullst_session=\") }\n",
        "fn f(c: &str, p: &str) -> Option<&str> { c.strip_prefix(p) }\n",
        // Sending the cookie, or text that only contains its name.
        "fn f(token: &str) -> String { format!(\"rullst_csrf={token}\") }\n",
        "fn f() -> (&'static str, &'static str) { (\"rullst_csrf_hint\", \"the rullst_csrf cookie\") }\n",
        "// cookie(headers, \"rullst_csrf\")\nfn f() {}\n",
    ] {
        assert_eq!(found("src/lib.rs", source), [], "{source}");
    }
}

/// An application's own helper that reads the CSRF cookie by name into a
/// page, as migration-guide step 12 describes (#824).
const OWN_COOKIE_HELPER: &str = r#"
fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers.get(COOKIE)?.to_str().ok()?.split(';').find_map(|pair| {
        let (key, value) = pair.trim().split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}
pub fn layout(headers: &HeaderMap) -> Markup {
    let token = cookie(headers, "rullst_csrf").unwrap_or_default();
    html! { <meta name="csrf-token" content={token}> }
}
fn from_jar(jar: &CookieJar) -> Option<String> {
    jar.get("rullst_csrf").map(|cookie| cookie.value().to_owned())
}
fn by_name(cookies: &[Cookie]) -> Option<&str> {
    cookies.iter().find(|c| c.name() == "rullst_csrf").map(Cookie::value)
}
"#;

#[test]
fn reading_the_csrf_cookie_by_name_is_reviewed_unless_the_file_masks_it() {
    let found = |file: &str, source: &str| scan_with(source, file, &WorkspaceFacts::default());
    assert_eq!(
        found("src/views/layout.rs", OWN_COOKIE_HELPER),
        [(COOKIE, 9), (COOKIE, 13), (COOKIE, 16)]
    );
    let masked = OWN_COOKIE_HELPER.replace(
        "cookie(headers, \"rullst_csrf\").unwrap_or_default()",
        "cookie(headers, \"rullst_csrf\").and_then(|value| rullst::security::mask_csrf_token(&value)).unwrap_or_default()",
    );
    assert_ne!(masked, OWN_COOKIE_HELPER);
    assert_eq!(found("src/views/layout.rs", &masked), []);
    assert_eq!(found("tests/layout.rs", OWN_COOKIE_HELPER), []);
    // A constant holding the name is reviewed where it is written.
    assert_eq!(
        found(
            "src/csrf.rs",
            "const CSRF_COOKIE: &str = \"rullst_csrf\";\nfn token(headers: &HeaderMap) -> Option<String> { cookie(headers, CSRF_COOKIE) }\n"
        ),
        [(COOKIE, 1)]
    );
}

#[test]
fn a_caddyfile_without_encode_is_reviewed_at_its_proxy() {
    let alpha5 = "{$DOMAIN:localhost} {\n\t# No `encode`: pages carry the CSRF token unmasked\n\treverse_proxy app:3000 {\n\t\thealth_uri /health\n\t\tlb_try_duration 15s\n\t}\n}\n";
    let project = Project::new();
    project.write("Caddyfile", alpha5);
    assert_eq!(
        project.findings(PACKAGE),
        [("Caddyfile".to_string(), ENCODE, 3)]
    );
    let commented = alpha5.replace("\treverse_proxy", "\t# encode zstd gzip\n\treverse_proxy");
    let project = Project::new();
    project.write("Caddyfile", &commented);
    assert_eq!(
        project.findings(PACKAGE),
        [("Caddyfile".to_string(), ENCODE, 4)]
    );
    let api_only = alpha5.replace(
        "\treverse_proxy",
        "\tencode /api/* zstd gzip\n\treverse_proxy",
    );
    // A Caddyfile moved to a deployment folder, beside one at the root.
    for folder in ["deploy", "ops", "infra", "docker"] {
        let project = Project::new();
        project.write(&format!("{folder}/Caddyfile"), alpha5);
        project.write("Caddyfile", crate::blueprints::deploy::CADDYFILE);
        assert_eq!(
            project.findings(PACKAGE),
            [(format!("{folder}/Caddyfile"), ENCODE, 3)],
            "{folder}"
        );
    }
    let project = Project::new();
    project.write("elsewhere/Caddyfile", alpha5);
    assert_eq!(project.findings(PACKAGE), []);
    for caddyfile in [
        crate::blueprints::deploy::CADDYFILE,
        api_only.as_str(),
        "example.com {\n    reverse_proxy web:8080\n}\n",
    ] {
        let project = Project::new();
        project.write("Caddyfile", caddyfile);
        assert_eq!(project.findings(PACKAGE), [], "{caddyfile}");
    }
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
        (
            &catalog::CSRF_COOKIE_IN_PAGE,
            FindingKind::Review,
            "CSRF token masked per response",
        ),
        (
            &catalog::DEPLOY_CADDY_ENCODE,
            FindingKind::Review,
            "Compressed pages in the generated Caddyfile",
        ),
    ] {
        assert_eq!((rule.kind, rule.row), (kind, row), "{}", rule.code);
        assert!(rule.introduced_after(&alpha5), "{}", rule.code);
        assert!(!rule.introduced_after(&alpha6), "{}", rule.code);
    }
}

#[test]
fn router_layers_that_server_now_applies_are_reviewed() {
    let source = r#"
pub fn router() -> Router {
    routes![get("/" => home)]
        .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
        .merge_axum(rullst::health::health_router())
        .layer(from_fn(headers_middleware))
        .route_layer(axum::middleware::from_fn(rullst_core::security::csrf_middleware))
}
"#;
    assert_eq!(found(source), [(LAYERS, 4), (LAYERS, 6), (LAYERS, 7)]);
    // Other layers, the names outside a layer call and the baseline itself.
    assert_only(
        "fn r(x: Router) -> Router {\n    x.layer(from_fn(auth_middleware)).route(\"/\", get(headers_middleware_docs))\n}\n",
        &[],
    );
    assert_only(
        "fn r(x: axum::Router) -> Result<axum::Router, Error> {\n    Ok(rullst::security::apply_security_baseline(x, config(), Environment::Test)?)\n}\n",
        &[],
    );
}
