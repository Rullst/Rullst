use super::{FiscalCommandStatus, FiscalJournalError, JournalEnvironment};
use crate::fiscal::{
    NfseApiEnvironment, NfseIssueRequest, NfseIssueResponse, NfseProcessingMessage,
};
use ring::digest;

pub(super) struct TerminalEvidence {
    pub(super) environment: JournalEnvironment,
    pub(super) status: FiscalCommandStatus,
    pub(super) result_digest: String,
    pub(super) http_status: Option<u16>,
    pub(super) processed_at_unix_ms: i64,
}

pub(super) fn response_evidence(
    request: &NfseIssueRequest,
    response: &NfseIssueResponse,
) -> Result<TerminalEvidence, FiscalJournalError> {
    let mut fields: Vec<Vec<u8>> = Vec::new();
    let (environment, status, http_status, processed_at_unix_ms) = match response {
        NfseIssueResponse::Authorized(value) => {
            if value.dps_id != request.dps_id() {
                return Err(FiscalJournalError::ResponseMismatch);
            }
            fields.push(b"authorized".to_vec());
            fields.push(value.application_version.as_bytes().to_vec());
            fields.push(value.dps_id.as_bytes().to_vec());
            fields.push(value.access_key.as_bytes().to_vec());
            fields.push(value.authorized_xml.as_bytes().to_vec());
            append_messages(&mut fields, &value.warnings);
            let processed_at = value.processed_at.timestamp_millis();
            if processed_at < 0 {
                return Err(FiscalJournalError::ResponseMismatch);
            }
            (
                api_environment(value.environment),
                FiscalCommandStatus::Authorized,
                None,
                processed_at,
            )
        }
        NfseIssueResponse::Rejected(value) => {
            if !matches!(value.http_status, 400 | 403 | 500)
                || value
                    .dps_id
                    .as_deref()
                    .is_some_and(|id| id != request.dps_id())
            {
                return Err(FiscalJournalError::ResponseMismatch);
            }
            fields.push(b"rejected".to_vec());
            fields.push(value.http_status.to_be_bytes().to_vec());
            fields.push(value.application_version.as_bytes().to_vec());
            fields.push(
                value
                    .dps_id
                    .as_deref()
                    .unwrap_or_default()
                    .as_bytes()
                    .to_vec(),
            );
            append_messages(&mut fields, &value.errors);
            let processed_at = value.processed_at.timestamp_millis();
            if processed_at < 0 {
                return Err(FiscalJournalError::ResponseMismatch);
            }
            (
                api_environment(value.environment),
                FiscalCommandStatus::Rejected,
                Some(value.http_status),
                processed_at,
            )
        }
    };
    fields.push(processed_at_unix_ms.to_be_bytes().to_vec());
    Ok(TerminalEvidence {
        environment,
        status,
        result_digest: digest_owned_fields(&fields),
        http_status,
        processed_at_unix_ms,
    })
}

pub(super) fn request_fingerprint(
    request: &NfseIssueRequest,
) -> Result<String, FiscalJournalError> {
    let bytes = request
        .to_json()
        .map_err(|_| FiscalJournalError::RequestEncoding)?;
    Ok(sha256_hex(&bytes))
}

fn append_messages(fields: &mut Vec<Vec<u8>>, messages: &[NfseProcessingMessage]) {
    fields.push((messages.len() as u64).to_be_bytes().to_vec());
    for message in messages {
        for value in [
            &message.message,
            &message.code,
            &message.description,
            &message.complement,
        ] {
            fields.push(value.as_deref().unwrap_or_default().as_bytes().to_vec());
        }
    }
}

fn api_environment(environment: NfseApiEnvironment) -> JournalEnvironment {
    match environment {
        NfseApiEnvironment::Homologation => JournalEnvironment::Homologation,
        NfseApiEnvironment::Production => JournalEnvironment::Production,
    }
}

fn digest_owned_fields(fields: &[Vec<u8>]) -> String {
    let mut context = digest::Context::new(&digest::SHA256);
    for field in fields {
        context.update(&(field.len() as u64).to_be_bytes());
        context.update(field);
    }
    hex::encode(context.finish().as_ref())
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(digest::digest(&digest::SHA256, bytes).as_ref())
}
