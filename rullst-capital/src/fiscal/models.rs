use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Tax regime of the emitting company.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TaxRegime {
    /// Simples Nacional (Microempresa / EPP)
    #[default]
    SimplesNacional = 1,
    /// Simples Nacional com excesso de sublimite
    SimplesNacionalExcesso = 2,
    /// Regime Normal (Lucro Presumido / Lucro Real)
    RegimeNormal = 3,
}

/// Strongly-typed error domain for Rullst Fiscal & NFS-e operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FiscalError {
    /// XML construction or digital signing failure.
    #[error("XML digital signing error: {0}")]
    XmlSigning(String),

    /// A1 Digital Certificate (.pfx/.p12) error or passphrase failure.
    #[error("Certificate error: {0}")]
    Certificate(String),

    /// Network connection to Receita Federal failed.
    #[error("Receita Federal network error: {0}")]
    Network(String),

    /// Receita Federal returned an API error.
    #[error("Receita Federal API error (HTTP {status}): {body}")]
    Api { status: u16, body: String },

    /// Failed to parse the response from the tax authority.
    #[error("Failed to parse fiscal response: {0}")]
    ResponseParse(String),

    /// A fiscal capability is intentionally unavailable until independently validated.
    #[error("Unsupported fiscal operation: {0}")]
    Unsupported(String),

    /// General fiscal exception.
    #[error("Fiscal error: {0}")]
    General(String),
}

/// A1 Digital Certificate (.pfx / .p12) container for digital signatures and mTLS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiscalCertificate {
    /// Base64-encoded PKCS#12 (.pfx / .p12) certificate file bytes.
    pub raw_pfx_base64: String,
    /// Secret passphrase decrypting the private key and public certificate.
    pub passphrase: String,
    /// Optional certificate issuer or subject CN description.
    pub subject_cn: Option<String>,
}

impl FiscalCertificate {
    /// Creates a new certificate container from raw .pfx bytes.
    pub fn from_bytes(pfx_bytes: &[u8], passphrase: impl Into<String>) -> Self {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;
        Self {
            raw_pfx_base64: STANDARD.encode(pfx_bytes),
            passphrase: passphrase.into(),
            subject_cn: None,
        }
    }

    /// Creates a new certificate container from a base64 string.
    pub fn from_base64(pfx_base64: impl Into<String>, passphrase: impl Into<String>) -> Self {
        Self {
            raw_pfx_base64: pfx_base64.into().trim().to_string(),
            passphrase: passphrase.into(),
            subject_cn: None,
        }
    }

    /// Decodes the underlying raw .pfx bytes.
    pub fn raw_bytes(&self) -> Result<Vec<u8>, FiscalError> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;
        if self.raw_pfx_base64.trim().is_empty() {
            return Err(FiscalError::Certificate(
                "PKCS#12 certificate cannot be empty".to_string(),
            ));
        }
        STANDARD.decode(&self.raw_pfx_base64).map_err(|e| {
            FiscalError::Certificate(format!("Failed to decode certificate base64: {}", e))
        })
    }
}

/// Provenance of a fiscal response.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FiscalResponseKind {
    /// Deterministic offline fixture. It is not a tax authorization.
    OfflineMock,
    /// Authorization received from an officially validated tax-authority integration.
    OfficialAuthorization,
}

/// The service provider / SaaS emitter company.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiscalEmitter {
    /// 14-digit CNPJ (cleaned without punctuation).
    pub cnpj: String,
    /// Inscrição Municipal (IM) with the local town hall.
    pub inscricao_municipal: String,
    /// Legal company name (Razão Social).
    pub legal_name: String,
    /// Optional trade name (Nome Fantasia).
    pub trade_name: Option<String>,
    /// IBGE 7-digit municipality code (e.g. "3550308" for São Paulo).
    pub ibge_code: String,
    /// Tax regime (Simples Nacional vs Normal).
    pub tax_regime: TaxRegime,
}

impl FiscalEmitter {
    /// Sanitizes the CNPJ string, keeping only digits.
    pub fn clean_cnpj(&self) -> String {
        self.cnpj.chars().filter(|c| c.is_ascii_digit()).collect()
    }
}

/// The customer / service taker (Tomador do Serviço).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiscalCustomer {
    /// CPF (11 digits) or CNPJ (14 digits).
    pub doc_number: String,
    /// Customer name or company legal name.
    pub name: String,
    /// Contact email for receiving the authorized NFS-e XML and PDF.
    pub email: String,
    /// Optional 8-digit postal code (CEP).
    pub zip_code: Option<String>,
    /// Optional street address and number.
    pub address: Option<String>,
    /// Optional city IBGE code.
    pub ibge_code: Option<String>,
}

impl FiscalCustomer {
    /// Sanitizes the document number, keeping only digits.
    pub fn clean_doc(&self) -> String {
        self.doc_number
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect()
    }

    /// Returns true if the document is a CNPJ (14 digits), false if CPF.
    pub fn is_company(&self) -> bool {
        self.clean_doc().len() > 11
    }
}

/// Declaração de Prestação de Serviços (DPS) - Standardized NFS-e Payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NfseDps {
    /// Unique DPS ID identifier (e.g. "DPS355030800010000000000000000000000000000001").
    pub id: String,
    /// Series identifier (usually "1" or "900" for national standard).
    pub series: String,
    /// Sequential DPS number.
    pub number: u64,
    /// Date and time of service provision.
    pub issued_at: DateTime<Utc>,
    /// National standard service classification code (e.g. "1.03.01" for SaaS & Data Processing).
    pub service_code: String,
    /// Clear textual description of the provided services.
    pub description: String,
    /// Total gross service amount in BRL.
    pub amount: f64,
    /// ISS tax rate percentage (e.g. 2.0 to 5.0 for 2% to 5%).
    pub iss_rate: f64,
    /// Whether ISS tax is retained at source by the customer.
    pub iss_retained: bool,
    /// Municipality IBGE code where the service was provided.
    pub service_city_ibge: String,
}

/// Result returned by the fiscal engine, with explicit authorization provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiscalResponse {
    /// Distinguishes an offline fixture from an official authorization.
    pub kind: FiscalResponseKind,
    /// Official access key, or an unmistakable `MOCK-NOT-AUTHORIZED-*` marker offline.
    pub access_key: String,
    /// Official NFS-e number, or zero for an offline fixture.
    pub nfse_number: u64,
    /// Official protocol, or an unmistakable `MOCK-ONLY-*` marker offline.
    pub protocol: String,
    /// Authorized XML for an official result, or the submitted DPS fixture offline.
    pub authorized_xml: String,
    /// Authorization timestamp, or a deterministic epoch timestamp offline.
    pub authorized_at: DateTime<Utc>,
    /// Status description. Offline fixtures always use `MOCK_NOT_AUTHORIZED`.
    pub status: String,
    /// List of validation warnings or errors if rejected.
    pub errors: Vec<String>,
}

impl FiscalResponse {
    /// Returns true only for a response produced by an officially validated integration.
    pub fn is_officially_authorized(&self) -> bool {
        self.kind == FiscalResponseKind::OfficialAuthorization
    }
}
