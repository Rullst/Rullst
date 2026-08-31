#[cfg(feature = "oauth")]
#[test]
fn oauth_feature_exposes_connect_crate() {
    let _ = core::any::TypeId::of::<rullst::connect::ConnectError>();
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
