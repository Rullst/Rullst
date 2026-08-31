use chrono::{DateTime, Utc};
use ring::digest::{SHA256, digest};

use crate::fiscal::contract::{NFSE_PRODUCTION_SEFIN, NFSE_RESTRICTED_SEFIN, NfseSefinContract};
use crate::fiscal::models::{
    FiscalCertificate, FiscalEmitter, FiscalError, FiscalResponse, FiscalResponseKind,
};
#[cfg(feature = "nfse")]
use crate::fiscal::signer::build_mtls_identity;

#[cfg(feature = "nfse")]
const NFSE_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
#[cfg(feature = "nfse")]
const NFSE_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

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
    /// Returns the immutable official SEFIN contract, or `None` for the offline mock.
    ///
    /// Knowing an endpoint does not enable transmission. The client remains fail-closed until
    /// the signed request and response paths pass independent interoperability and official tests.
    pub fn sefin_contract(&self) -> Option<&'static NfseSefinContract> {
        match self {
            Self::Mock => None,
            Self::Homologation => Some(&NFSE_RESTRICTED_SEFIN),
            Self::Production => Some(&NFSE_PRODUCTION_SEFIN),
        }
    }

    /// Returns the offline marker or the pinned official API base URL.
    pub fn endpoint(&self) -> &'static str {
        match self {
            Self::Mock => "mock://offline-nfse",
            Self::Homologation => NFSE_RESTRICTED_SEFIN.base_url,
            Self::Production => NFSE_PRODUCTION_SEFIN.base_url,
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
            NfseEnvironment::Homologation => {
                self.certificate.validate_for_live_use()?;
                #[cfg(feature = "nfse")]
                let _client = build_live_http_client(&self.certificate)?;
                Err(FiscalError::Unsupported(
                    "NFS-e homologation transport is disabled until certificate trust, durable idempotency/audit, restricted-environment evidence and official homologation are validated end to end"
                        .to_string(),
                ))
            }
            NfseEnvironment::Production => {
                self.certificate.validate_for_live_use()?;
                #[cfg(feature = "nfse")]
                let _client = build_live_http_client(&self.certificate)?;
                Err(FiscalError::Unsupported(
                    "NFS-e production is disabled until restricted-environment homologation and response evidence are complete"
                        .to_string(),
                ))
            }
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

#[cfg(feature = "nfse")]
fn build_live_http_client(certificate: &FiscalCertificate) -> Result<reqwest::Client, FiscalError> {
    let identity = build_mtls_identity(certificate)?;
    reqwest::Client::builder()
        .identity(identity)
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(NFSE_CONNECT_TIMEOUT)
        .timeout(NFSE_REQUEST_TIMEOUT)
        .build()
        .map_err(|_| {
            FiscalError::Certificate("cannot construct the bounded NFS-e mTLS client".to_string())
        })
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
        let cert = FiscalCertificate::offline_mock();
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
            for certificate in [FiscalCertificate::offline_mock()] {
                let client = NfseNationalClient::new(emitter(), certificate, environment);
                assert!(client.transmit_dps("<DPS/>").await.is_err());
            }
        }
    }

    #[test]
    fn endpoints_are_pinned_to_the_official_contracts() {
        assert_eq!(NfseEnvironment::Mock.endpoint(), "mock://offline-nfse");
        assert!(
            NfseEnvironment::Homologation
                .endpoint()
                .starts_with("https://sefin.producaorestrita.nfse.gov.br/")
        );
        assert!(
            NfseEnvironment::Production
                .endpoint()
                .starts_with("https://sefin.nfse.gov.br/")
        );
        assert!(NfseEnvironment::Mock.sefin_contract().is_none());
    }
}
