use crate::ai::AiError;
use std::time::Duration;

pub(super) const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn endpoint(base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

pub(super) async fn success_response(
    response: reqwest::Response,
    provider: &'static str,
) -> Result<reqwest::Response, AiError> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(AiError::ApiError(format!(
            "{provider} returned HTTP {}",
            response.status()
        )))
    }
}

pub(super) fn openai_chat_content(
    response: &serde_json::Value,
    provider: &'static str,
) -> Result<String, AiError> {
    response["choices"][0]["message"]["content"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| AiError::ApiError(format!("{provider} returned no message content")))
}

pub(super) fn embedding_values(
    values: Option<&Vec<serde_json::Value>>,
    provider: &'static str,
) -> Result<Vec<f32>, AiError> {
    let values = values
        .ok_or_else(|| AiError::ApiError(format!("{provider} returned no embedding values")))?;
    values
        .iter()
        .map(|value| {
            let number = value.as_f64().ok_or_else(|| {
                AiError::ApiError(format!("{provider} returned a non-numeric embedding value"))
            })?;
            let number = number as f32;
            if number.is_finite() {
                Ok(number)
            } else {
                Err(AiError::ApiError(format!(
                    "{provider} returned a non-finite embedding value"
                )))
            }
        })
        .collect()
}

pub(super) fn image_mime_type(image_bytes: &[u8]) -> Result<&'static str, AiError> {
    if image_bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Ok("image/jpeg")
    } else if image_bytes.starts_with(&[0x89, 0x50, 0x4e, 0x47]) {
        Ok("image/png")
    } else if image_bytes.starts_with(&[0x52, 0x49, 0x46, 0x46]) {
        Ok("image/webp")
    } else if image_bytes.starts_with(&[0x47, 0x49, 0x46, 0x38]) {
        Ok("image/gif")
    } else {
        Err(AiError::ConfigError(
            "vision input must be JPEG, PNG, WebP, or GIF".to_string(),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_formatting() {
        assert_eq!(
            endpoint("https://api.openai.com/", "/v1/chat"),
            "https://api.openai.com/v1/chat"
        );
        assert_eq!(
            endpoint("https://api.openai.com", "v1/chat"),
            "https://api.openai.com/v1/chat"
        );
        assert_eq!(
            endpoint("https://api.openai.com///", "///v1/chat"),
            "https://api.openai.com/v1/chat"
        );
    }

    #[test]
    fn test_openai_chat_content() {
        let json: serde_json::Value = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "Hello world"
                    }
                }
            ]
        });
        assert_eq!(openai_chat_content(&json, "OpenAI").unwrap(), "Hello world");

        let invalid_json: serde_json::Value = serde_json::json!({ "choices": [] });
        assert!(openai_chat_content(&invalid_json, "OpenAI").is_err());
    }

    #[test]
    fn test_embedding_values() {
        let json_values = vec![
            serde_json::json!(1.0),
            serde_json::json!(2.5),
            serde_json::json!(-0.5),
        ];
        let floats = embedding_values(Some(&json_values), "Provider").unwrap();
        assert_eq!(floats, vec![1.0, 2.5, -0.5]);

        assert!(embedding_values(None, "Provider").is_err());

        let invalid_json_values = vec![serde_json::json!("not-a-number")];
        assert!(embedding_values(Some(&invalid_json_values), "Provider").is_err());

        let overflowing_json_values = vec![serde_json::json!(1.0e308)];
        assert!(embedding_values(Some(&overflowing_json_values), "Provider").is_err());
    }

    #[test]
    fn test_image_mime_type() {
        assert_eq!(
            image_mime_type(&[0xff, 0xd8, 0xff, 0x00]).unwrap(),
            "image/jpeg"
        );
        assert_eq!(
            image_mime_type(&[0x89, 0x50, 0x4e, 0x47, 0x00]).unwrap(),
            "image/png"
        );
        assert_eq!(
            image_mime_type(&[0x52, 0x49, 0x46, 0x46, 0x00]).unwrap(),
            "image/webp"
        );
        assert_eq!(
            image_mime_type(&[0x47, 0x49, 0x46, 0x38, 0x00]).unwrap(),
            "image/gif"
        );
        assert!(image_mime_type(&[0x00, 0x01, 0x02]).is_err());
    }
}
