//! 13.0.0-alpha.6: matches without a wildcard arm and struct literals of the
//! Rullst types that became `#[non_exhaustive]`.

use super::*;

const MATCH: &str = "V13-NON-EXHAUSTIVE-MATCH";
const LITERAL: &str = "V13-NON-EXHAUSTIVE-LITERAL";

fn found(source: &str, code: &str) -> Vec<usize> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(found, _)| *found == code)
        .map(|(_, line)| line)
        .collect()
}

#[test]
fn matches_without_a_wildcard_on_rullst_enums_must_change() {
    let source = r#"use rullst::storage::StorageError;
use rullst::capital::SubscriptionStatus;
fn describe(error: StorageError) -> &'static str {
    match error {
        StorageError::NotFound(_) => "missing",
        StorageError::Io(_) | StorageError::Driver(_) => "unavailable",
        StorageError::PathTraversal(_) | StorageError::Unsupported(_) | StorageError::Cloud(_) => "refused",
    }
}
fn paid(status: SubscriptionStatus) -> bool {
    match status {
        SubscriptionStatus::Active | SubscriptionStatus::Trialing => true,
        SubscriptionStatus::Canceled | SubscriptionStatus::PastDue => false,
        SubscriptionStatus::Unpaid | SubscriptionStatus::Paused => false,
    }
}
fn ok(status: SubscriptionStatus) -> bool {
    match status {
        SubscriptionStatus::Active => true,
        _ => false,
    }
}
"#;
    assert_eq!(found(source, MATCH), [4, 11]);
}

#[test]
fn an_application_enum_of_the_same_name_is_not_reported() {
    // Other variants, or a file that names no Rullst crate.
    let own = "use rullst_capital::Plan;\nenum SubscriptionStatus { Open, Closed }\nfn f(s: SubscriptionStatus) -> u8 { match s { SubscriptionStatus::Open => 1, SubscriptionStatus::Closed => 2 } }\n";
    assert_eq!(found(own, MATCH), Vec::<usize>::new());
    let elsewhere = "enum ModelOperation { Created, Removed }\nfn f(o: ModelOperation) -> u8 { match o { ModelOperation::Created => 1, ModelOperation::Removed => 2 } }\n";
    assert_eq!(found(elsewhere, MATCH), Vec::<usize>::new());
    // A decision enum kept exhaustive on purpose is not reported.
    let decision = "use rullst_privacy::age_assurance::AgeDecision;\nfn f(d: AgeDecision) -> bool { match d { AgeDecision::Allowed => true, AgeDecision::BelowMinimumAge | AgeDecision::AlternativeRequired | AgeDecision::Unavailable => false } }\n";
    assert_eq!(found(decision, MATCH), Vec::<usize>::new());
}

#[test]
fn literals_of_rullst_structs_must_change_also_inside_vec() {
    let source = r#"use rullst::nexus::{FieldKind, FieldMeta};
use rullst::connect::provider::ExchangeParams;
fn fields() -> Vec<FieldMeta> {
    vec![
        FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
        FieldMeta::new("title", "Title", FieldKind::Text),
    ]
}
fn params(code: &str) -> ExchangeParams<'_> {
    ExchangeParams {
        auth_code: code,
        ..Default::default()
    }
}
"#;
    assert_eq!(found(source, LITERAL), [5, 10]);
}

#[test]
fn literals_with_fields_of_another_struct_are_not_reported() {
    // The application's own `Message` has a `body`, which Mail's has not.
    let own = "use rullst::mail::Mailer;\nstruct Message { to: String, body: String }\nfn m() -> Message { Message { to: String::new(), body: String::new() } }\n";
    assert_eq!(found(own, LITERAL), Vec::<usize>::new());
    // `openai::` does not name the AI crate.
    let lookalike = "use async_openai::types;\nstruct Message { role: String, content: String }\nfn m() -> Message { Message { role: String::new(), content: String::new() } }\n";
    assert_eq!(found(lookalike, LITERAL), Vec::<usize>::new());
    // Without a path that names the crate.
    let elsewhere = "struct TraceSpan { name: String }\nfn s() -> TraceSpan { TraceSpan { name: String::new() } }\n";
    assert_eq!(found(elsewhere, LITERAL), Vec::<usize>::new());
    // A module of the application named like a Rullst crate.
    let module = "use rullst::auth::Session;\nfn m() -> crate::mail::Message { crate::mail::Message { to: String::new(), subject: String::new() } }\nfn n() -> Message { Message { to: String::new(), subject: String::new() } }\nuse crate::mail::Message;\n";
    assert_eq!(found(module, LITERAL), Vec::<usize>::new());
    // Core's types count in any file that names Rullst.
    let core = "use rullst::telemetry_spans::{TraceSpan, global_span_collector};\nfn r() { global_span_collector().record(TraceSpan { name: String::new(), kind: String::new(), duration_us: 1, timestamp: 2 }); }\n";
    assert_eq!(found(core, LITERAL), [2]);
}

#[test]
fn a_literal_through_crate_self_or_super_is_the_applications_own_type() {
    let fields = "to: String::new(), subject: String::new()";
    // The file names `rullst::mail`, so the same literal through that crate is
    // reported; only the relative first segment keeps the others quiet.
    let rullst = format!("use rullst::mail::Mailer;\nfn m() {{ let _ = rullst::mail::Message {{ {fields} }}; }}\n");
    assert_eq!(found(&rullst, LITERAL), [2]);
    for first in ["crate", "self", "super"] {
        let own = format!("use rullst::mail::Mailer;\nfn m() {{ let _ = {first}::mail::Message {{ {fields} }}; }}\n");
        assert_eq!(found(&own, LITERAL), Vec::<usize>::new(), "{first}");
    }
}

#[test]
fn literals_a_glob_may_bring_in_are_reviewed() {
    const GLOB: &str = "V13-NON-EXHAUSTIVE-LITERAL-GLOB";
    // A test module that reaches Connect's `HttpResponse` through its parent.
    let test_module = "use super::*;\nfn ok(body: String) -> HttpResponse { HttpResponse { status: 200, body } }\nfn err() -> RagRetrievalError { RagRetrievalError(String::new()) }\nfn own() -> Point { Point { x: 1, y: 2 } }\n";
    assert_eq!(found(test_module, GLOB), [2, 3]);
    assert_eq!(found(test_module, LITERAL), Vec::<usize>::new());
    for glob in [
        "use crate::providers::*;",
        "pub use self::client::*;",
        "use rullst::connect::prelude::*;",
    ] {
        let source = format!(
            "{glob}\nfn ok(body: String) -> HttpResponse {{ HttpResponse {{ status: 200, body }} }}\n"
        );
        let (literal, review) = (found(&source, LITERAL), found(&source, GLOB));
        // A Rullst glob also names the crate, so the literal must change.
        if glob.contains("rullst") {
            assert_eq!((literal, review), (vec![2], vec![]), "{glob}");
        } else {
            assert_eq!((literal, review), (vec![], vec![2]), "{glob}");
        }
    }
    // Without a glob, or with a glob of another crate, nothing is reported.
    for header in ["", "use std::collections::*;\n", "use superb::*;\n"] {
        let source = format!(
            "{header}fn ok(body: String) -> HttpResponse {{ HttpResponse {{ status: 200, body }} }}\n"
        );
        assert!(found(&source, GLOB).is_empty(), "{header}");
    }
}

#[test]
fn calls_of_rullst_tuple_structs_must_change() {
    let source = "use rullst::ai::ai::rag::{RagAuditError, RagRetrievalError};\nfn e() -> RagRetrievalError {\n    RagRetrievalError(\"down\".to_string())\n}\nfn a() -> RagAuditError { RagAuditError::new(\"down\") }\n";
    assert_eq!(found(source, LITERAL), [3]);
    let own = "struct RagRetrievalError(String);\nfn e() -> RagRetrievalError { RagRetrievalError(String::new()) }\n";
    assert_eq!(found(own, LITERAL), Vec::<usize>::new());
}

#[test]
fn a_mail_message_literal_reports_both_releases() {
    let source = "use rullst::mail::Message;\nfn m() -> Message { Message { to: String::new(), subject: String::new(), body_html: None } }\nfn n() -> Message { Message { to: String::new(), ..Message::new() } }\n";
    let mut codes = scan_with(source, "lib.rs", &WorkspaceFacts::default());
    codes.sort_unstable();
    assert_eq!(
        codes,
        [("V13-MAIL-MESSAGE-REPLY-TO", 2), (LITERAL, 2), (LITERAL, 3)]
    );
}

#[test]
fn test_code_is_reported_because_it_no_longer_compiles_either() {
    // Must-change rules are not among the production-only rules that skip
    // test code: a test with these literals or matches fails to build too.
    let source = "use rullst::capital::{SubscriptionStatus, WebhookEvent};\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn paid() {\n        let event = WebhookEvent { subscription_id: String::new(), customer_id: String::new(), customer_email: String::new(), plan_id: String::new(), status: SubscriptionStatus::Active, ends_at: None };\n        match event.status {\n            SubscriptionStatus::Active | SubscriptionStatus::Trialing | SubscriptionStatus::Paused => {}\n            SubscriptionStatus::Canceled | SubscriptionStatus::PastDue | SubscriptionStatus::Unpaid => {}\n        }\n    }\n}\n";
    assert_eq!(found(source, LITERAL), [7]);
    assert_eq!(found(source, MATCH), [8]);
    let in_tests_directory = scan_with(source, "tests/billing.rs", &WorkspaceFacts::default());
    assert!(
        in_tests_directory.contains(&(LITERAL, 7)),
        "{in_tests_directory:?}"
    );
    assert!(
        in_tests_directory.contains(&(MATCH, 8)),
        "{in_tests_directory:?}"
    );
}

#[test]
fn media_types_are_reported_in_files_that_name_rullst_media() {
    let source = "use rullst_media::{PlaybackKind, RemoteVideo, WebhookHeaders};\nfn player(kind: PlaybackKind) -> &'static str {\n    match kind {\n        PlaybackKind::Embed => \"iframe\",\n        PlaybackKind::Hls | PlaybackKind::Mp4_720p | PlaybackKind::Original => \"video\",\n    }\n}\nfn headers<'a>(v: &'a str, a: &'a str, s: &'a str) -> WebhookHeaders<'a> {\n    WebhookHeaders { version: v, algorithm: a, signature: s }\n}\n";
    assert_eq!(found(source, MATCH), [3]);
    assert_eq!(found(source, LITERAL), [9]);
    // `Action` and `Scope` stay exhaustive on purpose.
    let kept = "use rullst_media::{Action, Scope};\nfn f(a: Action) -> u8 { match a { Action::Manage => 1, Action::Play => 2 } }\n";
    assert_eq!(found(kept, MATCH), Vec::<usize>::new());
}
