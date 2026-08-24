use crate::ai::AiError;

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
            value.as_f64().map(|number| number as f32).ok_or_else(|| {
                AiError::ApiError(format!("{provider} returned a non-numeric embedding value"))
            })
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
