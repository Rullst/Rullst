//! 13.0.0-alpha.5: Capital invoices in integer minor units and Mail's
//! `Message::reply_to`.

use super::*;

fn found(source: &str) -> Vec<(&'static str, usize)> {
    scan_with(source, "billing.rs", &WorkspaceFacts::default())
        .into_iter()
        .filter(|(code, _)| {
            matches!(
                *code,
                "V13-CAPITAL-INVOICE-MINOR-UNITS" | "V13-MAIL-MESSAGE-REPLY-TO"
            )
        })
        .collect()
}

#[test]
fn floating_point_invoice_literals_must_change() {
    let source = r#"
use rullst::capital::{Invoice, InvoiceItem};

fn invoice() -> Invoice {
    Invoice {
        invoice_id: "INV-1".into(),
        customer_email: "a@example.com".into(),
        date: chrono::Utc::now(),
        items: vec![InvoiceItem {
            description: "Plan".into(),
            amount: 49.90,
        }],
        total: 49.90,
        currency: "BRL".into(),
    }
}

fn cheaper(base: Invoice) -> Invoice {
    rullst_capital::invoice::Invoice { total: 1.0, ..base }
}

fn fine() -> Invoice {
    Invoice::new("INV-1", "a@example.com", chrono::Utc::now(), "BRL").with_item("Plan", 4_990)
}
"#;
    assert_eq!(
        found(source),
        [
            ("V13-CAPITAL-INVOICE-MINOR-UNITS", 5),
            ("V13-CAPITAL-INVOICE-MINOR-UNITS", 9),
            ("V13-CAPITAL-INVOICE-MINOR-UNITS", 19),
        ]
    );
}

#[test]
fn application_invoice_types_are_left_alone() {
    // Same field names, but the file never names Capital.
    let own = r#"
struct Invoice { id: i64, total: f64 }
struct InvoiceItem { amount: f64 }
fn build() -> Invoice {
    let _item = InvoiceItem { amount: 1.0 };
    Invoice { id: 1, total: 1.0 }
}
"#;
    assert!(found(own).is_empty());
    // A file that names Capital but builds literals without the old fields.
    let other = r#"
use rullst::capital::StripeProvider;
struct Invoice { id: i64 }
struct InvoiceItem { label: String }
fn build() -> (Invoice, InvoiceItem) {
    (Invoice { id: 1 }, InvoiceItem { label: String::new() })
}
"#;
    assert!(found(other).is_empty());
    // Nested `vec!` bodies are read; a repeat form is skipped.
    let nested = r#"
use rullst_capital::InvoiceItem;
fn items() -> Vec<Vec<InvoiceItem>> {
    vec![vec![InvoiceItem { description: "A".into(), amount: 1.0 }], vec![]]
}
fn repeated() -> Vec<InvoiceItem> { vec![InvoiceItem { description: "A".into(), amount: 1.0 }; 2] }
"#;
    assert_eq!(found(nested), [("V13-CAPITAL-INVOICE-MINOR-UNITS", 4)]);
    assert!(found("fn f() { let _ = rullst_capital::Invoice { total: 1.0, ..x() }; }").len() == 1);
}

#[test]
fn exhaustive_mail_message_literals_must_change() {
    let source = r#"
use rullst::mail::Message;

fn notice() -> Message {
    Message {
        to: "owner@example.com".into(),
        subject: "Booking".into(),
        body_html: None,
        body_text: Some("text".into()),
        from: None,
        unsubscribe_url: None,
        unsubscribe_email: None,
        send_at: None,
        attachments: Vec::new(),
    }
}

fn updated() -> Message {
    Message { subject: "Booking".into(), body_html: None, ..Message::new() }
}

fn complete() -> Message {
    Message {
        to: "owner@example.com".into(),
        subject: "Booking".into(),
        body_html: None,
        body_text: None,
        from: None,
        reply_to: None,
        unsubscribe_url: None,
        unsubscribe_email: None,
        send_at: None,
        attachments: Vec::new(),
    }
}

fn chat() -> ChatMessage {
    ai::Message { role: "user".into(), content: "hi".into() }
}
"#;
    assert_eq!(found(source), [("V13-MAIL-MESSAGE-REPLY-TO", 5)]);
    // Without a Mail path in the file, a `Message` is the application's own.
    let own = source.replace("use rullst::mail::Message;", "use crate::Message;");
    assert!(found(&own).is_empty());
    let crate_path = source.replace("use rullst::mail::Message;", "use rullst_mail::Message;");
    assert_eq!(found(&crate_path), [("V13-MAIL-MESSAGE-REPLY-TO", 5)]);
}

#[test]
fn the_rules_are_must_change_alpha5_rows() {
    for (rule, row) in [
        (
            &catalog::CAPITAL_INVOICE_MINOR_UNITS,
            "Capital invoice minor units",
        ),
        (&catalog::MAIL_MESSAGE_REPLY_TO, "Mail Reply-To"),
    ] {
        assert_eq!(rule.kind, FindingKind::MustChange);
        assert_eq!(rule.introduced, "13.0.0-alpha.5");
        assert_eq!(rule.row, row);
        assert!(CATALOG.iter().any(|listed| listed.code == rule.code));
    }
}
