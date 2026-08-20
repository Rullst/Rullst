// tests/integration_test.rs — Comprehensive fiscal, billing and webhook tests for Rullst Capital.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::Utc;
use rullst_capital::fiscal::{
    FiscalCustomer, FiscalEmitter, NfseDps, TaxRegime, build_dps_xml, compute_sha256_digest,
};

#[test]
fn test_fiscal_xml_builder_and_digest() {
    let emitter = FiscalEmitter {
        cnpj: "12.345.678/0001-90".to_string(),
        inscricao_municipal: "1234567".to_string(),
        legal_name: "Rullst SaaS & Software Ltda".to_string(),
        trade_name: Some("Rullst".to_string()),
        ibge_code: "3550308".to_string(),
        tax_regime: TaxRegime::SimplesNacional,
    };

    let customer = FiscalCustomer {
        doc_number: "123.456.789-00".to_string(),
        name: "João Silva & Cia".to_string(),
        email: "joao@example.com".to_string(),
        zip_code: Some("01310-100".to_string()),
        address: Some("Av. Paulista, 1000".to_string()),
        ibge_code: Some("3550308".to_string()),
    };

    let dps = NfseDps {
        id: "DPS355030800010000000000000000000000000000001".to_string(),
        series: "1".to_string(),
        number: 1001,
        issued_at: Utc::now(),
        service_code: "01.07.01".to_string(),
        description: "Software as a Service Subscription & Support".to_string(),
        amount: 99.00,
        iss_rate: 0.05,
        iss_retained: false,
        service_city_ibge: "3550308".to_string(),
    };

    let xml = build_dps_xml(&emitter, &customer, &dps);
    assert!(xml.contains("<DPS"));
    assert!(xml.contains("12345678000190"));
    assert!(xml.contains("99.00"));

    let digest = compute_sha256_digest(&xml);
    assert!(!digest.is_empty());
}
