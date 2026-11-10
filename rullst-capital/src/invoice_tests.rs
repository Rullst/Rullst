use super::*;
use crate::error::CapitalError;

fn invoice(amounts: &[u64], currency: &str) -> Invoice {
    amounts.iter().fold(
        Invoice::new("INV-2026-7", "owner@example.com", Utc::now(), currency),
        |invoice, amount| invoice.with_item("Plan", *amount),
    )
}

fn rejection(invoice: &Invoice) -> String {
    match invoice.total_minor() {
        Err(CapitalError::InvalidInvoice(message)) => message,
        other => panic!("expected an invalid invoice, got {other:?}"),
    }
}

#[test]
fn invoice_html_escapes_every_untrusted_text_field() {
    let mut invoice = Invoice::new("<INV&1>", "a\"b@example.com", Utc::now(), "U&SD")
        .with_item("<script>alert('x')</script>", 150);
    invoice.labels.title = "<b>Invoice</b>".to_string();
    let html = invoice.generate_html();
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<b>"));
    assert!(html.contains("&lt;script&gt;alert(&#x27;x&#x27;)&lt;/script&gt;"));
    assert!(html.contains("&lt;INV&amp;1&gt;"));
    assert!(html.contains("a&quot;b@example.com"));
    assert!(html.contains("U&amp;SD"));
}

#[test]
fn the_total_is_the_exact_sum_of_integer_minor_units() {
    let invoice = invoice(&[3_990, 1_000], "BRL");
    assert_eq!(invoice.total_minor().expect("valid money"), 4_990);
    let html = invoice.try_generate_html().expect("valid invoice");
    assert!(html.contains(">39.90 BRL<"));
    assert!(html.contains(">49.90 BRL<"));
    assert!(html.contains("Invoice INV-2026-7"));
    assert!(html.contains("owner@example.com"));
}

#[test]
fn minor_units_follow_the_currency_exponent() {
    assert_eq!(invoice(&[2_500], "JPY").total_minor().unwrap(), 2_500);
    assert_eq!(invoice(&[12_345], "kwd").total_minor().unwrap(), 12_345);
    let html = invoice(&[2_500], "JPY").generate_html();
    assert!(html.contains(">2500 JPY<"), "{html}");
    assert!(!html.contains("2500.00"));
    assert!(
        invoice(&[12_345], "KWD")
            .generate_html()
            .contains(">12.345 KWD<")
    );
    assert!(invoice(&[1], "usd").generate_html().contains(">0.01 USD<"));
}

#[test]
fn currency_must_be_exactly_three_ascii_letters() {
    for currency in ["US1", "USDX", "US", "R$"] {
        assert_eq!(
            rejection(&invoice(&[100], currency)),
            "currency must be exactly three ASCII letters",
            "{currency}"
        );
    }
}

#[test]
fn item_count_is_one_to_128() {
    assert_eq!(invoice(&[100; 128], "BRL").total_minor().unwrap(), 12_800);
    for count in [0, 129] {
        assert_eq!(
            rejection(&invoice(&vec![100; count], "BRL")),
            "invoice must contain 1 to 128 items",
            "{count}"
        );
    }
}

#[test]
fn amounts_and_the_total_fit_eight_digit_minor_units() {
    assert_eq!(
        invoice(&[99_999_999], "JPY").total_minor().unwrap(),
        99_999_999
    );
    for amounts in [vec![0], vec![100_000_000], vec![1, 0]] {
        assert_eq!(
            rejection(&invoice(&amounts, "JPY")),
            "item amounts must contain 1 to 99,999,999 currency minor units",
            "{amounts:?}"
        );
    }
    assert_eq!(
        rejection(&invoice(&[99_999_999, 1], "JPY")),
        "invoice total must not exceed 99,999,999 currency minor units"
    );
    assert_eq!(
        invoice(&[99_999_998, 1], "JPY").total_minor().unwrap(),
        99_999_999
    );
}

#[test]
fn text_fields_are_bounded_trimmed_and_free_of_control_characters() {
    let mut longest = invoice(&[100], "BRL");
    longest.invoice_id = "I".repeat(128);
    longest.items[0].description = "d".repeat(512);
    assert_eq!(longest.total_minor().unwrap(), 100);

    for invoice_id in [
        String::new(),
        "I".repeat(129),
        " INV".into(),
        "IN\u{1}V".into(),
    ] {
        let mut invalid = invoice(&[100], "BRL");
        invalid.invoice_id = invoice_id.clone();
        assert!(
            rejection(&invalid).starts_with("invoice ID must contain 1 to 128"),
            "{invoice_id:?}"
        );
    }
    let mut long_item = invoice(&[100], "BRL");
    long_item.items[0].description = "d".repeat(513);
    assert!(rejection(&long_item).starts_with("invoice item description must contain 1 to 512"));
}

#[test]
fn email_must_be_one_address_without_whitespace() {
    for email in ["own er@example.com", "invalid", "a@b@c"] {
        let mut invalid = invoice(&[100], "BRL");
        invalid.customer_email = email.to_string();
        assert_eq!(
            rejection(&invalid),
            "customer email must contain one non-empty local and domain part",
            "{email}"
        );
    }
}

#[test]
fn built_in_locales_print_receipt_wording_and_the_language() {
    let english = invoice(&[4_990], "BRL").try_generate_html().unwrap();
    for label in [
        "lang=\"en\"",
        ">Invoice INV-2026-7<",
        "Billed To:",
        "Date:",
        ">Description<",
        ">Amount<",
        ">Total<",
    ] {
        assert!(english.contains(label), "{label}");
    }
    let portuguese = invoice(&[4_990], "BRL")
        .with_locale(InvoiceLocale::PtBr)
        .try_generate_html()
        .unwrap();
    for label in [
        "lang=\"pt-BR\"",
        ">Recibo INV-2026-7<",
        "Recebido de:",
        "Data:",
        ">Descrição<",
        ">Valor<",
        ">Total<",
    ] {
        assert!(portuguese.contains(label), "{label}");
    }
    assert!(!portuguese.contains("Invoice") && !portuguese.contains("Billed To"));
    let spanish = invoice(&[4_990], "BRL")
        .with_locale(InvoiceLocale::Es)
        .try_generate_html()
        .unwrap();
    for label in [
        "lang=\"es\"",
        ">Recibo INV-2026-7<",
        "Recibido de:",
        "Fecha:",
        ">Descripción<",
        ">Importe<",
    ] {
        assert!(spanish.contains(label), "{label}");
    }
}

#[test]
fn locale_tags_map_to_the_built_in_labels() {
    for (tag, locale) in [
        ("en", InvoiceLocale::En),
        ("en-US", InvoiceLocale::En),
        ("pt-BR", InvoiceLocale::PtBr),
        ("pt_br", InvoiceLocale::PtBr),
        ("PT", InvoiceLocale::PtBr),
        ("es-MX", InvoiceLocale::Es),
        (" es ", InvoiceLocale::Es),
    ] {
        assert_eq!(InvoiceLocale::from_tag(tag), Some(locale), "{tag}");
    }
    for tag in ["fr", "", "english"] {
        assert_eq!(InvoiceLocale::from_tag(tag), None, "{tag}");
    }
    assert_eq!(InvoiceLocale::default(), InvoiceLocale::En);
    assert_eq!(InvoiceLocale::PtBr.tag(), "pt-BR");
    assert_eq!(InvoiceLabels::default(), InvoiceLocale::En.labels());
    assert_eq!(InvoiceLocale::Es.labels().language, "es");
}

#[test]
fn custom_labels_are_validated_before_rendering() {
    let mut labels = InvoiceLocale::En.labels();
    labels.title = "Receipt".to_string();
    labels.language = "en-GB".to_string();
    let html = invoice(&[100], "GBP")
        .with_labels(labels.clone())
        .try_generate_html()
        .unwrap();
    assert!(html.contains(">Receipt INV-2026-7<") && html.contains("lang=\"en-GB\""));

    let changes: [fn(&mut InvoiceLabels); 6] = [
        |labels: &mut InvoiceLabels| labels.total = String::new(),
        |labels: &mut InvoiceLabels| labels.amount = "x".repeat(65),
        |labels: &mut InvoiceLabels| labels.date = " Date".to_string(),
        |labels: &mut InvoiceLabels| labels.billed_to = "To\nBcc".to_string(),
        |labels: &mut InvoiceLabels| labels.language = "en\"><script>".to_string(),
        |labels: &mut InvoiceLabels| labels.language = String::new(),
    ];
    for change in changes {
        let mut invalid = labels.clone();
        change(&mut invalid);
        assert!(matches!(
            invoice(&[100], "GBP")
                .with_labels(invalid)
                .try_generate_html(),
            Err(CapitalError::InvalidInvoice(_))
        ));
    }
    let mut longest = labels;
    longest.amount = "a".repeat(64);
    longest.language = "x".repeat(35);
    assert!(
        invoice(&[100], "GBP")
            .with_labels(longest)
            .validate()
            .is_ok()
    );
}

#[test]
fn serialized_invoices_use_minor_units_and_default_to_english_labels() {
    let invoice = invoice(&[4_990], "BRL").with_locale(InvoiceLocale::PtBr);
    let json = serde_json::to_value(&invoice).unwrap();
    assert_eq!(json["items"][0]["amount_minor"], 4_990);
    assert!(json.get("total").is_none());
    assert_eq!(json["labels"]["title"], "Recibo");
    let mut without_labels = json.clone();
    without_labels.as_object_mut().unwrap().remove("labels");
    let restored: Invoice = serde_json::from_value(without_labels).unwrap();
    assert_eq!(restored.labels, InvoiceLabels::default());
    assert_eq!(restored.total_minor().unwrap(), 4_990);
    // A 13.0.0-alpha.4 body with floating-point amounts no longer decodes.
    let legacy = serde_json::json!({
        "invoice_id": "INV-1", "customer_email": "owner@example.com",
        "date": "2026-11-10T00:00:00Z", "currency": "BRL", "total": 49.9,
        "items": [{"description": "Plan", "amount": 49.9}]
    });
    assert!(serde_json::from_value::<Invoice>(legacy).is_err());
}
