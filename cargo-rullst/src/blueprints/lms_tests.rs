use super::file_manifest;

fn manifest(hot_reload: bool) -> Vec<(&'static str, String)> {
    file_manifest("demo", hot_reload, "Active Record", "Zero-Bundle HTMX")
}

fn source<'a>(manifest: &'a [(&'static str, String)], name: &str) -> &'a str {
    manifest
        .iter()
        .find(|(path, _)| *path == name)
        .map(|(_, source)| source.as_str())
        .unwrap_or_default()
}

#[test]
fn nexus_accepts_the_lesson_media_values_the_player_plays() {
    let manifest = file_manifest("demo", false, "Active Record", "Zero-Bundle HTMX");
    let lesson = source(&manifest, "src/models/lesson.rs");
    let migration = source(
        &manifest,
        "src/migrations/m20260601000000_create_lms_tables.rs",
    );
    // Nexus `Url` fields require an absolute URL, so the seeded same-origin
    // captions made every seeded video lesson uneditable.
    assert!(migration.contains("'/static/media/memory-safety.en.vtt'"));
    for field in ["media_url", "captions_url"] {
        let meta = lesson
            .lines()
            .find(|line| line.contains(&format!("name: \"{field}\"")))
            .unwrap_or_default();
        assert!(meta.contains("kind: FieldKind::Text,"), "{field}: {meta}");
    }
    assert!(lesson.contains(
        "name: \"media_kind\", label: \"Media Kind\", kind: FieldKind::Enum { options: vec![\"video\", \"audio\"] }"
    ));
    let controller = source(&manifest, "src/controllers/learning_controller.rs");
    assert!(!controller.contains("Err(_) => StatusCode::SERVICE_UNAVAILABLE"));
    assert!(controller.contains("eprintln!(\"Lesson {lesson_id} cannot be played: {error:?}\");"));
}

#[test]
fn starter_is_small_explicit_and_excludes_academy_verticals() {
    let manifest = manifest(false);
    for required in [
        "src/services/learning_service.rs",
        "src/controllers/learning_controller.rs",
        "src/models/course.rs",
        "src/models/enrollment.rs",
        "src/pages/lms.rs",
        "static/media/memory-safety.en.vtt",
    ] {
        assert!(
            manifest.iter().any(|(path, _)| *path == required),
            "{required}"
        );
    }
    for excluded in [
        "src/models/quiz.rs",
        "src/models/achievement.rs",
        "src/services/automation_worker_service.rs",
        "src/services/notification_service.rs",
        "rullst-lms-modules.json",
        "src/lib.rs",
    ] {
        assert!(
            manifest.iter().all(|(path, _)| *path != excluded),
            "{excluded}"
        );
    }
    assert!(
        manifest.len() < 30,
        "starter emitted {} files",
        manifest.len()
    );
    assert_eq!(
        manifest
            .iter()
            .filter(|(path, _)| *path == "src/main.rs")
            .count(),
        1
    );
    assert!(
        !source(&manifest, "src/controllers/auth_controller.rs")
            .contains("provision_self_registration_with_tx")
    );
}

#[test]
fn lesson_progress_keys_are_fresh_per_render_and_scoped_per_percentage() {
    let manifest = manifest(false);
    let controller = source(&manifest, "src/controllers/learning_controller.rs");
    // One fixed key per learner and lesson made every later save a 409.
    assert!(!controller.contains(":next"));
    assert!(controller.contains("let progress_key = new_progress_key(user_id, lesson_id);"));
    assert!(controller.contains("rullst::security::generate_csrf_token()"));
    assert!(
        controller.contains("&progress_event_key(&form.idempotency_key, form.progress_percent)")
    );
    assert!(
        controller
            .contains("fn each_render_and_requested_percentage_records_its_own_progress_event()")
    );
}

#[test]
fn progress_idempotency_keys_are_scoped_to_their_learner() {
    let manifest = manifest(false);
    let service = source(&manifest, "src/services/learning_service.rs");
    let migration = source(
        &manifest,
        "src/migrations/m20260827000000_add_learning_access.rs",
    );
    // A key unique across all learners let one learner submit another's
    // key first and turn every later save of the victim into a 409.
    assert!(migration.contains("ON lesson_progress_events(subject_user_id, event_key)\""));
    assert!(!migration.contains("ON lesson_progress_events(event_key)"));
    assert!(
        service.contains(
            "FROM lesson_progress_events WHERE subject_user_id = $1 AND event_key = $2\""
        )
    );
    assert!(
        service
            .contains("FROM lesson_progress_events WHERE subject_user_id = ? AND event_key = ?\"")
    );
    assert!(service.contains(".bind(user_id).bind(idempotency_key).fetch_optional(pool)"));
}

#[test]
fn a_concurrent_resubmission_replays_instead_of_failing() {
    let manifest = manifest(false);
    let service = source(&manifest, "src/services/learning_service.rs");
    // Checking the key before the transaction and inserting the event
    // last let a double click pass the check twice; the loser hit the
    // unique index (or SQLITE_BUSY after its read) and returned 503.
    let transaction = service
        .split_once("let mut transaction = pool.begin().await")
        .map(|(_, body)| body)
        .unwrap_or_default();
    let claim = transaction
        .find("INSERT INTO lesson_progress_events")
        .expect("the transaction claims the key");
    let upsert = transaction
        .find("INSERT INTO lesson_progress (")
        .expect("the transaction records progress");
    assert!(claim < upsert, "the key must be claimed by the first write");
    assert!(!transaction[..claim].contains("SELECT"));
    assert!(service.contains(
        "COALESCE((SELECT progress_percent FROM lesson_progress WHERE user_id = $5 AND lesson_id = $6), 0)"
    ));
    assert!(service.contains(
        "Err(rullst::db::sqlx::Error::Database(error)) if error.is_unique_violation() => {"
    ));
    assert_eq!(
        service
            .matches("replay(driver, user_id, lesson_id, progress_percent, idempotency_key).await?")
            .count(),
        2
    );
}

#[test]
fn dashboard_describes_the_schoolless_starter() {
    let manifest = manifest(false);
    let pages = source(&manifest, "src/pages/auth.rs");
    let controller = source(&manifest, "src/controllers/auth_controller.rs");
    // v13 provisions no school, yet the dashboard claimed one.
    assert!(!pages.to_lowercase().contains("school"));
    assert!(!controller.to_lowercase().contains("school"));
    assert!(pages.contains("Enroll in a course from the catalog"));
    assert!(pages.contains("<a href=\"/\">Browse courses</a>"));
}

#[test]
fn hot_reload_exports_the_router_library() {
    let manifest = manifest(true);
    assert!(source(&manifest, "src/lib.rs").contains("pub extern \"C\" fn rullst_router_init()"));
    assert!(source(&manifest, "src/lib.rs").contains("pub fn router()"));
    assert!(source(&manifest, "src/main.rs").contains("rullst::Server::new_hot(lib_path)"));
    assert!(source(&manifest, "src/main.rs").contains("demo::router()?"));
    assert_eq!(
        manifest
            .iter()
            .filter(|(path, _)| *path == "src/main.rs")
            .count(),
        1
    );
}

#[test]
fn generated_catalog_is_bounded_parameterized_and_nonce_compatible() {
    let manifest = manifest(false);
    let controller = source(&manifest, "src/controllers/lms_controller.rs");
    let page = source(&manifest, "src/pages/lms.rs");
    assert!(controller.contains(".where_like(\"title\","));
    assert!(controller.contains(".where_eq(\"category_id\","));
    assert!(controller.contains(".limit(MAX_CATALOG_RESULTS)"));
    assert!(controller.contains("Option<Extension<rullst::security::CspNonce>>"));
    assert!(page.contains("<style nonce={csp_nonce}>"));
    assert!(!page.contains("style="));
    assert!(!page.contains("https://"));
    assert!(!page.contains("<script"));
    assert!(!page.contains("hx-"));
}

#[test]
fn generated_catalog_has_keyboard_and_accessibility_landmarks() {
    let manifest = manifest(false);
    let page = source(&manifest, "src/pages/lms.rs");
    for marker in [
        "Skip to course results",
        ":focus-visible",
        "prefers-reduced-motion",
        "aria-live=\"polite\"",
        "role=\"search\"",
        "captions and transcripts",
    ] {
        assert!(page.contains(marker), "missing marker: {marker}");
    }
}

#[test]
fn player_is_accessible_bounded_nonce_based_and_does_not_autoplay() {
    let manifest = manifest(false);
    let page = source(&manifest, "src/pages/lms.rs");
    assert!(page.contains("<style nonce={csp_nonce}>"));
    assert!(page.contains("<track kind=\"captions\""));
    assert!(page.contains("<audio controls=\"controls\""));
    assert!(page.contains("InvalidTranscript"));
    assert!(page.contains("value.len() > 2_048"));
    assert!(!page.contains("autoplay"));
    assert!(!page.contains("style="));
    assert!(!page.contains("hx-"));
}
