use super::*;

fn request() -> OneOffCheckoutRequest {
    OneOffCheckoutRequest::new(
        "order_42",
        4_990,
        "BRL",
        "Aula de capoeira",
        "https://app.example/orders/42/paid",
        "https://app.example/orders/42",
    )
    .expect("valid request")
}

#[test]
fn request_normalizes_currency_and_multiplies_the_quantity() {
    let request = request();
    assert_eq!(request.reference(), "order_42");
    assert_eq!(request.currency(), "brl");
    assert_eq!(request.quantity(), 1);
    assert_eq!(request.total_amount_minor(), 4_990);
    assert_eq!(request.customer_email(), None);
    let group = request
        .clone()
        .with_quantity(3)
        .unwrap()
        .with_customer_email("aluno@example.com")
        .unwrap();
    assert_eq!(group.total_amount_minor(), 14_970);
    assert_eq!(group.unit_amount_minor(), 4_990);
    assert_eq!(group.customer_email(), Some("aluno@example.com"));
    assert_eq!(group.description(), "Aula de capoeira");
    assert_ne!(group.request_digest(), request.request_digest());
    assert_eq!(request.request_digest(), self::request().request_digest());
    assert!(request.idempotency_key().starts_with("rullst-one-off-"));
    assert_eq!(request.idempotency_key().len(), 15 + 64);
    let debug = format!("{group:?}");
    assert!(!debug.contains("aluno@example.com") && !debug.contains("order_42"));
    assert!(debug.starts_with("OneOffCheckoutRequest"));
    assert!(debug.contains("quantity: 3") && debug.contains("[REDACTED]"));
}

#[test]
fn request_bounds_are_inclusive() {
    let build = |reference: &str, amount: u64, currency: &str, description: &str| {
        OneOffCheckoutRequest::new(
            reference,
            amount,
            currency,
            description,
            "https://app.example/a",
            "https://app.example/b",
        )
    };
    assert!(build(&"r".repeat(200), 99_999_999, "usd", &"d".repeat(250)).is_ok());
    for (reference, amount, currency, description) in [
        ("", 1, "usd", "d"),
        ("r".repeat(201).as_str(), 1, "usd", "d"),
        ("user@example.com", 1, "usd", "d"),
        ("r", 0, "usd", "d"),
        ("r", 100_000_000, "usd", "d"),
        ("r", 1, "us", "d"),
        ("r", 1, "us1", "d"),
        ("r", 1, "usd", ""),
        ("r", 1, "usd", " d"),
        ("r", 1, "usd", "line\nbreak"),
        ("r", 1, "usd", "d".repeat(251).as_str()),
    ] {
        assert!(
            matches!(
                build(reference, amount, currency, description),
                Err(CapitalError::InvalidCharge(_))
            ),
            "{reference:?} {amount} {currency:?} {description:?}"
        );
    }
    assert!(request().with_quantity(0).is_err());
    assert!(request().with_quantity(MAX_ONE_OFF_QUANTITY + 1).is_err());
    assert_eq!(
        build("r", 9_999, "usd", "d")
            .unwrap()
            .with_quantity(10_000)
            .unwrap()
            .total_amount_minor(),
        99_990_000
    );
    assert!(
        build("r", 10_000, "usd", "d")
            .unwrap()
            .with_quantity(10_000)
            .is_err()
    );
    let largest = build("r", 99_999_999, "usd", "d")
        .unwrap()
        .with_quantity(1)
        .unwrap();
    assert_eq!(largest.total_amount_minor(), 99_999_999);
    assert!(
        build("r", 50_000_000, "usd", "d")
            .unwrap()
            .with_quantity(2)
            .is_err()
    );
    let longest = format!("{}@example.com", "a".repeat(320 - 12));
    assert_eq!(longest.len(), 320);
    assert!(request().with_customer_email(longest.as_str()).is_ok());
    assert!(
        request()
            .with_customer_email(format!("a{longest}"))
            .is_err()
    );
    assert!(request().with_customer_email("a@b").is_ok());
    for email in [
        "",
        "a b@example.com",
        "no-at",
        "a@b@c",
        "@example.com",
        "a@",
        "a\u{1}@b.c",
    ] {
        assert!(request().with_customer_email(email).is_err(), "{email:?}");
    }
}

#[test]
fn redirects_accept_loopback_http_for_local_development() {
    for (success, cancel) in [
        ("http://localhost:3000/paid", "http://127.0.0.1:3000/cancel"),
        ("https://app.example/paid", "http://[::1]/cancel"),
    ] {
        OneOffCheckoutRequest::new("r", 100, "brl", "d", success, cancel).expect(success);
    }
    for url in [
        "http://app.example/paid",
        "https://app.example/paid#fragment",
        "https://user:secret@app.example/paid",
    ] {
        assert!(matches!(
            OneOffCheckoutRequest::new("r", 100, "brl", "d", url, "https://app.example/c"),
            Err(CapitalError::ConfigurationError(_))
        ));
    }
}

#[test]
fn checkout_binds_a_verified_https_url_and_session() {
    let request = request();
    let checkout = OneOffCheckout::from_verified_provider_response(
        "acmepay",
        &request,
        "https://pay.acme.example/c/42",
    )
    .unwrap()
    .with_session_id("sess_42")
    .unwrap();
    assert_eq!(checkout.url(), "https://pay.acme.example/c/42");
    assert_eq!(checkout.session_id(), Some("sess_42"));
    assert_eq!(checkout.provider(), "acmepay");
    assert_eq!(checkout.reference(), "order_42");
    assert_eq!(checkout.request_digest(), request.request_digest());
    assert!(!checkout.is_mock());
    for url in [
        "http://pay.acme.example/c",
        "https://pay.acme.example/c#x",
        "javascript:x",
    ] {
        assert!(OneOffCheckout::from_verified_provider_response("acmepay", &request, url).is_err());
    }
    assert!(checkout.clone().with_session_id("bad id").is_err());
    let mock = OneOffCheckout::mock("acmepay", &request);
    assert!(mock.is_mock());
    assert!(
        mock.url()
            .starts_with("https://mock.acmepay.invalid/checkout/one_off_")
    );
    let debug = format!("{checkout:?}");
    assert!(!debug.contains("pay.acme.example"));
    assert!(debug.starts_with("OneOffCheckout") && debug.contains("acmepay"));
}

#[test]
fn notifications_carry_identifiers_only() {
    let notification = PaymentNotification::new("order_42")
        .unwrap()
        .with_transaction_id("2c7b-41ad_9.x")
        .unwrap()
        .with_invoice_id("slug-1")
        .unwrap();
    assert_eq!(notification.reference(), "order_42");
    assert_eq!(notification.transaction_id(), Some("2c7b-41ad_9.x"));
    assert_eq!(notification.invoice_id(), Some("slug-1"));
    notification.require_reference_of(&request()).unwrap();
    let other = PaymentNotification::new("order_43").unwrap();
    assert!(matches!(
        other.require_reference_of(&request()),
        Err(CapitalError::InvalidCharge(_))
    ));
    for invalid in ["", "a b", "a/b", "x".repeat(256).as_str()] {
        assert!(matches!(
            PaymentNotification::new("order_42")
                .unwrap()
                .with_transaction_id(invalid),
            Err(CapitalError::PayloadParseError(_))
        ));
        assert!(
            PaymentNotification::new("order_42")
                .unwrap()
                .with_invoice_id(invalid)
                .is_err()
        );
    }
    assert!(PaymentNotification::new("order 42").is_err());
    assert!(PaymentNotification::new("x".repeat(255)).is_err());
    assert!(
        PaymentNotification::new("o")
            .unwrap()
            .with_transaction_id("x".repeat(255))
            .is_ok()
    );
}

#[test]
fn a_paid_confirmation_must_match_the_server_side_total_and_currency() {
    let request = request().with_quantity(2).unwrap();
    let paid = OneOffPayment::paid("acmepay", &request, 9_980, 10_400, "BRL")
        .unwrap()
        .with_transaction_id("txn_1")
        .unwrap();
    assert!(paid.is_paid());
    assert_eq!(paid.state(), OneOffPaymentState::Paid);
    assert_eq!(paid.amount_minor(), 9_980);
    assert_eq!(paid.paid_amount_minor(), 10_400);
    assert_eq!(paid.currency(), "brl");
    assert_eq!(paid.reference(), "order_42");
    assert_eq!(paid.provider(), "acmepay");
    assert_eq!(paid.transaction_id(), Some("txn_1"));
    assert_eq!(paid.request_digest(), request.request_digest());
    OneOffPayment::paid("acmepay", &request, 9_980, 9_980, "brl").unwrap();

    // Another link that reuses the reference with another price never counts.
    for (charged, paid_amount, currency) in [
        (100, 100, "brl"),
        (9_981, 9_981, "brl"),
        (9_980, 9_979, "brl"),
        (9_980, 9_980, "usd"),
    ] {
        assert!(matches!(
            OneOffPayment::paid("acmepay", &request, charged, paid_amount, currency),
            Err(CapitalError::InvalidCharge(_))
        ));
    }
    let unpaid = OneOffPayment::unpaid("acmepay", &request);
    assert!(!unpaid.is_paid());
    assert_eq!(unpaid.paid_amount_minor(), 0);
    assert_eq!(unpaid.amount_minor(), 9_980);
    let mock = OneOffPayment::mock("acmepay", &request);
    assert_eq!(mock.state(), OneOffPaymentState::Mock);
    assert!(!mock.is_paid());
    let debug = format!("{paid:?}");
    assert!(!debug.contains("order_42"));
    assert!(debug.starts_with("OneOffPayment") && debug.contains("Paid"));
    assert!(matches!(
        OneOffPayment::unpaid("acmepay", &request).with_transaction_id("bad id"),
        Err(CapitalError::PayloadParseError(_))
    ));
}

#[test]
fn request_digest_is_a_sha256_known_answer() {
    // SHA-256 of the v1 tag, the big-endian amount (u64) and quantity (u32),
    // then each length-prefixed field, computed independently.
    assert_eq!(
        hex::encode(request().request_digest()),
        "4d5c56d95de660d8096c8543f8f6d455a605f2da2b56ae0c5d1c3926b08141f7"
    );
}

#[test]
fn a_notification_url_is_validated_and_bound_to_the_digest() {
    let url = "https://app.example/webhooks/42?token=t";
    let notified = request().with_notification_url(url).unwrap();
    assert_eq!(notified.notification_url(), Some(url));
    assert_eq!(request().notification_url(), None);
    // Without a URL the alpha.5 digest is unchanged (see the known answer
    // above); with one, the URL is hashed after the other fields.
    assert_eq!(
        hex::encode(notified.request_digest()),
        "69b56485bfb1d426fd5056ddae793f825ade8ec8e8ee9ed8ef7f52b1292912c7"
    );
    assert_ne!(notified.idempotency_key(), request().idempotency_key());
    assert!(!format!("{notified:?}").contains("token"));
    for invalid in [
        "http://app.example/hook",
        "http://localhost:3000/hook",
        "https://user:pass@app.example/hook",
        "https://app.example/hook#t",
        "/hook",
    ] {
        assert!(
            matches!(
                request().with_notification_url(invalid),
                Err(CapitalError::ConfigurationError(_))
            ),
            "{invalid}"
        );
    }
}

#[test]
fn a_checkout_reports_the_notification_url_only_once_sent() {
    let url = "https://app.example/webhooks/42";
    let notified = request().with_notification_url(url).unwrap();
    let checkout = OneOffCheckout::from_verified_provider_response(
        "acmepay",
        &notified,
        "https://pay.acme.example/c/1",
    )
    .unwrap();
    assert_eq!(checkout.notification_url(), None);
    let sent = checkout.clone().with_notification_url_sent();
    assert_eq!(sent.notification_url(), Some(url));
    assert_ne!(sent, checkout);
    let plain = OneOffCheckout::mock("acmepay", &request()).with_notification_url_sent();
    assert_eq!(plain.notification_url(), None);
    assert_eq!(
        OneOffCheckout::mock("acmepay", &notified)
            .with_notification_url_sent()
            .notification_url(),
        Some(url)
    );
}

#[test]
fn payment_methods_map_provider_names() {
    assert_eq!(
        PaymentMethod::from_provider_name("pix").unwrap(),
        PaymentMethod::Pix
    );
    assert_eq!(
        PaymentMethod::from_provider_name("PIX").unwrap(),
        PaymentMethod::Pix
    );
    assert_eq!(
        PaymentMethod::from_provider_name("credit_card").unwrap(),
        PaymentMethod::CreditCard
    );
    let other = PaymentMethod::from_provider_name("Boleto-2").unwrap();
    assert!(matches!(&other, PaymentMethod::Other(name) if name.as_str() == "boleto-2"));
    assert_eq!(other.as_str(), "boleto-2");
    assert_eq!(PaymentMethod::Pix.as_str(), "pix");
    assert_eq!(PaymentMethod::CreditCard.to_string(), "credit_card");
    assert_eq!(other.to_string(), "boleto-2");
    assert!(PaymentMethod::from_provider_name(&"x".repeat(64)).is_ok());
    for invalid in [
        "",
        "credit card",
        "pix.",
        "pix/1",
        "é",
        "x".repeat(65).as_str(),
    ] {
        assert!(
            matches!(
                PaymentMethod::from_provider_name(invalid),
                Err(CapitalError::PayloadParseError(_))
            ),
            "{invalid:?}"
        );
    }
}

#[test]
fn a_payment_records_its_method_and_installments() {
    let request = request();
    let paid = OneOffPayment::paid("acmepay", &request, 4_990, 5_100, "brl").unwrap();
    assert_eq!((paid.payment_method(), paid.installments()), (None, None));
    let paid = paid
        .with_payment_method(PaymentMethod::CreditCard)
        .with_installments(3)
        .unwrap();
    assert_eq!(paid.payment_method(), Some(&PaymentMethod::CreditCard));
    assert_eq!(paid.installments(), Some(3));
    assert_eq!(
        paid.clone().with_installments(1).unwrap().installments(),
        Some(1)
    );
    assert!(matches!(
        paid.clone().with_installments(0),
        Err(CapitalError::PayloadParseError(_))
    ));
    let debug = format!("{paid:?}");
    assert!(
        debug.contains("payment_method: Some(CreditCard)")
            && debug.contains("installments: Some(3)"),
        "{debug}"
    );
}
