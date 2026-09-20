//! Bounded JSON helpers for the additive transactional REST transports.
use crate::{MailError, Message};
use serde_json::Value;

pub(super) fn sender(message: &Message) -> Result<&str, MailError> {
    if message.body_text.as_deref().is_none_or(str::is_empty)
        && message.body_html.as_deref().is_none_or(str::is_empty)
    {
        return Err(MailError::ValidationError(
            "email requires nonempty text or HTML".into(),
        ));
    }
    message.from.as_deref().ok_or_else(|| {
        MailError::ConfigError("provider requires an explicit verified sender".into())
    })
}

pub(super) fn encode(value: &Value) -> Result<Vec<u8>, MailError> {
    let body = serde_json::to_vec(value).map_err(|_| contract())?;
    if body.len() > 10_000_000 {
        return Err(MailError::ValidationError(
            "encoded email exceeds 10 MB".into(),
        ));
    }
    Ok(body)
}

pub(super) fn headers(message: &Message) -> Value {
    let mut headers = serde_json::Map::new();
    if let Some(value) = message.list_unsubscribe_header() {
        headers.insert("List-Unsubscribe".into(), Value::String(value));
        if message.unsubscribe_url.is_some() {
            headers.insert(
                "List-Unsubscribe-Post".into(),
                Value::String("List-Unsubscribe=One-Click".into()),
            );
        }
    }
    Value::Object(headers)
}

pub(super) fn contract() -> MailError {
    MailError::SendError("mail provider returned an invalid or rejected delivery receipt".into())
}

pub(super) fn valid_id(value: &Value) -> bool {
    value.as_str().is_some_and(|id| {
        !id.is_empty()
            && id.len() <= 255
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    })
}

pub(super) async fn execute(
    request: reqwest::Request,
    provider: &'static str,
) -> Result<Value, MailError> {
    let mut response = super::http::client()?
        .execute(request)
        .await
        .map_err(|_| MailError::transport(provider, "request failed before response"))?;
    if !response.status().is_success() {
        return Err(crate::error::provider_http_error(provider, response).await);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| MailError::transport(provider, "response read failed"))?
    {
        if bytes.len().saturating_add(chunk.len()) > 65536 {
            return Err(contract());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| contract())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    async fn respond(status: &str, body: &str, extra: &str) -> Result<Value, MailError> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n{extra}\r\n{body}",
            body.len()
        );
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut data = [0; 4096];
            let _ = stream.read(&mut data).await.unwrap();
            let _ = stream.write_all(response.as_bytes()).await;
        });
        let request = super::super::http::client()
            .unwrap()
            .get(endpoint)
            .build()
            .unwrap();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            execute(request, "fixture"),
        )
        .await
        .unwrap();
        server.await.unwrap();
        result
    }

    #[tokio::test]
    async fn successful_responses_are_bounded_and_errors_preserve_only_safe_retry_metadata() {
        assert_eq!(
            respond("200 OK", r#"{"success":true}"#, "").await.unwrap()["success"],
            true
        );
        let error = respond(
            "429 Too Many Requests",
            r#"{"error":"private@example.com token=secret"}"#,
            "Retry-After: 7\r\n",
        )
        .await
        .unwrap_err();
        assert_eq!(error.failure_class(), crate::MailFailureClass::RateLimited);
        assert_eq!(error.retry_after(), Some(std::time::Duration::from_secs(7)));
        assert!(!format!("{error:?}").contains("private@example.com"));
        assert!(!format!("{error:?}").contains("token=secret"));
        assert!(respond("200 OK", "not JSON", "").await.is_err());
        assert!(respond("200 OK", &" ".repeat(65537), "").await.is_err());
        assert!(
            respond("302 Found", "", "Location: https://example.com\r\n")
                .await
                .is_err()
        );
    }
}
