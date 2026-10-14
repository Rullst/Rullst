use super::*;
use crate::error::CapitalError;

fn invoice(amounts: &[f64], total: f64, currency: &str) -> Invoice {
    Invoice {
        invoice_id: "INV-2026-7".to_string(),
        customer_email: "owner@example.com".to_string(),
        date: Utc::now(),
        items: amounts
            .iter()
            .map(|amount| InvoiceItem {
                description: "Plan".to_string(),
                amount: *amount,
            })
            .collect(),
        total,
        currency: currency.to_string(),
    }
}

fn rejection(invoice: &Invoice) -> String {
    match invoice.total_minor() {
        Err(CapitalError::InvalidInvoice(message)) => message,
        other => panic!("expected an invalid invoice, got {other:?}"),
    }
}

#[test]
fn currency_must_be_exactly_three_ascii_letters() {
    for currency in ["US1", "USDX", "US"] {
        assert_eq!(
            rejection(&invoice(&[1.0], 1.0, currency)),
            "currency must be exactly three ASCII letters",
            "{currency}"
        );
    }
}

#[test]
fn item_count_is_one_to_128() {
    assert_eq!(
        invoice(&[1.0; 128], 128.0, "BRL").total_minor().unwrap(),
        12_800
    );
    for count in [0, 129] {
        assert_eq!(
            rejection(&invoice(&vec![1.0; count], count.max(1) as f64, "BRL")),
            "invoice must contain 1 to 128 items",
            "{count}"
        );
    }
}

#[test]
fn amounts_must_be_finite_and_positive() {
    for amount in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        assert_eq!(
            rejection(&invoice(&[1.0, amount], 1.0, "BRL")),
            "amounts must be finite and greater than zero",
            "{amount}"
        );
    }
    assert_eq!(
        rejection(&invoice(&[1.0], f64::NAN, "BRL")),
        "amounts must be finite and greater than zero"
    );
}

#[test]
fn amounts_fit_eight_digit_minor_units() {
    assert_eq!(
        invoice(&[99_999_999.0], 99_999_999.0, "JPY")
            .total_minor()
            .unwrap(),
        99_999_999
    );
    assert!(
        rejection(&invoice(&[100_000_000.0], 100_000_000.0, "JPY"))
            .starts_with("amounts must fit 1 to 99,999,999 minor units")
    );
}

#[test]
fn text_fields_are_bounded_trimmed_and_free_of_control_characters() {
    let mut longest = invoice(&[1.0], 1.0, "BRL");
    longest.invoice_id = "I".repeat(128);
    longest.items[0].description = "d".repeat(512);
    assert_eq!(longest.total_minor().unwrap(), 100);

    for invoice_id in [
        String::new(),
        "I".repeat(129),
        " INV".into(),
        "IN\u{1}V".into(),
    ] {
        let mut invalid = invoice(&[1.0], 1.0, "BRL");
        invalid.invoice_id = invoice_id.clone();
        assert!(
            rejection(&invalid).starts_with("invoice ID must contain 1 to 128"),
            "{invoice_id:?}"
        );
    }
    let mut long_item = invoice(&[1.0], 1.0, "BRL");
    long_item.items[0].description = "d".repeat(513);
    assert!(rejection(&long_item).starts_with("invoice item description must contain 1 to 512"));
}

#[test]
fn email_must_not_contain_whitespace() {
    let mut invalid = invoice(&[1.0], 1.0, "BRL");
    invalid.customer_email = "own er@example.com".to_string();
    assert_eq!(
        rejection(&invalid),
        "customer email must contain one non-empty local and domain part"
    );
}

#[test]
fn validated_html_renders_the_invoice() {
    let html = invoice(&[39.9, 10.0], 49.9, "BRL")
        .try_generate_html()
        .unwrap();
    assert!(html.contains("Invoice INV-2026-7"));
    assert!(html.contains(">49.90 BRL<"));
    assert!(html.contains("owner@example.com"));
}

#[test]
fn an_amount_that_rounds_to_zero_minor_units_is_rejected() {
    // 1e-7 yen and 5e-10 dollars are positive and within the rounding
    // tolerance, but neither is a whole minor unit.
    for (amounts, total, currency) in [([1.0, 1e-7], 1.0, "JPY"), ([1.0, 5e-10], 1.0, "USD")] {
        assert!(
            rejection(&invoice(&amounts, total, currency))
                .starts_with("amounts must fit 1 to 99,999,999 minor units"),
            "{currency}"
        );
    }
    assert_eq!(invoice(&[0.01], 0.01, "USD").total_minor().unwrap(), 1);
    assert_eq!(invoice(&[1.0], 1.0, "JPY").total_minor().unwrap(), 1);
}
