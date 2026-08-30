pub mod client;
pub mod contract;
pub mod dps;
pub mod dps_v101;
pub mod models;
#[cfg(feature = "nfse")]
pub mod schema;
pub mod signer;

pub use client::{NfseEnvironment, NfseNationalClient};
pub use contract::{
    MAX_DPS_XML_BYTES, MAX_SEFIN_RESPONSE_BYTES, NFSE_NAMESPACE, NFSE_PRODUCTION_SEFIN,
    NFSE_PRODUCTION_V1_01_20260209, NFSE_RESTRICTED_SEFIN, NFSE_RESTRICTED_V1_01_20260727,
    NfseArtifactManifest, NfseSefinContract,
};
pub use dps::build_dps_xml;
pub use dps_v101::{IssRetention, IssTaxation, NfseDpsV101, build_dps_xml_v1_01};
pub use models::{
    FiscalCertificate, FiscalCustomer, FiscalEmitter, FiscalError, FiscalResponse,
    FiscalResponseKind, NfseDps, TaxRegime,
};
#[cfg(feature = "nfse")]
pub use schema::NfseDpsSchemaValidator;
pub use signer::{compute_sha256_digest, sign_dps_xml};

/// High-level trait for issuing digital invoices.
#[async_trait::async_trait]
pub trait FiscalEngine: Send + Sync {
    /// Processes a fiscal document and returns its explicit authorization provenance.
    ///
    /// Implementations must never label offline or unverified output as an official authorization.
    async fn issue_nfse(
        &self,
        emitter: &FiscalEmitter,
        customer: &FiscalCustomer,
        dps: &NfseDps,
        cert: &FiscalCertificate,
    ) -> Result<FiscalResponse, FiscalError>;
}

/// Processes a digital invoice through an explicitly selected fiscal environment.
///
/// `Mock` produces a clearly marked offline fixture. Homologation and production fail closed until
/// the full official NFS-e integration is independently validated.
pub async fn issue_nfse_direct(
    emitter: &FiscalEmitter,
    customer: &FiscalCustomer,
    dps: &NfseDps,
    cert: &FiscalCertificate,
    environment: NfseEnvironment,
) -> Result<FiscalResponse, FiscalError> {
    let unsigned_xml = build_dps_xml(emitter, customer, dps);
    let client = NfseNationalClient::new(emitter.clone(), cert.clone(), environment);
    client.transmit_dps(&unsigned_xml).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_dps_xml_generation_and_escaping() {
        let emitter = FiscalEmitter {
            cnpj: "12.345.678/0001-90".to_string(),
            inscricao_municipal: "1234567".to_string(),
            legal_name: "Rullst SaaS & Software Ltda".to_string(),
            trade_name: Some("Rullst".to_string()),
            ibge_code: "3550308".to_string(), // São Paulo
            tax_regime: TaxRegime::SimplesNacional,
        };

        let customer = FiscalCustomer {
            doc_number: "123.456.789-00".to_string(),
            name: "João Silva & Filhos".to_string(),
            email: "joao@example.com".to_string(),
            zip_code: Some("01310-100".to_string()),
            address: Some("Av Paulista, 1000".to_string()),
            ibge_code: Some("3550308".to_string()),
        };

        let dps = NfseDps {
            id: "DPS355030800010000000000000000000000000000001".to_string(),
            series: "1".to_string(),
            number: 101,
            issued_at: Utc::now(),
            service_code: "1.03.01".to_string(),
            description: "Assinatura Mensal SaaS Rullst Pro <Plano Pro>".to_string(),
            amount: 99.00,
            iss_rate: 2.0,
            iss_retained: false,
            service_city_ibge: "3550308".to_string(),
        };

        let xml = build_dps_xml(&emitter, &customer, &dps);

        assert!(xml.contains("12345678000190"));
        assert!(xml.contains("12345678900"));
        assert!(xml.contains("Rullst SaaS &amp; Software Ltda"));
        assert!(xml.contains("João Silva &amp; Filhos"));
        assert!(xml.contains("&lt;Plano Pro&gt;"));
        assert!(xml.contains("<vServ>99.00</vServ>"));
    }

    #[test]
    #[cfg(feature = "nfse")]
    fn xml_digital_signature_rejects_an_invalid_pkcs12() {
        let cert = FiscalCertificate::from_bytes(b"not-a-real-pkcs12", "mock_pass").unwrap();

        let emitter = FiscalEmitter {
            cnpj: "12345678000190".to_string(),
            inscricao_municipal: "1234".to_string(),
            legal_name: "Empresa Teste".to_string(),
            trade_name: None,
            ibge_code: "3550308".to_string(),
            tax_regime: TaxRegime::SimplesNacional,
        };

        let customer = FiscalCustomer {
            doc_number: "11122233344".to_string(),
            name: "Cliente Teste".to_string(),
            email: "cliente@teste.com".to_string(),
            zip_code: None,
            address: None,
            ibge_code: None,
        };

        let dps = NfseDps {
            id: "DPS355030800010000000000000000000000000000001".to_string(),
            series: "1".to_string(),
            number: 1,
            issued_at: Utc::now(),
            service_code: "1.03.01".to_string(),
            description: "Serviço SaaS".to_string(),
            amount: 50.00,
            iss_rate: 2.0,
            iss_retained: false,
            service_city_ibge: "3550308".to_string(),
        };

        let unsigned_xml = build_dps_xml(&emitter, &customer, &dps);
        let signable_xml = concat!(
            "<DPS xmlns=\"http://www.sped.fazenda.gov.br/nfse\" versao=\"1.01\">",
            "<infDPS Id=\"DPS355030821122233300018100001000000000000101\">",
            "<tpAmb>2</tpAmb></infDPS></DPS>"
        );
        let result = sign_dps_xml(signable_xml, &cert);

        assert!(matches!(result, Err(FiscalError::Certificate(_))));
        assert!(!unsigned_xml.contains("<Signature"));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_issue_nfse_direct_mock() {
        let emitter = FiscalEmitter {
            cnpj: "12345678000190".to_string(),
            inscricao_municipal: "1234".to_string(),
            legal_name: "Empresa SaaS".to_string(),
            trade_name: None,
            ibge_code: "3550308".to_string(),
            tax_regime: TaxRegime::SimplesNacional,
        };

        let customer = FiscalCustomer {
            doc_number: "11122233344".to_string(),
            name: "Cliente".to_string(),
            email: "cliente@saas.com".to_string(),
            zip_code: None,
            address: None,
            ibge_code: None,
        };

        let dps = NfseDps {
            id: "DPS355030800010000000000000000000000000000001".to_string(),
            series: "1".to_string(),
            number: 1,
            issued_at: Utc::now(),
            service_code: "1.03.01".to_string(),
            description: "Plano Anual".to_string(),
            amount: 1200.00,
            iss_rate: 2.0,
            iss_retained: false,
            service_city_ibge: "3550308".to_string(),
        };

        let cert = FiscalCertificate::offline_mock();

        let response = issue_nfse_direct(&emitter, &customer, &dps, &cert, NfseEnvironment::Mock)
            .await
            .unwrap();

        assert_eq!(response.kind, FiscalResponseKind::OfflineMock);
        assert_eq!(response.status, "MOCK_NOT_AUTHORIZED");
        assert!(response.protocol.starts_with("MOCK-ONLY-"));
        assert_eq!(response.nfse_number, 0);
        assert!(!response.is_officially_authorized());
    }
}
