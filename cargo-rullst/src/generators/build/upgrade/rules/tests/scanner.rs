//! Token-scanner paths that decide individual rules.

#![allow(clippy::expect_used)]

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

/// `codes` on a helper thread, failing instead of hanging.
fn bounded_codes(source: &'static str) -> Vec<&'static str> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(codes(source));
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("the scan returns promptly")
}

#[test]
fn field_kind_number_counts_only_when_compared() {
    for source in [
        "fn a(k: &FieldKind) -> bool { *k == FieldKind::Number }",
        "fn a(k: &FieldKind) -> bool { *k != FieldKind::Number }",
        "fn a(k: &FieldKind) -> bool { FieldKind::Number == *k }",
        "fn a(k: &FieldKind) -> bool { FieldKind::Number != *k }",
        "fn a(k: &FieldKind) -> u8 { match k { FieldKind::Number => 1, _ => 0 } }",
        "fn a(k: &FieldKind) -> u8 { match k { FieldKind::Number | FieldKind::Text => 1, _ => 0 } }",
    ] {
        assert!(has(source, "V13-NEXUS-FIELD-KIND-NUMBER"), "{source}");
    }
    for source in [
        "fn a() { let k = FieldKind::Number; }",
        "fn a(k: FieldKind) { let FieldKind::Number = k else { return; }; }",
        "fn a(k: u8) -> bool { FieldKind::Number >= k }",
        "fn a() { FieldKind::Number += 1; }",
        "fn a(k: u8) -> bool { k <= FieldKind::Number }",
    ] {
        assert!(!has(source, "V13-NEXUS-FIELD-KIND-NUMBER"), "{source}");
    }
}

#[test]
fn a_single_joined_colon_does_not_continue_a_path() {
    assert!(has("fn a() { Storage::r2(c); }", "V13-R2-PUBLIC-URL"));
    assert!(!has("fn a() { x!(Storage:&r2(c)); }", "V13-R2-PUBLIC-URL"));
}

#[test]
fn attributes_are_skipped_and_a_lone_hash_is_not_one() {
    for source in [
        "#[doc = \"progress:{u}:next\"] fn a() {}",
        "mod m { #![doc = \"progress:{u}:next\"] }",
    ] {
        assert!(
            !bounded_codes(source).contains(&"V13-LMS-PROGRESS-KEY"),
            "{source}"
        );
    }
    // `#` without a bracket group, as in a quoting macro, is skipped alone.
    assert!(
        bounded_codes("fn a() { x!(# \"progress:{u}:next\"); }").contains(&"V13-LMS-PROGRESS-KEY")
    );
}
