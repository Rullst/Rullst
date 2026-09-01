//! Native SES v2 transport backed by the official AWS SDK for Rust.

use crate::error::MailError;
use crate::message::Message;
use aws_sdk_sesv2::error::ProvideErrorMetadata;
use aws_sdk_sesv2::types::{
    Attachment as SesAttachment, AttachmentContentDisposition, Body, Content, Destination,
    EmailContent, Message as SesMessage, MessageHeader,
};
use aws_sdk_sesv2::{Client, Config};

const MAX_SES_V2_MESSAGE_BYTES: usize = 40 * 1024 * 1024;
const MESSAGE_ENVELOPE_ALLOWANCE: usize = 4 * 1024;

pub(super) struct NativeSesConfig {
    client: Client,
    endpoint_client: tokio::sync::OnceCell<Client>,
}

impl NativeSesConfig {
    pub(super) fn try_new(config: Config) -> Result<Self, MailError> {
        let config = config.to_builder().behavior_version_latest().build();
        let client =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Client::from_conf(config)))
                .map_err(|_| {
                    MailError::ConfigError("invalid native AWS SES SDK configuration".to_string())
                })?;
        Ok(Self {
            client,
            endpoint_client: tokio::sync::OnceCell::new(),
        })
    }

    pub(super) async fn send(
        &self,
        endpoint_override: Option<&str>,
        message: &Message,
    ) -> Result<(), MailError> {
        validate_message_limits(message)?;
        let client = if let Some(endpoint) = endpoint_override {
            self.endpoint_client
                .get_or_try_init(|| async { self.client_for_endpoint(endpoint) })
                .await?
        } else {
            &self.client
        };
        let destination = Destination::builder().to_addresses(&message.to).build();
        let content = build_content(message)?;
        let result = client
            .send_email()
            .from_email_address(message.from.as_deref().unwrap_or("noreply@rullst.dev"))
            .destination(destination)
            .content(content)
            .send()
            .await;

        match result {
            Ok(output) if output.message_id().is_some_and(|id| !id.trim().is_empty()) => Ok(()),
            Ok(_) => Err(MailError::from_provider_response(
                "aws_ses",
                502,
                "SES accepted the request without a message identifier",
                None,
            )),
            Err(error) => Err(map_sdk_error(error)),
        }
    }

    fn client_for_endpoint(&self, endpoint: &str) -> Result<Client, MailError> {
        let config = self
            .client
            .config()
            .to_builder()
            .behavior_version_latest()
            .endpoint_url(endpoint)
            .build();
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| Client::from_conf(config)))
            .map_err(|_| {
                MailError::ConfigError("invalid AWS SES endpoint configuration".to_string())
            })
    }
}

pub(super) fn config_with_provider<P>(region: &str, provider: P) -> Config
where
    P: aws_sdk_sesv2::config::ProvideCredentials + 'static,
{
    Config::builder()
        .behavior_version_latest()
        .region(aws_sdk_sesv2::config::Region::new(region.to_string()))
        .credentials_provider(provider)
        .build()
}

fn build_content(message: &Message) -> Result<EmailContent, MailError> {
    let subject = Content::builder()
        .data(&message.subject)
        .charset("UTF-8")
        .build()
        .map_err(|_| MailError::ValidationError("invalid AWS SES subject".to_string()))?;
    let body = Body::builder()
        .set_html(message.body_html.as_ref().map(utf8_content).transpose()?)
        .set_text(message.body_text.as_ref().map(utf8_content).transpose()?)
        .build();
    let mut simple = SesMessage::builder().subject(subject).body(body);

    if let Some(unsubscribe) = message.list_unsubscribe_header() {
        simple = simple.headers(header("List-Unsubscribe", unsubscribe)?);
        if message.unsubscribe_url.is_some() {
            simple = simple.headers(header(
                "List-Unsubscribe-Post",
                "List-Unsubscribe=One-Click".to_string(),
            )?);
        }
    }
    for attachment in &message.attachments {
        let disposition = if attachment.is_inline() {
            AttachmentContentDisposition::Inline
        } else {
            AttachmentContentDisposition::Attachment
        };
        let built = SesAttachment::builder()
            .raw_content(aws_sdk_sesv2::primitives::Blob::new(
                attachment.content.clone(),
            ))
            .file_name(&attachment.filename)
            .content_type(&attachment.mime_type)
            .content_disposition(disposition)
            .set_content_id(attachment.cid.clone())
            .build()
            .map_err(|_| MailError::ValidationError("invalid AWS SES attachment".to_string()))?;
        simple = simple.attachments(built);
    }

    Ok(EmailContent::builder().simple(simple.build()).build())
}

fn utf8_content(value: &String) -> Result<Content, MailError> {
    Content::builder()
        .data(value)
        .charset("UTF-8")
        .build()
        .map_err(|_| MailError::ValidationError("invalid AWS SES body".to_string()))
}

fn header(name: &str, value: String) -> Result<MessageHeader, MailError> {
    if name.is_empty()
        || name.len() > 126
        || !name
            .bytes()
            .all(|byte| (33..=126).contains(&byte) && byte != b':')
        || value.is_empty()
        || value.len() > 995
        || name.len().saturating_add(value.len()) > 996
        || !value.bytes().all(|byte| (32..=126).contains(&byte))
    {
        return Err(MailError::ValidationError(
            "AWS SES custom header exceeds its printable-ASCII boundary".to_string(),
        ));
    }
    MessageHeader::builder()
        .name(name)
        .value(value)
        .build()
        .map_err(|_| MailError::ValidationError("invalid AWS SES message header".to_string()))
}

fn validate_message_limits(message: &Message) -> Result<(), MailError> {
    let mut size = MESSAGE_ENVELOPE_ALLOWANCE;
    for value in [
        Some(&message.to),
        Some(&message.subject),
        message.from.as_ref(),
        message.body_html.as_ref(),
        message.body_text.as_ref(),
        message.unsubscribe_url.as_ref(),
        message.unsubscribe_email.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        add_size(&mut size, json_string_size(value)?)?;
    }

    for attachment in &message.attachments {
        if attachment.filename.is_empty() || attachment.filename.len() > 255 {
            return Err(MailError::ValidationError(
                "AWS SES attachment filename must contain 1-255 bytes".to_string(),
            ));
        }
        if attachment.mime_type.is_empty() || attachment.mime_type.len() > 78 {
            return Err(MailError::ValidationError(
                "AWS SES attachment MIME type must contain 1-78 bytes".to_string(),
            ));
        }
        if attachment
            .cid
            .as_ref()
            .is_some_and(|cid| cid.is_empty() || cid.len() > 78)
        {
            return Err(MailError::ValidationError(
                "AWS SES attachment Content-ID must contain 1-78 bytes".to_string(),
            ));
        }
        add_size(&mut size, json_string_size(&attachment.filename)?)?;
        add_size(&mut size, json_string_size(&attachment.mime_type)?)?;
        if let Some(cid) = &attachment.cid {
            add_size(&mut size, json_string_size(cid)?)?;
        }
        let encoded = attachment
            .content
            .len()
            .checked_add(2)
            .and_then(|length| length.checked_div(3))
            .and_then(|length| length.checked_mul(4))
            .ok_or_else(message_size_error)?;
        add_size(&mut size, encoded)?;
    }
    Ok(())
}

fn json_string_size(value: &str) -> Result<usize, MailError> {
    serde_json::to_vec(value)
        .map(|encoded| encoded.len())
        .map_err(|_| MailError::ValidationError("AWS SES message encoding failed".to_string()))
}

fn add_size(total: &mut usize, additional: usize) -> Result<(), MailError> {
    *total = total
        .checked_add(additional)
        .ok_or_else(message_size_error)?;
    if *total > MAX_SES_V2_MESSAGE_BYTES {
        return Err(message_size_error());
    }
    Ok(())
}

fn message_size_error() -> MailError {
    MailError::ValidationError(
        "AWS SES v2 message exceeds the 40 MiB encoded safety boundary".to_string(),
    )
}

fn map_sdk_error(
    error: aws_sdk_sesv2::error::SdkError<
        aws_sdk_sesv2::operation::send_email::SendEmailError,
        aws_sdk_sesv2::config::http::HttpResponse,
    >,
) -> MailError {
    let Some(service_error) = error.as_service_error() else {
        return MailError::transport(
            "aws_ses",
            "AWS SDK request failed before a service response",
        );
    };
    let status = if service_error.is_too_many_requests_exception() {
        429
    } else {
        error
            .raw_response()
            .map(|response| response.status().as_u16())
            .unwrap_or(502)
    };
    let detail = service_error
        .message()
        .or_else(|| service_error.code())
        .unwrap_or("AWS SES rejected the request");
    let detail = crate::security::redact_email_secrets(detail);
    let retry_after = error
        .raw_response()
        .and_then(|response| response.headers().get("retry-after"))
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| std::time::Duration::from_secs(seconds.min(86_400)));
    MailError::from_provider_response("aws_ses", status, detail, retry_after)
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
