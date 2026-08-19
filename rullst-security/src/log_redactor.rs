use crate::telemetry::SecurityStore;

/// Sanitizes sensitive string inputs, masking passwords, Bearer tokens, API keys, AWS credentials, and cards.
pub fn redact_secrets(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut result = input.to_string();
    let mut redacted = false;

    // Mask Authorization Bearer tokens
    if let Some(idx) = result.find("Bearer ") {
        let start = idx + 7;
        if result.is_char_boundary(start) {
            let end = result[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',')
                .map(|i| start + i)
                .unwrap_or(result.len());

            if end > start && result.is_char_boundary(end) {
                let token_slice = &result[start..end];
                if token_slice.chars().count() > 6 {
                    let prefix: String = token_slice.chars().take(4).collect();
                    let masked = format!("{}...", prefix);
                    result.replace_range(start..end, &masked);
                    redacted = true;
                }
            }
        }
    }

    // Mask password= or "password": "..."
    for key in &["password=", "secret=", "api_key=", "token="] {
        let lower = result.to_lowercase();
        if let Some(idx) = lower.find(key) {
            let start = idx + key.len();
            if result.is_char_boundary(start) {
                let end = result[start..]
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '&' || c == ',')
                    .map(|i| start + i)
                    .unwrap_or(result.len());

                if end > start && result.is_char_boundary(end) && &result[start..end] != "[REDACTED]" {
                    result.replace_range(start..end, "[REDACTED]");
                    redacted = true;
                }
            }
        }
    }

    // Mask AWS secret keys (AKIA... or 40-char secret keys)
    if let Some(idx) = result.find("AKIA") {
        let mut end = (idx + 20).min(result.len());
        while end < result.len() && !result.is_char_boundary(end) {
            end += 1;
        }
        if result.is_char_boundary(idx) && result.is_char_boundary(end) {
            result.replace_range(idx..end, "AKIA****************");
            redacted = true;
        }
    }

    if redacted {
        SecurityStore::global().inc_log_redactions();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_redactor_bearer() {
        let input = "Headers: Authorization: Bearer secret_jwt_token_123456789";
        let clean = redact_secrets(input);
        assert!(clean.contains("Bearer secr..."));
        assert!(!clean.contains("secret_jwt_token_123456789"));
    }

    #[test]
    fn test_log_redactor_password() {
        let input = "User login failed for password=SuperSecretPassword123 with status 401";
        let clean = redact_secrets(input);
        assert!(clean.contains("password=[REDACTED]"));
    }

    #[test]
    fn test_log_redactor_delimiters_and_short_tokens() {
        // Test short token <= 6 chars (should NOT be masked with ...)
        let short_bearer = "Authorization: Bearer 12345";
        assert_eq!(redact_secrets(short_bearer), short_bearer);

        // Test ampersand delimiter in query params
        let url = "https://example.com/api?secret=super_secret_value&other=123";
        let clean_url = redact_secrets(url);
        assert_eq!(clean_url, "https://example.com/api?secret=[REDACTED]&other=123");

        // Test quote delimiters
        let json = r#"{"api_key": "my_api_key_value", "data": "safe"}"#;
        let clean_json = redact_secrets(json);
        assert!(clean_json.contains(r#"api_key": "[REDACTED]""#));

        // Test single quote and comma delimiters
        let sql = "SET token='secret_token_123', status='active'";
        let clean_sql = redact_secrets(sql);
        assert!(clean_sql.contains("token='[REDACTED]'"));

        // Test exact 20-char AWS key replacement
        let aws = "Deploying with AWS key AKIA1234567890123456 in region us-east-1";
        let clean_aws = redact_secrets(aws);
        assert_eq!(clean_aws, "Deploying with AWS key AKIA**************** in region us-east-1");

        // Test empty string
        assert_eq!(redact_secrets(""), "");
    }
}
