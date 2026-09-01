#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{TimeZone, Utc};

#[test]
fn certificate_constructors_enforce_bounds_and_redact_secrets() {
    let certificate = FiscalCertificate::from_base64(
        format!("  {}\n", STANDARD.encode([1_u8, 2, 3])),
        "top-secret",
    )
    .expect("valid bounded certificate")
    .with_subject_cn("Rullst Test CA");

    assert_eq!(certificate.pkcs12_der().unwrap(), &[1, 2, 3]);
    assert_eq!(certificate.passphrase().unwrap(), "top-secret");
    certificate.validate_for_live_use().unwrap();
    assert_eq!(certificate.subject_cn.as_deref(), Some("Rullst Test CA"));

    let debug = format!("{certificate:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(debug.contains("Rullst Test CA"));
    assert!(!debug.contains("top-secret"));
    assert!(!debug.contains("1, 2, 3"));

    assert!(matches!(
        FiscalCertificate::from_bytes(&[], "secret"),
        Err(FiscalError::Certificate(message)) if message.contains("cannot be empty")
    ));
    assert!(matches!(
        FiscalCertificate::from_bytes(&vec![0_u8; MAX_PKCS12_BYTES + 1], "secret"),
        Err(FiscalError::Certificate(message)) if message.contains("exceeds")
    ));
    assert!(FiscalCertificate::from_base64(" ", "secret").is_err());
    assert!(FiscalCertificate::from_base64("%%%", "secret").is_err());

    let oversized = "A".repeat(MAX_PKCS12_BYTES.saturating_mul(4).div_ceil(3) + 5);
    assert!(matches!(
        FiscalCertificate::from_base64(oversized, "secret"),
        Err(FiscalError::Certificate(message)) if message.contains("exceeds")
    ));
}

#[test]
fn offline_certificate_fails_closed_for_both_secret_accessors() {
    let certificate = FiscalCertificate::offline_mock().with_subject_cn("offline fixture");
    assert!(certificate.pkcs12_der().is_err());
    assert!(certificate.passphrase().is_err());
    assert!(certificate.validate_for_live_use().is_err());
}

#[test]
fn fiscal_identity_and_authorization_helpers_preserve_provenance() {
    let emitter = FiscalEmitter {
        cnpj: "12.345.678/0001-90".to_string(),
        inscricao_municipal: "123".to_string(),
        legal_name: "Rullst".to_string(),
        trade_name: None,
        ibge_code: "3550308".to_string(),
        tax_regime: TaxRegime::default(),
    };
    assert_eq!(emitter.clean_cnpj(), "12345678000190");

    let person = FiscalCustomer {
        doc_number: "123.456.789-00".to_string(),
        name: "Person".to_string(),
        email: "person@example.com".to_string(),
        zip_code: None,
        address: None,
        ibge_code: None,
    };
    assert_eq!(person.clean_doc(), "12345678900");
    assert!(!person.is_company());

    let company = FiscalCustomer {
        doc_number: emitter.cnpj.clone(),
        ..person
    };
    assert!(company.is_company());

    let timestamp = Utc.timestamp_opt(0, 0).single().expect("Unix epoch");
    let offline = FiscalResponse {
        kind: FiscalResponseKind::OfflineMock,
        access_key: "MOCK-NOT-AUTHORIZED-1".to_string(),
        nfse_number: 0,
        protocol: "MOCK-ONLY-1".to_string(),
        authorized_xml: "<DPS/>".to_string(),
        authorized_at: timestamp,
        status: "MOCK_NOT_AUTHORIZED".to_string(),
        errors: Vec::new(),
    };
    assert!(!offline.is_officially_authorized());

    let official = FiscalResponse {
        kind: FiscalResponseKind::OfficialAuthorization,
        ..offline
    };
    assert!(official.is_officially_authorized());
}
