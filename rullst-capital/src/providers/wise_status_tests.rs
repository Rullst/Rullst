use super::*;
use axum::{Json, Router, extract::Path, http::HeaderMap, routing::get};

async fn transfer_fixture(Path(id): Path<String>, headers: HeaderMap) -> Json<Value> {
    let authorized = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        == Some("Bearer fixture_wise_token");
    let body = match (authorized, id.as_str()) {
        (false, _) => serde_json::json!({"error": "unauthorized"}),
        (_, "1") => serde_json::json!({"id": 1, "status": "outgoing_payment_sent"}),
        (_, "2") => serde_json::json!({"id": 2, "status": "bounced_back"}),
        (_, "3") => serde_json::json!({"id": 3, "status": "charged_back"}),
        (_, "4") => serde_json::json!({"id": 4, "status": "unknown"}),
        (_, "5") => serde_json::json!({"id": 5}),
        (_, "6") => serde_json::json!({"id": 999, "status": "outgoing_payment_sent"}),
        (_, "7") => serde_json::json!({"id": "7", "status": "processing"}),
        (_, "8") => serde_json::json!({"id": 8, "status": "funds_converted"}),
        _ => serde_json::json!({}),
    };
    Json(body)
}

async fn start_fixture() -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new().route("/v1/transfers/{id}", get(transfer_fixture));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Wise fixture");
    let address = listener.local_addr().expect("fixture address");
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve Wise fixture");
    });
    (format!("http://{address}"), server)
}

async fn status_at(base: &str, transfer_id: &str) -> Result<PayoutStatus, CapitalError> {
    legacy_payout_status(transfer_state_at("fixture_wise_token", base, transfer_id).await?)
}

#[tokio::test]
async fn live_transfer_status_is_bound_to_the_transfer_and_never_guessed() {
    let (base, server) = start_fixture().await;

    assert_eq!(
        status_at(&base, "1").await.expect("sent transfer"),
        PayoutStatus::OutgoingPaymentSent
    );
    assert_eq!(
        status_at(&base, "8").await.expect("converted transfer"),
        PayoutStatus::Processing
    );
    assert_eq!(
        transfer_state_at("fixture_wise_token", &base, "2")
            .await
            .expect("bounced transfer"),
        WiseTransferState::BouncedBack
    );

    // Returned or reversed payouts are failures, not transfers in flight.
    for failed in ["2", "3"] {
        assert!(matches!(
            status_at(&base, failed).await,
            Err(CapitalError::UnsupportedOperation(_))
        ));
    }
    // `unknown`, a missing state, another transfer's body or a non-numeric
    // ID in the response fail the response contract.
    for mismatched in ["4", "5", "6", "7"] {
        assert!(matches!(
            status_at(&base, mismatched).await,
            Err(CapitalError::Provider(_))
        ));
    }
    // Wise transfer IDs are positive decimals; anything else never leaves.
    for invalid in ["0", "+1", "01", "abc", "1/2"] {
        assert!(status_at(&base, invalid).await.is_err());
    }
    server.abort();
}

#[tokio::test]
async fn offline_mock_status_reports_only_mock_issued_transfers() {
    let provider = WiseProvider::new("mock_wise_token", "profile");
    let transfer_id = provider
        .send_payout("payee@example.com", 1_000, "EUR", "Invoice 1")
        .await
        .expect("mock transfer");
    assert_eq!(
        provider.get_payout_status(&transfer_id).await.unwrap(),
        PayoutStatus::OutgoingPaymentSent
    );
    assert!(matches!(
        provider.get_payout_status("123456").await,
        Err(CapitalError::UnsupportedOperation(_))
    ));
}
