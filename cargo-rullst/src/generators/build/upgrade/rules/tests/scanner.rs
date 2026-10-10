//! Token-scanner paths that decide individual rules.

use super::*;

fn has(source: &str, code: &str) -> bool {
    codes(source).contains(&code)
}

#[test]
fn scanning_resumes_after_macros_and_calls() {
    // The call after a late macro invocation is still scanned.
    assert!(has(
        "fn a() { let v = 1; let w = 2; x!(); render_page(h, t, b) }",
        "V13-RENDER-PAGE-LANGUAGE"
    ));
    // A call's result keeps the method chain alive.
    assert!(has(
        "async fn a() { Doc::query().remember(60).get().await }",
        "V13-ORM-CACHE-PREFIX"
    ));
}

#[test]
fn context_guards_decide_rules() {
    let media = "fn k() -> FieldKind { FieldKind::Url }";
    assert!(!has(media, "V13-LMS-MEDIA-KINDS"));
    assert!(has(
        &format!("{media} const F: &str = \"captions_url\";"),
        "V13-LMS-MEDIA-KINDS"
    ));

    let hashing = "fn r(p: &str) { hash_password(p); }";
    assert!(has(hashing, "V13-CREDENTIAL-RATE-LIMIT"));
    assert!(!has(
        &format!("{hashing} fn l() {{ credential_rate_limit(); }}"),
        "V13-CREDENTIAL-RATE-LIMIT"
    ));

    let paging = "async fn p() { Doc::query().paginate(page, 20).await; }";
    assert!(has(paging, "V13-PAGE-BOUNDS"));
    assert!(!has(
        &format!("const MAX_PAGE: u64 = 100; {paging}"),
        "V13-PAGE-BOUNDS"
    ));
}

#[test]
fn quota_scheduler_and_model_rules_need_their_exact_shape() {
    assert!(has(
        "fn q(e: QuotaError) -> bool { matches!(e, QuotaError::InvalidRequest) }",
        "V13-CAPITAL-ZERO-TIER"
    ));
    assert!(has(
        "fn s() { let s = Scheduler::new(); }",
        "V13-SCHEDULER-WEEKDAYS"
    ));
    assert!(has(
        "async fn m() { let v = User::all().await; }",
        "V13-MODEL-ALL"
    ));
    assert!(!has(
        "async fn m() { let v = items::all().await; let w = User::all(1).await; }",
        "V13-MODEL-ALL"
    ));
}

#[test]
fn cache_prefix_needs_a_query_or_a_filter_in_the_chain() {
    for source in [
        "async fn a() { Doc::query().remember(60).get().await }",
        "async fn a() { Doc::where_eq(DocColumn::Id, 1).remember(60).get().await }",
        "async fn a() { Doc::filter_by(f).remember(60).get().await }",
    ] {
        assert!(has(source, "V13-ORM-CACHE-PREFIX"), "{source}");
    }
    assert!(!has(
        "async fn a() { cache.remember(60).await }",
        "V13-ORM-CACHE-PREFIX"
    ));
}

#[test]
fn the_progress_key_needs_both_parts() {
    assert!(!has(
        "fn k() -> &'static str { \"progress:{user}\" }",
        "V13-LMS-PROGRESS-KEY"
    ));
    assert!(has(
        "fn k() -> &'static str { \"progress:{user}:next\" }",
        "V13-LMS-PROGRESS-KEY"
    ));
}

#[test]
fn only_an_equals_sign_assigns_a_dynamic_handler() {
    let source = r#"
fn view(h: &str) -> String {
    html! {
        <a> onclick > {h} </a>
        <b data-hx-on-click={h}></b>
        <i hx-on-submit={h}></i>
    }
}
"#;
    let lines: Vec<usize> = scan_with(source, "view.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(code, _)| *code == "V13-HTML-DYNAMIC-EVENT-HANDLER")
        .map(|(_, line)| line)
        .collect();
    assert_eq!(lines, [5, 6], "{lines:?}");
}
