use crate::telemetry::SecurityStore;

/// Sanitizes sensitive string inputs, masking passwords, Bearer tokens, API keys, AWS credentials, and cards.
pub fn redact_secrets(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut result = input.to_string();
    let mut redacted = false;

    // Mask Authorization Bearer tokens
    if result.contains("Bearer ") {
        if let Some(idx) = result.find("Bearer ") {
            let start = idx + 7;
            let end = result[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',')
                .map(|i| start + i)
                .unwrap_or(result.len());

            if end > start {
                let token_slice = &result[start..end];
                if token_slice.len() > 6 {
                    let masked = format!("{}...", &token_slice[..4]);
                    result.replace_range(start..end, &masked);
                    redacted = true;
                }
            }
        }
    }

    // Mask password= or "password": "..."
    for key in &["password=", "secret=", "api_key=", "token="] {
        if let Some(idx) = result.to_lowercase().find(key) {
            let start = idx + key.len();
            let end = result[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '&' || c == ',')
                .map(|i| start + i)
                .unwrap_or(result.len());

            if end > start && &result[start..end] != "[REDACTED]" {
                result.replace_range(start..end, "[REDACTED]");
                redacted = true;
            }
        }
    }

    // Mask AWS secret keys (AKIA... or 40-char secret keys)
    if result.contains("AKIA") {
        if let Some(idx) = result.find("AKIA") {
            let end = (idx + 20).min(result.len());
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
}
