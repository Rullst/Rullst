//! Rules decided by a function body: bounded password hashing in request
//! code and the LMS progress claim.

use super::*;

const CREDENTIAL: &str = "V13-CREDENTIAL-RATE-LIMIT";
const PROGRESS: &str = "V13-LMS-RECORD-PROGRESS";

/// The lines of `code` in `source`, scanned as the file at `path`.
fn lines_at(source: &str, path: &str, code: &str) -> Vec<usize> {
    let context = rust::FileContext {
        text: source,
        file_name: Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap(),
        path: Path::new(path),
        facts: &WorkspaceFacts::default(),
    };
    let mut collector = Collector::default();
    rust::scan_source(&context, &mut collector).expect("fixture parses");
    let mut findings = Vec::new();
    collector.finish(Path::new(path), &mut findings);
    findings
        .into_iter()
        .filter(|finding| finding.rule.code == code)
        .map(|finding| finding.line)
        .collect()
}

/// Calls through the generated `run_password_work` helper, as in the
/// `cargo rullst auth` controllers, need no review.
#[test]
fn hashing_through_the_generated_bounded_helper_is_not_reviewed() {
    let generated = r#"
pub async fn change_password(form: Form<Change>) -> Response {
    let hash = run_password_work(move || rullst_auth::hash_password(&password)).await;
    let other = crate::controllers::auth_controller::run_password_work(move || {
        rullst::auth::hash_password(&password)
    })
    .await;
}
"#;
    assert!(
        lines_at(
            generated,
            "src/controllers/password_controller.rs",
            CREDENTIAL
        )
        .is_empty()
    );
    let unbounded = r#"
pub async fn register(form: Form<Register>) -> Response {
    let checked = run_password_work(move || verify(&password)).await;
    let hash = rullst::auth::hash_password(&form.password);
}
pub async fn later() { rullst_auth::hash_password_async(password).await; }
"#;
    // A review rule reports every location in a file.
    assert_eq!(
        lines_at(unbounded, "src/controllers/auth_controller.rs", CREDENTIAL),
        [4, 6]
    );
    let async_only = "pub async fn r(p: String) { rullst::auth::hash_password_async(p).await; }";
    assert_eq!(lines_at(async_only, "src/lib.rs", CREDENTIAL), [1]);
}

#[test]
fn a_semaphore_permit_bounds_the_hashing_of_its_function() {
    for permit in [
        "let _permit = PERMITS.acquire().await;",
        "let _permit = Arc::clone(&PERMITS).acquire_owned().await;",
        "let _permit = PERMITS.try_acquire_many(1);",
    ] {
        let source = format!(
            "async fn register(p: String) {{ {permit} spawn_blocking(move || hash_password(&p)); }}"
        );
        assert!(
            lines_at(&source, "src/lib.rs", CREDENTIAL).is_empty(),
            "{source}"
        );
    }
    // `acquire` must be a method call; another function's permit bounds
    // nothing here.
    for unbounded in [
        "async fn register(p: String) { acquire(); hash_password(&p); }",
        "async fn register(p: String) { let a = s.acquire; hash_password(&p); }",
        "async fn other() { s.acquire().await; } async fn register(p: String) { hash_password(&p); }",
    ] {
        assert_eq!(
            lines_at(unbounded, "src/lib.rs", CREDENTIAL),
            [1],
            "{unbounded}"
        );
    }
}

/// Only the middleware's name in code silences the rule: a comment, a doc
/// comment or a string mentioning it does not.
#[test]
fn only_code_that_names_the_rate_limit_counts() {
    let hashing = "async fn register(p: String) { hash_password(&p); }\n";
    for silent in [
        "// credential_rate_limit is applied in main.rs\n",
        "/// See credential_rate_limit.\nconst NOTE: u8 = 1;\n",
        "const NOTE: &str = \"credential_rate_limit\";\n",
        "#[doc = \"credential_rate_limit\"]\nconst DOC: u8 = 1;\n",
    ] {
        let source = format!("{silent}{hashing}");
        assert!(
            !lines_at(&source, "src/lib.rs", CREDENTIAL).is_empty(),
            "{source}"
        );
    }
    for applied in [
        "fn routes(r: Router) -> Router { r.layer(from_fn(auth_controller::credential_rate_limit)) }\n",
        "pub async fn credential_rate_limit(request: Request, next: Next) -> Response { next.run(request).await }\n",
    ] {
        let source = format!("{applied}{hashing}");
        assert!(
            lines_at(&source, "src/lib.rs", CREDENTIAL).is_empty(),
            "{source}"
        );
    }
}

/// Seeds, other binaries, tests and examples serve no request; test
/// functions and `#[cfg(test)]` modules in request code are skipped too.
#[test]
fn hashing_outside_request_code_is_not_reviewed() {
    let hashing = "pub async fn seed(p: String) { rullst::auth::hash_password(&p); }";
    for offline in [
        "src/migrations/demo.rs",
        "src/seeds/users.rs",
        "src/seeders/users.rs",
        "src/bin/create_admin.rs",
        "tests/auth.rs",
        "benches/hashing.rs",
        "examples/demo.rs",
        "build.rs",
    ] {
        assert!(
            lines_at(hashing, offline, CREDENTIAL).is_empty(),
            "{offline}"
        );
    }
    for request in [
        "src/services/passwords.rs",
        "src/main.rs",
        "src/migrations.rs",
    ] {
        assert_eq!(lines_at(hashing, request, CREDENTIAL), [1], "{request}");
    }
    let tests = r#"
#[cfg(test)]
mod tests {
    fn helper(p: &str) { hash_password(p); }
}
#[test]
fn hashes() { hash_password("x"); }
#[tokio::test]
async fn hashes_async() { hash_password("x"); }
#[cfg(test)]
impl Fixture { fn user(p: &str) { hash_password(p); } }
#[cfg(not(test))]
fn production(p: &str) { hash_password(p); }
"#;
    assert_eq!(lines_at(tests, "src/lib.rs", CREDENTIAL), [13]);
}

#[test]
fn hashing_definitions_and_attributes_are_not_calls() {
    let source = r#"
fn hash_password(p: &str) -> String { p.to_string() }
impl Hasher { fn hash_password(&self, p: &str) {} }
fn outer() { fn hash_password(p: &str) {} }
fn other() { #![inner(hash_password(p))] #[check(hash_password(p))] let _ = 1; }
"#;
    assert!(lines_at(source, "src/lib.rs", CREDENTIAL).is_empty());
    assert_eq!(
        lines_at(
            "fn outer(p: &str) { fn f() {} hash_password(p); }",
            "src/lib.rs",
            CREDENTIAL
        ),
        [1]
    );
}

/// The 12.x LMS service read and upserted the progress before inserting
/// the event that claims the key.
const OLD_RECORD_PROGRESS: &str = r#"
pub async fn record_progress(user_id: i32, key: &str) -> Result<(), Error> {
    let replay_sql = match driver {
        "postgres" => "SELECT subject_user_id FROM lesson_progress_events WHERE event_key = $1",
        _ => "SELECT subject_user_id FROM lesson_progress_events WHERE event_key = ?",
    };
    let current = "SELECT progress_percent FROM lesson_progress WHERE user_id = ?";
    let upsert = "INSERT INTO lesson_progress (user_id, lesson_id) VALUES (?, ?)";
    let event = "INSERT INTO lesson_progress_events (event_key) VALUES (?)";
    Ok(())
}
"#;

#[test]
fn record_progress_is_reviewed_only_when_it_writes_before_the_claim() {
    assert_eq!(
        lines_at(
            OLD_RECORD_PROGRESS,
            "src/services/learning_service.rs",
            PROGRESS
        ),
        [2]
    );
    // The current service claims the key with its first write; the
    // controller and the route only name the function.
    let current = r#"
pub async fn record_progress(user_id: i32, key: &str) -> Result<(), Error> {
    if let Some(receipt) = replay(driver, user_id, key).await? { return Ok(receipt); }
    let event_sql = match driver {
        "postgres" => "INSERT INTO lesson_progress_events (event_key, previous_percent) VALUES ($1, COALESCE((SELECT progress_percent FROM lesson_progress WHERE user_id = $2), 0))",
        _ => "insert into `lesson_progress_events`(event_key) values (?)",
    };
    let upsert = "INSERT INTO lesson_progress (user_id) VALUES (?)";
    Ok(())
}
pub async fn record_progress_handler() { learning_service::record_progress(1, "k").await; }
"#;
    assert!(lines_at(current, "src/services/learning_service.rs", PROGRESS).is_empty());
    let controller = r#"
pub async fn record_progress(Form(form): Form<Progress>) -> Response {
    match learning_service::record_progress(&context, form.user_id, &form.key).await { _ => redirect("/") }
}
fn routes() { routes! { post("/progress" => record_progress) } }
"#;
    assert!(
        lines_at(
            controller,
            "src/controllers/learning_controller.rs",
            PROGRESS
        )
        .is_empty()
    );
    // Another function, a body without progress events, a write-free body
    // and a claim spelled differently decide nothing; an UPDATE first does.
    for silent in [
        OLD_RECORD_PROGRESS.replace("fn record_progress", "fn save_progress"),
        "fn record_progress() { let s = \"INSERT INTO progress (a) VALUES (?)\"; }".to_string(),
        "fn record_progress() { let s = \"SELECT 1 FROM lesson_progress_events\"; }".to_string(),
        "fn record_progress() { let s = \"INSERT INTO lesson_progress_events\"; }".to_string(),
    ] {
        assert!(
            lines_at(&silent, "src/lib.rs", PROGRESS).is_empty(),
            "{silent}"
        );
    }
    for reported in [
        "fn record_progress() { let a = \"UPDATE lesson_progress SET p = 1\"; let b = \"INSERT INTO lesson_progress_events (k) VALUES (?)\"; }",
        "fn record_progress() { let a = \"DELETE FROM lesson_progress\"; let b = \"INSERT INTO lesson_progress_events (k) VALUES (?)\"; }",
        "fn record_progress() { let a = \"REPLACE INTO lesson_progress (p) VALUES (1)\"; let b = \"INSERT INTO lesson_progress_events (k) VALUES (?)\"; }",
        "fn record_progress() { let b = \"INSERT INTO lesson_progress_events_archive (k) VALUES (?)\"; }",
        "impl S { fn record_progress(&self) { let a = \"  insert   into lesson_progress (p) values (1)\"; let b = \"INSERT INTO lesson_progress_events (k) VALUES (?)\"; } }",
    ] {
        assert_eq!(
            lines_at(reported, "src/lib.rs", PROGRESS),
            [1],
            "{reported}"
        );
    }
    assert!(
        lines_at(
            &format!("#[cfg(test)]\nmod tests {{ {OLD_RECORD_PROGRESS} }}"),
            "src/lib.rs",
            PROGRESS
        )
        .is_empty()
    );
}

/// The plataforma's `src/auth.rs`: `hash_password` runs Rullst's hasher under
/// its own `run_password_work`, whose semaphore permit bounds it, and the
/// file applies `credential_rate_limit` to the public login.
const PLATFORM_AUTH: &str = r#"
use rullst::async_runtime::tokio::sync::Semaphore;
static PASSWORD_WORK: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(4)));
pub async fn run_password_work<T, F>(work: F) -> Result<T, PasswordWorkError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let permit = rullst::runtime::time::timeout(WAIT, Arc::clone(&*PASSWORD_WORK).acquire_owned()).await;
    let _permit = permit.map_err(|_| PasswordWorkError::Busy)?;
    spawn_blocking(work).await.map_err(|_| PasswordWorkError::Failed)
}
pub async fn hash_password(password: String) -> Result<String, String> {
    match run_password_work(move || rullst::auth::hash_password(&password)).await {
        Ok(Ok(hash)) => Ok(hash),
        _ => Err("busy".into()),
    }
}
pub async fn credential_rate_limit(request: Request, next: Next) -> Response {
    next.run(request).await
}
"#;

/// The three pages of the plataforma report (item 46), each calling the
/// application's `auth::hash_password` from another module.
const PLATFORM_PAGES: [(&str, &str); 3] = [
    (
        "src/pages/admin.rs",
        "use crate::auth;\nuse rullst::html;\npub async fn guardian(form: Form<G>) -> Response {\n    let hash = auth::hash_password(form.password.clone()).await;\n    page(hash)\n}\n",
    ),
    (
        "src/pages/admin_import.rs",
        "use crate::{auth, db, util};\npub async fn import(rows: Vec<Row>) {\n    for row in rows {\n        let hash = auth::hash_password(row.password).await;\n    }\n}\n",
    ),
    (
        "src/pages/family.rs",
        "use crate::auth;\npub async fn change(form: Form<C>) -> Response {\n    let ok = auth::run_password_work(move || verify(&form.current)).await;\n    let hash = auth::hash_password(form.new.clone()).await;\n    page(ok, hash)\n}\n",
    ),
];

/// The `V13-CREDENTIAL-RATE-LIMIT` findings of a package holding `files`,
/// as (path, line), and the rules that markers left out.
fn package_findings(files: &[(&str, &str)]) -> (Vec<(String, usize)>, Vec<&'static str>) {
    let root = std::env::temp_dir().join(format!("rullst-rules-hashers-{}", rand::random::<u64>()));
    for (path, text) in files {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let mut scan = SourceScan::default();
    scan_package(&root, &WorkspaceFacts::default(), &mut scan).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let found = scan
        .findings
        .iter()
        .filter(|finding| finding.rule.code == CREDENTIAL)
        .map(|finding| {
            let relative = finding.path.strip_prefix(&root).unwrap();
            (relative.to_string_lossy().replace('\\', "/"), finding.line)
        })
        .collect();
    let reviewed = scan.reviewed.iter().map(|rule| rule.code).collect();
    (found, reviewed)
}

fn with_pages(auth: &'static str) -> Vec<(&'static str, &'static str)> {
    std::iter::once(("src/auth.rs", auth))
        .chain(PLATFORM_PAGES)
        .collect()
}

#[test]
fn a_call_of_the_applications_bounded_hasher_in_another_module_is_not_reviewed() {
    assert_eq!(
        package_findings(&with_pages(PLATFORM_AUTH)),
        (vec![], vec![])
    );
    // The same package whose `hash_password` hashes without a bound: each
    // call is reported, and the wrapper's own file applies the rate limit.
    let unbounded = "pub async fn hash_password(password: String) -> Result<String, String> {\n    rullst::auth::hash_password(&password).map_err(|e| e.to_string())\n}\npub async fn credential_rate_limit(r: Request, n: Next) -> Response { n.run(r).await }\n";
    let expected = vec![
        ("src/pages/admin.rs".to_string(), 4),
        ("src/pages/admin_import.rs".to_string(), 4),
        ("src/pages/family.rs".to_string(), 4),
    ];
    assert_eq!(
        package_findings(&with_pages(unbounded)),
        (expected.clone(), vec![])
    );
    // Without any definition of `hash_password` in the package, too.
    assert_eq!(package_findings(&PLATFORM_PAGES), (expected, vec![]));
    // A semaphore permit in the wrapper bounds it as well.
    let permit = "pub async fn hash_password(p: String) -> String {\n    let _permit = PERMITS.acquire().await;\n    spawn_blocking(move || rullst::auth::hash_password(&p)).await\n}\n";
    assert_eq!(package_findings(&with_pages(permit)), (vec![], vec![]));
    // A wrapper that calls the helper but also hashes outside it is not bounded.
    let leaky = "pub async fn hash_password(p: String) -> String {\n    let _ = run_password_work(move || warm()).await;\n    rullst::auth::hash_password(&p)\n}\npub async fn credential_rate_limit(r: Request, n: Next) -> Response { n.run(r).await }\n";
    assert_eq!(package_findings(&with_pages(leaky)).0.len(), 3);
}

#[test]
fn a_reviewed_marker_resolves_a_limit_the_rule_does_not_follow() {
    let marked = "use crate::auth;\npub async fn guardian(form: Form<G>) -> Response {\n    // rullst-upgrade: reviewed V13-CREDENTIAL-RATE-LIMIT: auth::hash_password runs under auth::run_password_work\n    let hash = auth::hash_password(form.password.clone()).await;\n    page(hash)\n}\n";
    assert_eq!(
        package_findings(&[("src/pages/admin.rs", marked)]),
        (vec![], vec![CREDENTIAL])
    );
    let message = CATALOG
        .iter()
        .find(|rule| rule.code == CREDENTIAL)
        .unwrap()
        .message;
    assert!(
        message.contains(
            "`// rullst-upgrade: reviewed V13-CREDENTIAL-RATE-LIMIT: <reason>` on the line above the call"
        ),
        "{message}"
    );
}

#[test]
fn calls_that_reach_rullsts_hasher_stay_reported_next_to_a_bounded_wrapper() {
    for (label, page) in [
        (
            "rullst::auth imported as auth",
            "use rullst::auth;\npub async fn a(p: String) {\n    let _ = auth::hash_password(&p);\n}\n",
        ),
        (
            "rullst_auth renamed",
            "use rullst_auth as auth;\npub async fn a(p: String) {\n    let _ = auth::hash_password(&p);\n}\n",
        ),
        (
            "an unqualified import",
            "use rullst::auth::{hash_password, verify_password};\npub async fn a(p: String) {\n    let _ = hash_password(&p);\n}\n",
        ),
        (
            "a Rullst glob",
            "use rullst::prelude::*;\npub async fn a(p: String) {\n    let _ = hash_password(&p);\n}\n",
        ),
        (
            "an external crate",
            "// No import.\npub async fn a(p: String) {\n    let _ = passwords::hash_password(&p);\n}\n",
        ),
        (
            "a glob of the application's module",
            "use crate::auth::*;\npub async fn a(p: String) {\n    let _ = hash_password(&p);\n}\n",
        ),
        (
            "a full path",
            "// No import.\npub async fn a(p: String) {\n    let _ = ::rullst::auth::hash_password(&p);\n}\n",
        ),
        (
            "a method",
            "// No import.\npub async fn a(p: String) {\n    let _ = argon.hash_password(p.as_bytes(), &salt);\n}\n",
        ),
        (
            "a qualified type",
            "// No import.\npub async fn a(p: String) {\n    let _ = <Argon2 as PasswordHasher>::hash_password(&argon, p.as_bytes(), &salt);\n}\n",
        ),
    ] {
        assert_eq!(
            package_findings(&[("src/auth.rs", PLATFORM_AUTH), ("src/pages/a.rs", page)]).0,
            [("src/pages/a.rs".to_string(), 3)],
            "{label}"
        );
    }
    // An application path wins over a Rullst glob, and `mod` declares one.
    for page in [
        "use rullst::prelude::*;\nuse crate::auth;\npub async fn a(p: String) {\n    let _ = auth::hash_password(p).await;\n}\n",
        "mod auth;\npub async fn a(p: String) {\n    let _ = auth::hash_password(p).await;\n}\n",
        "pub async fn a(p: String) {\n    let _ = crate::auth::hash_password(p).await;\n    let _ = super::auth::hash_password(p).await;\n}\n",
        "use crate::auth::hash_password;\npub async fn a(p: String) {\n    let _ = hash_password(p).await;\n}\n",
    ] {
        assert_eq!(
            package_findings(&[("src/auth.rs", PLATFORM_AUTH), ("src/pages/a.rs", page)]),
            (vec![], vec![]),
            "{page}"
        );
    }
}

#[test]
fn a_single_file_resolves_calls_of_its_own_bounded_hasher() {
    let own = "pub async fn hash_password(p: String) -> String {\n    run_password_work(move || rullst::auth::hash_password(&p)).await\n}\npub async fn register(p: String) {\n    let _ = hash_password(p).await;\n    let _ = rullst::auth::hash_password(\"x\");\n}\n";
    // Rullst's hasher outside the helper is still reported, at its own line.
    assert_eq!(lines_at(own, "src/lib.rs", CREDENTIAL), [6]);
    let unresolved = "pub async fn register(p: String) {\n    let _ = crate::auth::hash_password(p).await;\n    let _ = hash_password_async(p).await;\n}\n";
    assert_eq!(lines_at(unresolved, "src/lib.rs", CREDENTIAL), [2, 3]);
}
