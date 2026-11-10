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
    // A review rule reports its first location in a file.
    assert_eq!(
        lines_at(unbounded, "src/controllers/auth_controller.rs", CREDENTIAL),
        [4]
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
