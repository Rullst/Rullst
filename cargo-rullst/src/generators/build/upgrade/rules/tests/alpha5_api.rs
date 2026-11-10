//! 13.0.0-alpha.5: one path per item, crate-private internals, private
//! `SecurityStore` collections, Rullst-owned error payloads and
//! `AiError::HttpStatus`.

use super::*;

fn found(source: &str) -> Vec<(&'static str, usize)> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
}

fn lines_of(source: &str, code: &str) -> Vec<usize> {
    found(source)
        .into_iter()
        .filter(|(found, _)| *found == code)
        .map(|(_, line)| line)
        .collect()
}

#[test]
fn removed_alias_paths_must_change_in_paths_and_use_trees() {
    let source = r#"
use rullst::rullst_orm::Orm;
use rullst::{
    security_runtime::VaultSecret,
    web,
};
use rullst_mail::mail::*;
use rullst_mail::drivers::mock::OfflineMailMock;
fn f() {
    let _ = rullst::mail::mail::Message::new();
    let _ = rullst::mail::drivers::mock::credential_mode("x");
    let _ = rullst::rullst_orm::Orm::pool();
}
"#;
    assert_eq!(
        lines_of(source, "V13-API-ALIAS-PATHS"),
        vec![2, 4, 7, 8, 10, 11, 12]
    );
    assert_eq!(
        lines_of(
            "use rullst::{\n    rullst_orm as orm_alias,\n};",
            "V13-API-ALIAS-PATHS"
        ),
        vec![2]
    );
    let rule = CATALOG
        .iter()
        .find(|rule| rule.code == "V13-API-ALIAS-PATHS")
        .unwrap();
    assert_eq!(rule.kind, FindingKind::MustChange);
}

#[test]
fn the_kept_paths_are_not_reported() {
    assert_only(
        r#"
use rullst::orm::Orm;
use rullst::orm as rullst_orm;
use rullst::security::runtime::VaultSecret;
use rullst_mail::{mock::OfflineMailMock, Mail, Message};
use rullst_orm::Error;
fn f(store: &Other) {
    let _ = rullst_mail::mock::credential_mode("x");
    let _ = rullst::mail::Mail::send;
    let _ = store.live_events;
    let _ = store.banned_ips.len();
}
"#,
        &[],
    );
}

#[test]
fn hidden_internals_are_reviewed_by_module_path_or_distinctive_name() {
    let source = r#"
use rullst_orm::tenant::CURRENT_TENANT;
use rullst::artisan::runner::check_and_run_artisan;
fn f() {
    let _ = rullst_orm::schema::QUERY_LOGGING.load(Ordering::SeqCst);
    let _ = rullst_orm::privacy::encrypt_aes_gcm("x", "k");
    let _ = rullst_security::get_real_rss_memory_mb();
    let _ = rullst_connect::client::DEFAULT_HTTP_CLIENT.clone();
}
"#;
    let found = found(source);
    let rows: Vec<_> = found
        .iter()
        .filter(|(code, _)| *code == "V13-HIDDEN-INTERNALS-PRIVATE")
        .collect();
    // A review rule reports its first location in a file.
    assert_eq!(rows, vec![&("V13-HIDDEN-INTERNALS-PRIVATE", 2)]);
    for source in [
        "fn f() { let _ = rullst_orm::privacy::encrypt_aes_gcm(\"x\", \"k\"); }",
        "fn f() { let _ = rullst_security::current_timestamp_str(); }",
        "use rullst_connect::client::DEFAULT_HTTP_CLIENT;",
        "use rullst_auth::passkey::cbor::parse_cbor;",
        "fn f() { rullst_core::server::dylib_loader::load(); }",
        "fn f() { let _ = schema::MAX_QUERY_LIMIT; }",
    ] {
        assert_only(source, &["V13-HIDDEN-INTERNALS-PRIVATE"]);
    }
    assert_only("fn f() { let _ = CURRENT_TENANT; let _ = runner; }", &[]);
}

#[test]
fn security_store_fields_are_reviewed_only_beside_the_store() {
    let source = r#"
use rullst_security::SecurityStore;
fn f() {
    let store = SecurityStore::global();
    let events = store.live_events.lock().unwrap();
    let bans = store.active_bans();
}
"#;
    assert_eq!(lines_of(source, "V13-SECURITY-STORE-COLLECTIONS"), vec![5]);
    assert_only(
        "fn f(t: &rullst_security::SecurityTelemetry) { t.honeypot_route_hits.len(); }",
        &["V13-SECURITY-STORE-COLLECTIONS"],
    );
    assert_only(
        "fn f(s: &SecurityStore) { s.live_events(); s.is_banned(\"x\"); }",
        &[],
    );
}

#[test]
fn changed_error_payloads_and_ai_http_statuses_are_reviewed() {
    assert_only(
        "fn f(e: AiError) { if let AiError::RequestError(inner) = e { inner.url(); } }",
        &["V13-ERROR-PAYLOADS"],
    );
    for source in [
        "fn f(e: AiError) { matches!(e, AiError::SerializationError(_)); }",
        "fn f(e: ClientContractError) { matches!(e, ClientContractError::InvalidJson(_)); }",
        "fn f(e: ClientContractError) { matches!(e, ClientContractError::EncodeJson(_)); }",
        "fn f(e: TwinSyncError) { match e { TwinSyncError::Serialization(s) => {}, _ => {} } }",
    ] {
        assert_only(source, &["V13-ERROR-PAYLOADS"]);
    }
    assert_only(
        "fn f(e: AiError) -> bool { matches!(e, AiError::ApiError(m) if m.contains(\"429\")) }",
        &["V13-AI-HTTP-STATUS"],
    );
    assert_only(
        "fn f(e: AiError) -> bool { matches!(e, AiError::HttpStatus(h) if h.status() == 429) }",
        &[],
    );
}

#[test]
fn removed_internals_repeat_their_guidance_on_a_failed_gate() {
    for code in [
        "V13-HIDDEN-INTERNALS-PRIVATE",
        "V13-SECURITY-STORE-COLLECTIONS",
    ] {
        assert!(
            REMOVED_API_RULES.iter().any(|rule| rule.code == code),
            "{code}"
        );
    }
}
