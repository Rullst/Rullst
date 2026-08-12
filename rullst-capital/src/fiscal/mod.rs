pub mod client;
pub mod dps;
pub mod models;
pub mod signer;

pub use client::{NfseEnvironment, NfseNationalClient};
pub use dps::build_dps_xml;
pub use models::{
    FiscalCertificate, FiscalCustomer, FiscalEmitter, FiscalResponse, NfseDps, TaxRegime,
};
pub use signer::{compute_sha256_digest, sign_dps_xml};

/// High-level trait for issuing digital invoices.
#[async_trait::async_trait]
pub trait FiscalEngine: Send + Sync {
    /// Issues an authorized digital invoice for a completed payment.
    async fn issue_nfse(
        &self,
        emitter: &FiscalEmitter,
        customer: &FiscalCustomer,
        dps: &NfseDps,
        cert: &FiscalCertificate,
    ) -> Result<FiscalResponse, String>;
}

/// Issues a digital invoice (NFS-e Nacional) using the direct zero-cost Receita Federal engine.
pub async fn issue_nfse_direct(
    emitter: &FiscalEmitter,
    customer: &FiscalCustomer,
    dps: &NfseDps,
    cert: &FiscalCertificate,
    environment: NfseEnvironment,
) -> Result<FiscalResponse, String> {
    let unsigned_xml = build_dps_xml(emitter, customer, dps);
    let signed_xml = sign_dps_xml(&unsigned_xml, cert)?;

    let client = NfseNationalClient::new(emitter.clone(), cert.clone(), environment);
    client.transmit_dps(&signed_xml).await
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
    fn test_xml_digital_signature_structure() {
        let cert = FiscalCertificate::from_base64("MIIKggIBAzCCCl8GCSqGSIb3DQEHA", "mock_pass");

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
        let signed_xml = sign_dps_xml(&unsigned_xml, &cert).unwrap();

        assert!(signed_xml.contains("<Signature xmlns=\"http://www.w3.org/2000/09/xmldsig#\">"));
        assert!(signed_xml.contains("<SignedInfo"));
        assert!(signed_xml.contains("<SignatureValue>"));
        assert!(
            signed_xml.contains("<X509Certificate>MIIKggIBAzCCCl8GCSqGSIb3DQEHA</X509Certificate>")
        );
    }

    #[tokio::test]
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

        let cert = FiscalCertificate::from_base64("MIIKggIBAzCCCl8GCSqGSIb3DQEHA", "mock");

        let response = issue_nfse_direct(
            &emitter,
            &customer,
            &dps,
            &cert,
            NfseEnvironment::Homologation,
        )
        .await
        .unwrap();

        assert_eq!(response.status, "Autorizada");
        assert_eq!(response.protocol, "PROT-MOCK-999888777");
        assert_eq!(response.nfse_number, 1);
    }
}
