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
    // Core's types count in any file that names Rullst.
    let core = "use rullst::telemetry_spans::{TraceSpan, global_span_collector};\nfn r() { global_span_collector().record(TraceSpan { name: String::new(), kind: String::new(), duration_us: 1, timestamp: 2 }); }\n";
    assert_eq!(found(core, LITERAL), [2]);
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
