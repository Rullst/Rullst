//! The 13.0.0-alpha.6 changes the first alpha.6 rules left to the checklist:
//! refused cron literals, CSRF tokens compared with the cookie, `jsonwebtoken`
//! 11 key constructors, `ed25519-dalek` 2 beside Rullst's 3, SQLite without a
//! journal-mode setting and the `new` of a
//! `define_provider!` type.

use super::super::alpha6_gaps::refused_cron;
use super::super::files::manifest_findings;
use super::files::{Project, plan};
use super::*;

const PACKAGE: &str = "[package]\nname = \"app\"\n\n";

fn lines_of(code: &str, found: &[(&'static str, usize)]) -> Vec<usize> {
    found
        .iter()
        .filter(|(found, _)| *found == code)
        .map(|(_, line)| *line)
        .collect()
}

fn facts(manifests: &[(&str, &str)]) -> WorkspaceFacts {
    let plans: Vec<_> = manifests
        .iter()
        .map(|(path, text)| plan(PathBuf::from(path), text))
        .collect();
    manifest_findings(&plans).1
}

fn manifest_codes(manifests: &[(&str, &str)]) -> Vec<(String, &'static str, usize)> {
    let plans: Vec<_> = manifests
        .iter()
        .map(|(path, text)| plan(PathBuf::from(path), text))
        .collect();
    manifest_findings(&plans)
        .0
        .into_iter()
        .map(|finding| {
            (
                finding.path.to_string_lossy().into_owned(),
                finding.rule.code,
                finding.line,
            )
        })
        .collect()
}

#[test]
fn cron_expressions_with_a_split_field_are_refused() {
    for refused in [
        "1, 2 * * *",
        "1 ,2 * * * *",
        "0 9 * * MON- FRI",
        "*/ 15 * * * *",
        "-5 * * * *",
    ] {
        assert!(refused_cron(refused), "{refused}");
    }
    // Valid expressions, and one that the `cron` crate refused too (four
    // fields), are not reported.
    for accepted in [
        "1,2 * * * *",
        "*/15 * * * *",
        "5/15 1-5 * * MON-FRI",
        "0 18 * *",
        "send-reports",
    ] {
        assert!(!refused_cron(accepted), "{accepted}");
    }
}

#[test]
fn scheduler_registrations_with_refused_literals_are_reviewed() {
    let source = r#"
use rullst::Scheduler;
fn schedule(deferral: rullst::ScheduleDeferral) -> Result<Scheduler, rullst::SchedulerError> {
    Scheduler::new()
        .task("1, 2 * * *", || async {})?
        .task("1,2 * * * *", || async {})?
        .task_in_tz("0 9 * * MON- FRI", "Europe/Lisbon", || async {})?
        .deferrable_task("*/ 15 * * * *", deferral.clone(), || async {})?
        .deferrable_task_in_tz("0 0 * * *", "UTC", deferral, || async {})?
        .deferrable_task_in_tz("0 ,30 * * *", "UTC", deferral, || async {})?
        .task(EXPRESSION, || async {})
}
fn other(queue: Queue) { queue.retry("1, 2 * * *"); }
"#;
    let found = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-SCHEDULER-CRON-FIELDS", &found),
        [5, 7, 8, 10],
        "{found:?}"
    );
    // A file that never names `Scheduler` registers no cron task.
    let queue = "fn f(queue: Queue) { queue.task(\"1, 2 * * *\", || async {}); }\n";
    let found = scan_with(queue, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-SCHEDULER-CRON-FIELDS", &found),
        Vec::<usize>::new()
    );
}

const CSRF_TEST: &str = r#"
#[tokio::test]
async fn the_page_embeds_the_csrf_token() {
    let cookie = csrf_cookie(&response);
    let page_token = token_in(&body);
    assert_eq!(page_token, cookie);
    assert!(body.contains(&cookie));
    assert!(page_token == cookie);
    assert_ne!(page_token, cookie);
    assert!(!body.contains(&cookie));
    assert!(set_cookie.contains("rullst_csrf="));
    assert!(set_cookie.contains(&cookie));
    assert_eq!(cookie.len(), 32);
    assert_eq!(page_token.len(), 86);
    let header = response.headers()["set-cookie"].to_str()?;
    assert_eq!(header.strip_prefix("rullst_csrf=")?, page_token);
    debug_assert_eq!(cookie, page_token);
    assert_eq!(response.status(), 200);
}
"#;

#[test]
fn csrf_tokens_compared_with_the_cookie_are_reviewed_in_tests_too() {
    let found = scan_with(CSRF_TEST, "tests/csrf.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-CSRF-TOKEN-COMPARED", &found),
        [6, 7, 8, 16, 17],
        "{found:?}"
    );
}

#[test]
fn custom_csrf_checks_against_the_cookie_are_reviewed() {
    let source = r#"
use subtle::ConstantTimeEq;
fn check(form: &Form, csrf_cookie: &str) -> bool {
    if form.token == csrf_cookie { return true; }
    let matched = form.token.as_bytes().ct_eq(csrf_cookie.as_bytes()).into();
    let same = form.token.eq(csrf_cookie);
    let unrelated = form.token == form.previous;
    match form.kind { Kind::A => form.token == csrf_cookie, _ => matched && same && unrelated }
}
"#;
    let found = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-CSRF-TOKEN-COMPARED", &found),
        [4, 5, 6, 8],
        "{found:?}"
    );
}

/// Each operand ends at a separator, an assignment, a `=>`, `&&`/`||`, a
/// block or a clause keyword; references and call groups stay inside it.
const BOUNDARIES: &str = r#"
const COOKIE: &str = "rullst_csrf";
fn boundaries(page_token: &str, header_token: &str, session_cookie: Option<&str>, kind: Source) {
    assert!(session_cookie.is_some() && page_token == header_token);
    let csrf_cookie_matches = page_token == header_token;
    let picked = match kind { Source::Cookie => page_token == header_token, _ => false };
    { let cookie = 1; } page_token == header_token;
    assert!(token_in(&body) == csrf_cookie(&response));
    assert!(&page_token == &cookie);
    assert!(page_token != cookie);
    let unrelated = form.kind.eq(&other_kind);
    let other = match kind { Source::Cookie => page_token, _ => header_token };
    assert!(headers.set_cookie().contains(&cookie));
}
"#;

#[test]
fn comparison_operands_end_at_their_boundaries() {
    let found = scan_with(BOUNDARIES, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-CSRF-TOKEN-COMPARED", &found),
        [8, 9],
        "{found:?}"
    );
}

#[test]
fn generic_arguments_stay_inside_an_operand() {
    // Only the `>` of `=>` ends an operand: the turbofish's `>` does not, so
    // the cookie before it still belongs to the left operand.
    let source = "fn f(csrf_cookie: Jar, page_token: String) {\n    assert!(csrf_cookie.get::<String>() == page_token);\n}\n";
    let found = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-CSRF-TOKEN-COMPARED", &found),
        [2],
        "{found:?}"
    );
}

#[test]
fn only_method_calls_with_parentheses_are_read_as_comparisons() {
    // A free `contains(..)` call and a `.contains[..]` index are not the
    // method; `.contains(..)` on the line after them is.
    let source = "fn f(body: &str, csrf_cookie: &str, pages: Pages) -> bool {\n    let found = contains(body, csrf_cookie);\n    let page = pages.contains[csrf_cookie];\n    found && body.contains(csrf_cookie)\n}\n";
    let found = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-CSRF-TOKEN-COMPARED", &found),
        [4],
        "{found:?}"
    );
}

#[test]
fn every_mention_in_an_operand_counts_once_or_more() {
    // Two identifiers of each kind in one operand, and CSRF named twice by
    // literals only: neither cancels the other.
    let source = r#"
fn twice(csrf_form: Form, cookie_jar: Jar, cookie: &str, form: Form) {
    assert_eq!(csrf_form.csrf_token, cookie_jar.cookie_value());
    assert_eq!(cookie, form.token("csrf", "x-csrf"));
}
"#;
    let found = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-CSRF-TOKEN-COMPARED", &found),
        [3, 4],
        "{found:?}"
    );
    let names = "fn f(header: &str, other: &str, page_token: &str) {\n    assert_eq!(header.split(\"rullst_csrf=\").nth(1).or(other.strip_prefix(\"rullst_csrf=\")), Some(page_token));\n}\n";
    let found = scan_with(names, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-CSRF-TOKEN-COMPARED", &found),
        [2],
        "{found:?}"
    );
    // `CsrfToken` alone names CSRF for the file.
    let extension = "fn f(Extension(page): Extension<CsrfToken>, html: &str, cookie: &str) {\n    assert!(html.contains(&cookie));\n}\n";
    let found = scan_with(extension, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-CSRF-TOKEN-COMPARED", &found),
        [2],
        "{found:?}"
    );
}

#[test]
fn token_and_cookie_comparisons_without_csrf_are_not_reviewed() {
    let source = r#"
fn check(api_token: &str, session_cookie: &str, body: &str) {
    assert_eq!(api_token, session_cookie);
    assert!(api_token == session_cookie);
    assert!(body.contains(&session_cookie));
    assert!(api_token.ct_eq(session_cookie));
}
"#;
    let found = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-CSRF-TOKEN-COMPARED", &found),
        Vec::<usize>::new()
    );
    // Naming CSRF in the operands is enough, without the cookie's name.
    let named = "fn f(csrf: &str, cookie: &str) { assert_eq!(csrf, cookie); }\n";
    let found = scan_with(named, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(lines_of("V13-CSRF-TOKEN-COMPARED", &found), [1]);
    // Both operands read cookies: nothing compares the page's token.
    let cookies =
        "fn f(csrf_cookie: &str, cookie_token: &str) { assert_eq!(csrf_cookie, cookie_token); }\n";
    let found = scan_with(cookies, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-CSRF-TOKEN-COMPARED", &found),
        Vec::<usize>::new()
    );
}

const JWT_SOURCE: &str = r#"
use jsonwebtoken::{DecodingKey, EncodingKey};
fn keys(secret: &[u8]) {
    let encoding = EncodingKey::from_secret(secret);
    let decoding = jsonwebtoken::DecodingKey::from_secret(secret);
    let other = Hmac::from_secret(secret);
    let pem = DecodingKey::from_rsa_pem(secret);
}
"#;

#[test]
fn jsonwebtoken_key_constructors_are_reviewed_below_12() {
    for requirement in [
        r#""11""#,
        r#""=11.0.1""#,
        r#"{ version = "10", default-features = false }"#,
    ] {
        let manifest = format!("{PACKAGE}[dependencies]\njsonwebtoken = {requirement}\n");
        let facts = facts(&[("Cargo.toml", manifest.as_str())]);
        assert!(facts.jsonwebtoken_below_12, "{requirement}");
        let found = scan_with(JWT_SOURCE, "src/middlewares/jwt_middleware.rs", &facts);
        assert_eq!(
            lines_of("V13-JSONWEBTOKEN-12", &found),
            [4, 5],
            "{requirement}"
        );
    }
    for requirement in [
        r#""12""#,
        r#"{ version = "12.1", features = ["aws_lc_rs"] }"#,
        r#"{ workspace = true }"#,
    ] {
        let manifest = format!("{PACKAGE}[dependencies]\njsonwebtoken = {requirement}\n");
        let facts = facts(&[("Cargo.toml", manifest.as_str())]);
        assert!(!facts.jsonwebtoken_below_12, "{requirement}");
        let found = scan_with(JWT_SOURCE, "lib.rs", &facts);
        assert_eq!(lines_of("V13-JSONWEBTOKEN-12", &found), Vec::<usize>::new());
    }
    let workspace = "[workspace.dependencies]\njsonwebtoken = \"11\"\n";
    assert!(facts(&[("Cargo.toml", workspace)]).jsonwebtoken_below_12);
}

#[test]
fn ed25519_dalek_2_is_reviewed_beside_rullsts_3() {
    let old = "ed25519-dalek = { version = \"2\", features = [\"fast\"] }";
    for rullst in [
        r#"rullst = { version = "=13.0.0-alpha.5", default-features = false, features = ["iot"] }"#,
        r#"rullst = { version = "=13.0.0-alpha.5", default-features = false, features = ["oauth-sqlite"] }"#,
        r#"rullst-iot = "=13.0.0-alpha.5""#,
        r#"connect = { package = "rullst-connect", version = "=13.0.0-alpha.5" }"#,
    ] {
        let manifest = format!("{PACKAGE}[dependencies]\n{rullst}\n{old}\n");
        assert_eq!(
            manifest_codes(&[("Cargo.toml", manifest.as_str())]),
            [("Cargo.toml".to_string(), "V13-ED25519-DALEK-3", 6)],
            "{rullst}"
        );
    }
    // Another manifest of the workspace builds Rullst's copy.
    let member = format!("{PACKAGE}[dependencies]\n{old}\n");
    let app = format!("{PACKAGE}[dependencies]\nrullst-iot = \"=13.0.0-alpha.5\"\n");
    assert_eq!(
        manifest_codes(&[
            ("app/Cargo.toml", app.as_str()),
            ("keys/Cargo.toml", member.as_str())
        ]),
        [("keys/Cargo.toml".to_string(), "V13-ED25519-DALEK-3", 5)]
    );
    // A `[features]` entry enables the facade's `iot`.
    let optional = format!(
        "{PACKAGE}[dependencies]\nrullst = {{ version = \"=13.0.0-alpha.5\", default-features = false, optional = true, features = [\"auth\"] }}\n{old}\n\n[features]\ndevices = [\"rullst?/iot\"]\n"
    );
    assert_eq!(
        manifest_codes(&[("Cargo.toml", optional.as_str())]),
        [("Cargo.toml".to_string(), "V13-ED25519-DALEK-3", 6)]
    );
}

#[test]
fn ed25519_dalek_3_or_without_rullsts_copy_is_not_reviewed() {
    let current =
        "ed25519-dalek = { version = \"3\", default-features = false, features = [\"fast\"] }";
    let iot =
        r#"rullst = { version = "=13.0.0-alpha.6", default-features = false, features = ["iot"] }"#;
    let auth = r#"rullst = { version = "=13.0.0-alpha.6", default-features = false, features = ["auth"] }"#;
    for (rullst, dalek) in [(iot, current), (auth, "ed25519-dalek = \"2\"")] {
        let manifest = format!("{PACKAGE}[dependencies]\n{rullst}\n{dalek}\n");
        assert_eq!(
            manifest_codes(&[("Cargo.toml", manifest.as_str())]),
            [],
            "{rullst}"
        );
    }
    let unrelated = format!(
        "{PACKAGE}[dependencies]\n{auth}\ned25519-dalek = \"2\"\n\n[features]\nextra = [\"rullst/mail\"]\n"
    );
    assert_eq!(manifest_codes(&[("Cargo.toml", unrelated.as_str())]), []);
}

#[test]
fn sqlite_projects_without_a_journal_mode_are_reviewed_once() {
    let project = Project::new();
    project.write(
        "Rullst.toml",
        "[mail]\nurl = \"smtp://mail\"\n\n[database]\nurl = \"sqlite://db.sqlite\"\n",
    );
    assert_eq!(
        project.findings(PACKAGE),
        [("Rullst.toml".to_string(), "V13-SQLITE-WAL-AVAILABLE", 5)]
    );
    // Without a URL in `Rullst.toml`, the environment files name the database.
    for name in [".env", ".env.example"] {
        let project = Project::new();
        project.write(
            name,
            "APP_KEY=x\nexport DATABASE_URL=\"sqlite://db.sqlite?mode=rwc\"\n",
        );
        if name == ".env" {
            project.write("Rullst.toml", "[database]\npool_size = 4\n");
        }
        assert_eq!(
            project.findings(PACKAGE),
            [(name.to_string(), "V13-SQLITE-WAL-AVAILABLE", 2)]
        );
    }
}

#[test]
fn a_journal_mode_or_another_database_is_not_reviewed() {
    for (rullst_toml, env) in [
        (
            "[database]\nurl = \"sqlite://db.sqlite\"\nsqlite_journal_mode = \"delete\"\n",
            "",
        ),
        (
            "[database]\nsqlite_journal_mode = \"wal\"\n",
            "DATABASE_URL=sqlite://db.sqlite\n",
        ),
        (
            "[database]\nurl = \"postgres://app@db/app\"\n",
            "DATABASE_URL=sqlite://db.sqlite\n",
        ),
        ("[database]\nurl = \"libsql://app.turso.io\"\n", ""),
        ("[database\nbroken", "DATABASE_URL=sqlite://db.sqlite\n"),
        ("", "DATABASE_URL=postgres://app@db/app\n"),
    ] {
        let project = Project::new();
        project.write("Rullst.toml", rullst_toml).write(".env", env);
        assert_eq!(project.findings(PACKAGE), [], "{rullst_toml}{env}");
    }
    let marked = Project::new();
    marked.write(
        "Rullst.toml",
        "[database]\n# rullst-upgrade: reviewed V13-SQLITE-WAL-AVAILABLE on NFS\nurl = \"sqlite://db.sqlite\"\n",
    );
    let plans = vec![plan(marked.root().join("Cargo.toml"), PACKAGE)];
    let mut found = super::super::files::project_findings(marked.root(), &plans).0;
    let reviewed = super::super::reviewed::drop_reviewed(&mut found);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(reviewed.len(), 1);
}

#[test]
fn facts_and_findings_hold_when_several_declarations_set_them() {
    let both = format!(
        "{PACKAGE}[dependencies]\nrullst = {{ version = \"=13.0.0-alpha.6\", default-features = false, features = [\"auth-session-registry-sqlite\"] }}\nrullst-auth = {{ version = \"=13.0.0-alpha.6\", features = [\"session-registry-sqlite\"] }}\njsonwebtoken = \"11\"\n\n[workspace.dependencies]\njsonwebtoken = \"11\"\n"
    );
    let declared = facts(&[("Cargo.toml", both.as_str())]);
    assert!(declared.session_registry);
    assert!(declared.jsonwebtoken_below_12);
    let referenced = format!(
        "{PACKAGE}[dependencies]\nrullst = {{ version = \"=13.0.0-alpha.6\", default-features = false, optional = true, features = [\"auth-session-registry-sqlite\", \"iot\"] }}\ned25519-dalek = \"2\"\n\n[features]\nsessions = [\"rullst/auth-session-registry-sqlite\"]\ndevices = [\"rullst/iot\"]\n"
    );
    assert!(facts(&[("Cargo.toml", referenced.as_str())]).session_registry);
    assert_eq!(
        manifest_codes(&[("Cargo.toml", referenced.as_str())]),
        [("Cargo.toml".to_string(), "V13-ED25519-DALEK-3", 6)]
    );
    let two = format!(
        "{PACKAGE}[dependencies]\nrullst-iot = \"=13.0.0-alpha.5\"\nrullst = {{ version = \"=13.0.0-alpha.5\", default-features = false, features = [\"iot\"] }}\ned25519-dalek = \"2\"\n"
    );
    assert_eq!(
        manifest_codes(&[("Cargo.toml", two.as_str())]),
        [("Cargo.toml".to_string(), "V13-ED25519-DALEK-3", 7)]
    );
}

#[test]
fn the_new_of_a_define_provider_type_must_change_in_its_package() {
    let source = "rullst_connect::define_provider!(Gitlab, \"read_user\");\nfn connect() {\n    let provider = Gitlab::new(id, secret, url);\n    let checked = Gitlab::try_new(id, secret, url);\n    let other = Other::new(id);\n}\n";
    let found = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    assert_eq!(lines_of("V13-CONNECT-INFALLIBLE-NEW-REMOVED", &found), [3]);

    let root =
        std::env::temp_dir().join(format!("rullst-rules-providers-{}", rand::random::<u64>()));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/main.rs"),
        "fn main() {\n    let provider = providers::Gitlab::new(id, secret, url);\n    let other = Vec::<u8>::new();\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/providers.rs"),
        "define_provider!(Gitlab);\nvec![Other];\n",
    )
    .unwrap();
    let mut scan = SourceScan::default();
    scan_package(&root, &WorkspaceFacts::default(), &mut scan).unwrap();
    let found: Vec<_> = scan
        .findings
        .iter()
        .map(|finding| {
            (
                finding.path.strip_prefix(&root).unwrap().to_path_buf(),
                finding.rule.code,
                finding.line,
            )
        })
        .collect();
    assert_eq!(
        found,
        [(
            PathBuf::from("src/main.rs"),
            "V13-CONNECT-INFALLIBLE-NEW-REMOVED",
            2
        )]
    );
    std::fs::remove_dir_all(root).unwrap();
}
