//! Bounded offline codec for the SEFIN National synchronous issuance contract.

use std::io::{Read, Write};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, FixedOffset};
use flate2::{Compression, GzBuilder, read::GzDecoder};
use serde::{Deserialize, Serialize};

use crate::fiscal::{
    MAX_DPS_XML_BYTES, MAX_SEFIN_RESPONSE_BYTES, NfseEnvironment, models::FiscalError,
};

mod validation;

use validation::{validate_authorized_nfse, validate_signed_dps_shape};

const MAX_COMPRESSED_DPS_BYTES: usize = MAX_DPS_XML_BYTES + 64 * 1024;
const MAX_PROCESSING_MESSAGES: usize = 100;
const MAX_APP_VERSION_BYTES: usize = 128;
const MAX_DPS_ID_BYTES: usize = 128;
const MAX_MESSAGE_FIELD_BYTES: usize = 2 * 1024;
const ACCESS_KEY_BYTES: usize = 50;

/// Environment identifier defined by the SEFIN JSON contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NfseApiEnvironment {
    Production,
    Homologation,
}

impl NfseApiEnvironment {
    fn from_execution(environment: NfseEnvironment) -> Result<Self, FiscalError> {
        match environment {
            NfseEnvironment::Production => Ok(Self::Production),
            NfseEnvironment::Homologation => Ok(Self::Homologation),
            NfseEnvironment::Mock => Err(FiscalError::InvalidInput {
                field: "nfse.environment",
                reason: "offline mock is not a SEFIN API environment".to_string(),
            }),
        }
    }

    fn from_code(code: u8) -> Result<Self, FiscalError> {
        match code {
            1 => Ok(Self::Production),
            2 => Ok(Self::Homologation),
            _ => Err(response_error("tipoAmbiente must be 1 or 2")),
        }
    }
}

/// One bounded SEFIN processing diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NfseProcessingMessage {
    pub message: Option<String>,
    pub code: Option<String>,
    pub description: Option<String>,
    pub complement: Option<String>,
}

/// Exact synchronous issuance request envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NfseIssueRequest {
    dps_id: String,
    environment: NfseApiEnvironment,
    dps_xml_gzip_base64: String,
}

impl NfseIssueRequest {
    /// Validates the signed DPS shape and builds deterministic GZip/Base64 JSON material.
    ///
    /// This checks envelope structure, not certificate trust or emitter ownership. Callers
    /// should pass the output of [`crate::fiscal::sign_dps_xml`].
    pub fn try_from_signed_dps(signed_xml: &str) -> Result<Self, FiscalError> {
        let (dps_id, environment) = validate_signed_dps_shape(signed_xml)?;
        let compressed = gzip(signed_xml.as_bytes())?;
        if compressed.len() > MAX_COMPRESSED_DPS_BYTES {
            return Err(FiscalError::InvalidInput {
                field: "dps.xml",
                reason: "compressed DPS exceeds the bounded request limit".to_string(),
            });
        }
        Ok(Self {
            dps_id,
            environment,
            dps_xml_gzip_base64: STANDARD.encode(compressed),
        })
    }

    pub fn dps_id(&self) -> &str {
        &self.dps_id
    }

    /// Returns the environment encoded inside the signed `infDPS/tpAmb` field.
    pub const fn environment(&self) -> NfseApiEnvironment {
        self.environment
    }

    pub fn dps_xml_gzip_base64(&self) -> &str {
        &self.dps_xml_gzip_base64
    }

    /// Serializes exactly the documented `dpsXmlGZipB64` request object.
    pub fn to_json(&self) -> Result<Vec<u8>, FiscalError> {
        serde_json::to_vec(&IssueRequestWire {
            dps_xml_gzip_base64: &self.dps_xml_gzip_base64,
        })
        .map_err(|_| FiscalError::General("cannot serialize the NFS-e request".to_string()))
    }

    /// Parses one bounded response and binds a successful/rejected result to this DPS.
    pub fn parse_response(
        &self,
        http_status: u16,
        environment: NfseEnvironment,
        body: &[u8],
    ) -> Result<NfseIssueResponse, FiscalError> {
        if NfseApiEnvironment::from_execution(environment)? != self.environment {
            return Err(response_error(
                "execution environment does not match the signed DPS tpAmb",
            ));
        }
        parse_issue_response(http_status, environment, &self.dps_id, body)
    }
}

/// Successfully decoded SEFIN authorization material.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NfseIssueAuthorization {
    pub environment: NfseApiEnvironment,
    pub application_version: String,
    pub processed_at: DateTime<FixedOffset>,
    pub dps_id: String,
    pub access_key: String,
    pub authorized_xml: String,
    pub warnings: Vec<NfseProcessingMessage>,
}

/// Structured SEFIN rejection; it is not an authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NfseIssueRejection {
    pub http_status: u16,
    pub environment: NfseApiEnvironment,
    pub application_version: String,
    pub processed_at: DateTime<FixedOffset>,
    pub dps_id: Option<String>,
    pub errors: Vec<NfseProcessingMessage>,
}

/// Strictly distinguished SEFIN issuance result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NfseIssueResponse {
    Authorized(NfseIssueAuthorization),
    Rejected(NfseIssueRejection),
}

#[derive(Serialize)]
struct IssueRequestWire<'a> {
    #[serde(rename = "dpsXmlGZipB64")]
    dps_xml_gzip_base64: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SuccessWire {
    #[serde(rename = "tipoAmbiente")]
    environment: u8,
    #[serde(rename = "versaoAplicativo")]
    application_version: String,
    #[serde(rename = "dataHoraProcessamento")]
    processed_at: String,
    #[serde(rename = "idDps")]
    dps_id: String,
    #[serde(rename = "chaveAcesso")]
    access_key: String,
    #[serde(rename = "nfseXmlGZipB64")]
    nfse_xml_gzip_base64: String,
    #[serde(default)]
    alertas: Option<Vec<ProcessingMessageWire>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RejectionWire {
    #[serde(rename = "tipoAmbiente")]
    environment: u8,
    #[serde(rename = "versaoAplicativo")]
    application_version: String,
    #[serde(rename = "dataHoraProcessamento")]
    processed_at: String,
    #[serde(rename = "idDPS", alias = "idDps")]
    dps_id: Option<String>,
    #[serde(rename = "erros")]
    errors: Vec<ProcessingMessageWire>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessingMessageWire {
    #[serde(default, rename = "mensagem", alias = "Mensagem")]
    message: Option<String>,
    #[serde(default, rename = "codigo", alias = "Codigo")]
    code: Option<String>,
    #[serde(default, rename = "descricao", alias = "Descricao")]
    description: Option<String>,
    #[serde(default, rename = "complemento", alias = "Complemento")]
    complement: Option<String>,
}

fn parse_issue_response(
    http_status: u16,
    execution_environment: NfseEnvironment,
    expected_dps_id: &str,
    body: &[u8],
) -> Result<NfseIssueResponse, FiscalError> {
    if body.is_empty() || body.len() > MAX_SEFIN_RESPONSE_BYTES {
        return Err(response_error("response body is empty or exceeds four MiB"));
    }
    let expected_environment = NfseApiEnvironment::from_execution(execution_environment)?;
    match http_status {
        201 => parse_authorization(body, expected_environment, expected_dps_id)
            .map(NfseIssueResponse::Authorized),
        400 | 403 | 500 => {
            parse_rejection(http_status, body, expected_environment, expected_dps_id)
                .map(NfseIssueResponse::Rejected)
        }
        _ => Err(response_error("unexpected SEFIN issuance HTTP status")),
    }
}

fn parse_authorization(
    body: &[u8],
    expected_environment: NfseApiEnvironment,
    expected_dps_id: &str,
) -> Result<NfseIssueAuthorization, FiscalError> {
    let wire: SuccessWire = parse_json(body)?;
    let environment = validate_environment(wire.environment, expected_environment)?;
    validate_text(
        &wire.application_version,
        "versaoAplicativo",
        MAX_APP_VERSION_BYTES,
    )?;
    validate_exact_dps_id(&wire.dps_id, expected_dps_id)?;
    validate_access_key(&wire.access_key)?;
    let processed_at = parse_timestamp(&wire.processed_at)?;
    let compressed = decode_base64(&wire.nfse_xml_gzip_base64)?;
    let authorized_xml = gunzip_bounded(&compressed, MAX_SEFIN_RESPONSE_BYTES)?;
    validate_authorized_nfse(&authorized_xml, &wire.access_key)?;
    let warnings = validate_messages(wire.alertas.unwrap_or_default(), false)?;
    Ok(NfseIssueAuthorization {
        environment,
        application_version: wire.application_version,
        processed_at,
        dps_id: wire.dps_id,
        access_key: wire.access_key,
        authorized_xml,
        warnings,
    })
}

fn parse_rejection(
    http_status: u16,
    body: &[u8],
    expected_environment: NfseApiEnvironment,
    expected_dps_id: &str,
) -> Result<NfseIssueRejection, FiscalError> {
    let wire: RejectionWire = parse_json(body)?;
    let environment = validate_environment(wire.environment, expected_environment)?;
    validate_text(
        &wire.application_version,
        "versaoAplicativo",
        MAX_APP_VERSION_BYTES,
    )?;
    if let Some(dps_id) = wire.dps_id.as_deref() {
        validate_exact_dps_id(dps_id, expected_dps_id)?;
    }
    let processed_at = parse_timestamp(&wire.processed_at)?;
    let errors = validate_messages(wire.errors, true)?;
    Ok(NfseIssueRejection {
        http_status,
        environment,
        application_version: wire.application_version,
        processed_at,
        dps_id: wire.dps_id,
        errors,
    })
}

fn parse_json<T: for<'de> Deserialize<'de>>(body: &[u8]) -> Result<T, FiscalError> {
    serde_json::from_slice(body).map_err(|_| response_error("invalid or ambiguous JSON response"))
}

fn validate_environment(
    code: u8,
    expected: NfseApiEnvironment,
) -> Result<NfseApiEnvironment, FiscalError> {
    let actual = NfseApiEnvironment::from_code(code)?;
    if actual != expected {
        return Err(response_error(
            "response environment does not match the request",
        ));
    }
    Ok(actual)
}

fn parse_timestamp(value: &str) -> Result<DateTime<FixedOffset>, FiscalError> {
    validate_text(value, "dataHoraProcessamento", 64)?;
    DateTime::parse_from_rfc3339(value)
        .map_err(|_| response_error("dataHoraProcessamento is not RFC 3339"))
}

fn validate_exact_dps_id(value: &str, expected: &str) -> Result<(), FiscalError> {
    validate_text(value, "idDps", MAX_DPS_ID_BYTES)?;
    if value != expected {
        return Err(response_error(
            "response idDps does not match the submitted DPS",
        ));
    }
    Ok(())
}

fn validate_access_key(value: &str) -> Result<(), FiscalError> {
    if value.len() != ACCESS_KEY_BYTES || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(response_error("chaveAcesso must contain exactly 50 digits"));
    }
    Ok(())
}

fn validate_messages(
    messages: Vec<ProcessingMessageWire>,
    required: bool,
) -> Result<Vec<NfseProcessingMessage>, FiscalError> {
    if messages.len() > MAX_PROCESSING_MESSAGES || (required && messages.is_empty()) {
        return Err(response_error("processing message cardinality is invalid"));
    }
    messages.into_iter().map(validate_message).collect()
}

fn validate_message(wire: ProcessingMessageWire) -> Result<NfseProcessingMessage, FiscalError> {
    let fields = [
        (wire.message.as_deref(), "mensagem"),
        (wire.code.as_deref(), "codigo"),
        (wire.description.as_deref(), "descricao"),
        (wire.complement.as_deref(), "complemento"),
    ];
    if fields.iter().all(|(value, _)| value.is_none()) {
        return Err(response_error("processing message has no fields"));
    }
    for (value, name) in fields {
        if let Some(value) = value {
            validate_text(value, name, MAX_MESSAGE_FIELD_BYTES)?;
        }
    }
    Ok(NfseProcessingMessage {
        message: wire.message,
        code: wire.code,
        description: wire.description,
        complement: wire.complement,
    })
}

fn validate_text(value: &str, field: &'static str, maximum: usize) -> Result<(), FiscalError> {
    if value.is_empty()
        || value.len() > maximum
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(response_error(field));
    }
    Ok(())
}

fn gzip(bytes: &[u8]) -> Result<Vec<u8>, FiscalError> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::default());
    encoder
        .write_all(bytes)
        .map_err(|_| FiscalError::General("cannot compress the signed DPS".to_string()))?;
    encoder
        .finish()
        .map_err(|_| FiscalError::General("cannot finalize the signed DPS GZip".to_string()))
}

fn decode_base64(value: &str) -> Result<Vec<u8>, FiscalError> {
    if value.is_empty() || value.len() > MAX_SEFIN_RESPONSE_BYTES {
        return Err(response_error(
            "compressed XML base64 is empty or oversized",
        ));
    }
    STANDARD
        .decode(value)
        .map_err(|_| response_error("compressed XML is not valid base64"))
}

fn gunzip_bounded(bytes: &[u8], maximum: usize) -> Result<String, FiscalError> {
    if !bytes.starts_with(&[0x1f, 0x8b]) {
        return Err(response_error("compressed XML is not GZip"));
    }
    let mut decoded = Vec::new();
    GzDecoder::new(bytes)
        .take((maximum + 1) as u64)
        .read_to_end(&mut decoded)
        .map_err(|_| response_error("compressed XML cannot be decoded"))?;
    if decoded.len() > maximum {
        return Err(response_error("decompressed XML exceeds four MiB"));
    }
    String::from_utf8(decoded).map_err(|_| response_error("decompressed XML is not UTF-8"))
}

fn invalid_dps(reason: impl Into<String>) -> FiscalError {
    FiscalError::InvalidInput {
        field: "dps.xml",
        reason: reason.into(),
    }
}

fn response_error(message: impl Into<String>) -> FiscalError {
    FiscalError::ResponseParse(message.into())
}

#[cfg(test)]
mod tests;
