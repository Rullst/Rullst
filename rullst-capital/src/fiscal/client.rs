use chrono::{DateTime, Utc};
use ring::digest::{SHA256, digest};

use crate::fiscal::models::{
    FiscalCertificate, FiscalEmitter, FiscalError, FiscalResponse, FiscalResponseKind,
};

/// NFS-e execution mode.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfseEnvironment {
    /// Deterministic offline fixture. No document is transmitted or authorized.
    Mock,
    /// Homologation is blocked until the integration passes official end-to-end validation.
    Homologation,
    /// Production is blocked until the integration passes official end-to-end validation.
    Production,
}

impl NfseEnvironment {
    /// Returns a deliberately non-network endpoint marker.
    ///
    /// Rullst does not expose unverified NFS-e URLs. Transmission through the client remains
    /// fail-closed until the full official contract and mTLS setup are implemented.
    pub fn endpoint(&self) -> &'static str {
        match self {
            Self::Mock => "mock://offline-nfse",
            Self::Homologation | Self::Production => "unsupported://nfse-national",
        }
    }
}

/// Fail-closed National NFS-e client.
///
/// Only the explicit [`NfseEnvironment::Mock`] mode is currently executable. Homologation and
/// production return [`FiscalError::Unsupported`] without performing network I/O.
#[derive(Debug, Clone)]
pub struct NfseNationalClient {
    pub emitter: FiscalEmitter,
    pub certificate: FiscalCertificate,
    pub environment: NfseEnvironment,
}

impl NfseNationalClient {
    /// Creates a client with an explicit execution environment.
    pub fn new(
        emitter: FiscalEmitter,
        certificate: FiscalCertificate,
        environment: NfseEnvironment,
    ) -> Self {
        Self {
            emitter,
            certificate,
            environment,
        }
    }

    /// Processes a DPS according to the explicitly selected environment.
    pub async fn transmit_dps(&self, dps_xml: &str) -> Result<FiscalResponse, FiscalError> {
        match self.environment {
            NfseEnvironment::Mock => self.mock_response(dps_xml),
            NfseEnvironment::Homologation => Err(FiscalError::Unsupported(
                "NFS-e homologation is disabled until XMLDSig, PKCS#12, mTLS, XSD validation and official response parsing are validated end to end"
                    .to_string(),
            )),
            NfseEnvironment::Production => Err(FiscalError::Unsupported(
                "NFS-e production is disabled until XMLDSig, PKCS#12, mTLS, XSD validation and official homologation are complete"
                    .to_string(),
            )),
        }
    }

    fn mock_response(&self, dps_xml: &str) -> Result<FiscalResponse, FiscalError> {
        let payload_hash = hex::encode(digest(&SHA256, dps_xml.as_bytes()).as_ref());
        let short_hash = payload_hash.get(..24).ok_or_else(|| {
            FiscalError::General("Failed to construct deterministic mock identifier".to_string())
        })?;
        let authorized_at = DateTime::<Utc>::from_timestamp(0, 0).ok_or_else(|| {
            FiscalError::General("Failed to construct deterministic mock timestamp".to_string())
        })?;

        Ok(FiscalResponse {
            kind: FiscalResponseKind::OfflineMock,
            access_key: format!("MOCK-NOT-AUTHORIZED-{short_hash}"),
            nfse_number: 0,
            protocol: format!("MOCK-ONLY-{short_hash}"),
            authorized_xml: dps_xml.to_string(),
            authorized_at,
            status: "MOCK_NOT_AUTHORIZED".to_string(),
            errors: vec![
                "Offline mock fixture: no XMLDSig was produced and no tax authority authorized this document"
                    .to_string(),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fiscal::models::TaxRegime;

    fn emitter() -> FiscalEmitter {
        FiscalEmitter {
            cnpj: "12.345.678/0001-90".to_string(),
            inscricao_municipal: "12345".to_string(),
            legal_name: "Empresa Teste LTDA".to_string(),
            trade_name: Some("Teste".to_string()),
            ibge_code: "3550308".to_string(),
            tax_regime: TaxRegime::SimplesNacional,
        }
    }

    #[tokio::test]
    async fn explicit_mock_is_distinguishable_and_deterministic() {
        let cert = FiscalCertificate::from_base64("", "");
        let client = NfseNationalClient::new(emitter(), cert, NfseEnvironment::Mock);
        let dps_xml = "<DPS><infDPS>test</infDPS></DPS>";

        let first = client.transmit_dps(dps_xml).await.unwrap();
        let second = client.transmit_dps(dps_xml).await.unwrap();

        assert_eq!(first.kind, FiscalResponseKind::OfflineMock);
        assert!(!first.is_officially_authorized());
        assert_eq!(first.status, "MOCK_NOT_AUTHORIZED");
        assert_eq!(first.nfse_number, 0);
        assert!(first.access_key.starts_with("MOCK-NOT-AUTHORIZED-"));
        assert_eq!(first.protocol, second.protocol);
        assert_eq!(first.authorized_at, second.authorized_at);
    }

    #[tokio::test]
    // TM-PAY-06: non-mock fiscal transmission must never simulate authorization.
    async fn real_environments_fail_closed_even_with_mock_or_empty_certificate() {
        for environment in [NfseEnvironment::Homologation, NfseEnvironment::Production] {
            for certificate in [
                FiscalCertificate::from_base64("", "mock"),
                FiscalCertificate::from_base64("TU9DSw==", "mock"),
            ] {
                let client = NfseNationalClient::new(emitter(), certificate, environment);
                assert!(matches!(
                    client.transmit_dps("<DPS/>").await,
                    Err(FiscalError::Unsupported(_))
                ));
            }
        }
    }

    #[test]
    fn endpoints_are_non_network_markers() {
        assert_eq!(NfseEnvironment::Mock.endpoint(), "mock://offline-nfse");
        assert!(
            NfseEnvironment::Homologation
                .endpoint()
                .starts_with("unsupported://")
        );
        assert!(
            NfseEnvironment::Production
                .endpoint()
                .starts_with("unsupported://")
        );
    }
}
