use super::*;

#[tokio::test]
async fn test_wise_provider_payout_lifecycle() {
    let provider = WiseProvider::new("mock_wise_token", "sec_wise123");
    assert_eq!(provider.name(), "wise");

    // 1. Send payout
    let transfer_id = provider
        .send_payout("beneficiary@wise.com", 15000, "USD", "Invoice 1234")
        .await
        .unwrap();
    assert!(transfer_id.starts_with("wise_tr_"));

    // 2. Validation errors
    assert!(provider.send_payout("", 1000, "USD", "desc").await.is_err());
    assert!(
        provider
            .send_payout("a@b.com", 0, "USD", "desc")
            .await
            .is_err()
    );
    assert!(
        provider
            .send_payout("a@b.com", 1000, "", "desc")
            .await
            .is_err()
    );

    // 3. Status
    let status = provider.get_payout_status(&transfer_id).await.unwrap();
    assert_eq!(status, PayoutStatus::OutgoingPaymentSent);
    assert!(provider.get_payout_status("").await.is_err());

    // 4. Webhook payload parsing
    let payload = r#"{
        "data": {
            "resource": {
                "id": 987654321,
                "recipient_email": "payee@wise.com",
                "amount": 250.75,
                "currency": "EUR"
            },
            "current_state": "outgoing_payment_sent"
        }
    }"#;
    let event = provider.parse_webhook_payload(payload.as_bytes()).unwrap();
    assert_eq!(event.transfer_id, "987654321");
    assert_eq!(event.recipient_email, "payee@wise.com");
    assert_eq!(event.amount_cents, 25075);
    assert_eq!(event.currency, "EUR");
    assert_eq!(event.status, PayoutStatus::OutgoingPaymentSent);

    // Other states
    let fixture = |state: &str| {
        format!(
            r#"{{"data":{{"resource":{{"id":1,"recipient_email":"payee@wise.com","amount":"10","currency":"EUR"}},"current_state":"{state}"}}}}"#
        )
    };
    for (state, expected) in [
        ("funds_refunded", PayoutStatus::FundsRefunded),
        ("cancelled", PayoutStatus::Cancelled),
        ("incoming_payment_waiting", PayoutStatus::Processing),
        ("funds_converted", PayoutStatus::Processing),
    ] {
        let event = provider
            .parse_webhook_payload(fixture(state).as_bytes())
            .unwrap();
        assert_eq!(event.status, expected);
    }
    for state in ["other", "charged_back", "bounced_back"] {
        assert!(
            provider
                .parse_webhook_payload(fixture(state).as_bytes())
                .is_err()
        );
    }

    // Webhook error paths
    assert!(provider.parse_webhook_payload(b"invalid json").is_err());
}

fn fixture_event(resource: Value, state: Option<&str>) -> Result<PayoutEvent, CapitalError> {
    let mut body = serde_json::json!({"data": {"resource": resource}});
    if let Some(state) = state {
        body["data"]["current_state"] = Value::from(state);
    }
    WiseProvider::new("mock_wise_token", "profile")
        .parse_webhook_payload(&serde_json::to_vec(&body).unwrap())
}

#[test]
fn fixture_amounts_are_exact_minor_units_without_float_truncation() {
    let amount = |amount: Value, currency: &str| {
        fixture_event(
            serde_json::json!({"id": 7, "recipient_email": "payee@wise.com", "amount": amount, "currency": currency}),
            Some("outgoing_payment_sent"),
        )
        .map(|event| event.amount_cents)
    };
    for (value, currency, expected) in [
        (serde_json::json!(19.99), "EUR", 1999),
        (serde_json::json!(0.29), "USD", 29),
        (serde_json::json!(1.15), "GBP", 115),
        (serde_json::json!(100.0), "USD", 10000),
        (serde_json::json!("19.90"), "EUR", 1990),
        (serde_json::json!(1000), "JPY", 1000),
        (serde_json::json!(1000.0), "JPY", 1000),
        (serde_json::json!("1.234"), "KWD", 1234),
    ] {
        assert_eq!(
            amount(value.clone(), currency).unwrap(),
            expected,
            "{value} {currency}"
        );
    }
    for (value, currency) in [
        (serde_json::json!(-1), "USD"),
        (serde_json::json!(-0.5), "USD"),
        (serde_json::json!(0), "USD"),
        (serde_json::json!("19.999"), "USD"),
        (serde_json::json!(1.5), "JPY"),
        (serde_json::json!(1e21), "USD"),
        (serde_json::json!("1."), "USD"),
        (serde_json::json!(".5"), "USD"),
        (serde_json::json!("18446744073709551615"), "USD"),
        (serde_json::json!("ten"), "USD"),
        (serde_json::json!(true), "USD"),
    ] {
        assert!(
            amount(value.clone(), currency).is_err(),
            "{value} {currency}"
        );
    }
}

#[test]
fn fixture_fields_are_required_instead_of_invented() {
    let complete = serde_json::json!({"id": 7, "recipient_email": "payee@wise.com", "amount": 5, "currency": "EUR"});
    assert!(fixture_event(complete.clone(), Some("processing")).is_ok());
    assert!(fixture_event(complete.clone(), None).is_err());
    for field in ["id", "recipient_email", "amount", "currency"] {
        let mut resource = complete.clone();
        resource.as_object_mut().unwrap().remove(field);
        assert!(
            fixture_event(resource, Some("processing")).is_err(),
            "{field}"
        );
    }
    for (field, wrong) in [
        ("id", serde_json::json!(0)),
        ("id", serde_json::json!(-3)),
        ("id", serde_json::json!("7")),
        ("recipient_email", serde_json::json!("  ")),
        ("currency", serde_json::json!("usd")),
        ("currency", serde_json::json!("EURO")),
    ] {
        let mut resource = complete.clone();
        resource[field] = wrong;
        assert!(
            fixture_event(resource, Some("processing")).is_err(),
            "{field}"
        );
    }
}
