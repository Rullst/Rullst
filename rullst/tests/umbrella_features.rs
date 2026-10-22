#[cfg(feature = "oauth")]
#[test]
fn oauth_feature_exposes_connect_crate() {
    let _ = core::any::TypeId::of::<rullst::connect::ConnectError>();
}

/// The facade's own dev-dependency on `rullst-connect` enables Axum for every
/// test build, so the manifest is the only place where a facade-only `oauth`
/// application can lose the callback extractor.
#[test]
fn oauth_feature_enables_the_connect_axum_integration() {
    let manifest: toml::Table = toml::from_str(include_str!("../Cargo.toml")).unwrap();
    let oauth = manifest["features"]["oauth"].as_array().unwrap();
    let oauth = oauth
        .iter()
        .filter_map(toml::Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(oauth, ["dep:rullst-connect", "rullst-connect/rullst"]);
}

#[cfg(feature = "oauth")]
#[tokio::test]
async fn oauth_feature_exposes_the_axum_callback_extractor() {
    use axum::{Router, body::Body, http::Request, routing::get};
    use rullst::connect::extractors::AuthCallback;
    use tower::ServiceExt;

    async fn callback(callback: AuthCallback) -> String {
        format!(
            "{}:{}",
            callback.code.unwrap_or_default(),
            callback.state.unwrap_or_default()
        )
    }

    let router = Router::new().route("/auth/callback", get(callback));
    let request = Request::builder()
        .uri("/auth/callback?code=fixture-code&state=fixture-state")
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), 200);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    assert_eq!(&body[..], b"fixture-code:fixture-state");
}

#[cfg(feature = "security")]
#[test]
fn security_feature_exposes_extended_suite_without_hiding_core_security() {
    let _ = core::any::TypeId::of::<rullst::security::TenantContext>();
    let _ = core::any::TypeId::of::<rullst::security::runtime::AuditRecord>();
    let _ = core::any::TypeId::of::<rullst::security_runtime::VaultError>();
}

#[cfg(feature = "iot")]
#[test]
fn iot_feature_exposes_iot_crate() {
    let telemetry = rullst::iot::SensorTelemetry::new("sensor-1", "temperature", 24.0, 1);
    assert_eq!(telemetry.device_id, "sensor-1");
}

#[cfg(feature = "mailer")]
#[test]
fn mailer_feature_exposes_mail_with_smtp_enabled() {
    let _ = core::any::TypeId::of::<rullst::mail::SmtpDriver>();
}

#[cfg(feature = "capital-mail")]
#[test]
fn capital_mail_exposes_payment_bound_delivery_without_requiring_smtp() {
    let _ = core::any::TypeId::of::<rullst::mail::PaidInvoiceDelivery>();
    let _ = core::any::TypeId::of::<rullst::capital::PaidInvoice>();
}
