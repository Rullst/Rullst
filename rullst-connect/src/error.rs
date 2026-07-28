use thiserror::Error;

/// Official errors of the Rullst Connect library
#[derive(Error, Debug)]
pub enum ConnectError {
    #[error("HTTP request failed: {0}")]
    Reqwest(String),

    #[error("Failed to parse JSON: {0}")]
    Json(String),

    #[error("Failed to decode Base64: {0}")]
    Base64(String),

    #[error("JWT processing failed: {0}")]
    Jwt(String),

    #[error("System time error: {0}")]
    Time(String),

    #[error("Missing token or unexpected response: {0}")]
    Token(String),

    #[error("Provider API Error ({code}): {message}")]
    ProviderApiError { code: String, message: String },

    #[error("Provider specific error: {0}")]
    Provider(String),

    #[error("Invalid CSRF state: {0}")]
    InvalidState(String),
}

impl From<reqwest::Error> for ConnectError {
    fn from(err: reqwest::Error) -> Self {
        ConnectError::Reqwest(err.to_string())
    }
}

impl From<serde_json::Error> for ConnectError {
    fn from(err: serde_json::Error) -> Self {
        ConnectError::Json(err.to_string())
    }
}

impl From<base64::DecodeError> for ConnectError {
    fn from(err: base64::DecodeError) -> Self {
        ConnectError::Base64(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for ConnectError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        ConnectError::Jwt(err.to_string())
    }
}

impl From<std::time::SystemTimeError> for ConnectError {
    fn from(err: std::time::SystemTimeError) -> Self {
        ConnectError::Time(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_reqwest_error_conversion() {
        let err = reqwest::Client::new()
            .get("htt p://invalid")
            .build()
            .unwrap_err();
        let connect_err: ConnectError = err.into();
        match connect_err {
            ConnectError::Reqwest(_) => (),
            _ => panic!("Expected ConnectError::Reqwest"),
        }
    }

    #[test]
    fn test_serde_json_error_conversion() {
        let err: serde_json::Error =
            serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let connect_err: ConnectError = err.into();
        match connect_err {
            ConnectError::Json(_) => (),
            _ => panic!("Expected ConnectError::Json"),
        }
    }

    #[test]
    fn test_base64_error_conversion() {
        use base64::Engine;
        let err = base64::engine::general_purpose::STANDARD
            .decode("invalid!base64")
            .unwrap_err();
        let connect_err: ConnectError = err.into();
        match connect_err {
            ConnectError::Base64(_) => (),
            _ => panic!("Expected ConnectError::Base64"),
        }
    }

    #[test]
    fn test_jwt_error_conversion() {
        let err = jsonwebtoken::decode_header("invalid.jwt.header").unwrap_err();
        let connect_err: ConnectError = err.into();
        match connect_err {
            ConnectError::Jwt(_) => (),
            _ => panic!("Expected ConnectError::Jwt"),
        }
    }

    #[test]
    fn test_time_error_conversion() {
        let err = std::time::SystemTime::UNIX_EPOCH
            .duration_since(std::time::SystemTime::now())
            .unwrap_err();
        let connect_err: ConnectError = err.into();
        match connect_err {
            ConnectError::Time(_) => (),
            _ => panic!("Expected ConnectError::Time"),
        }
    }

    #[test]
    fn test_error_debug_and_display() {
        let errors = vec![
            ConnectError::Reqwest("test".to_string()),
            ConnectError::Json("test".to_string()),
            ConnectError::Base64("test".to_string()),
            ConnectError::Jwt("test".to_string()),
            ConnectError::Time("test".to_string()),
            ConnectError::Token("test".to_string()),
            ConnectError::ProviderApiError {
                code: "400".to_string(),
                message: "test".to_string(),
            },
            ConnectError::Provider("test".to_string()),
            ConnectError::InvalidState("test".to_string()),
        ];

        for err in errors {
            let _debug = format!("{:?}", err);
            let _display = format!("{}", err);
        }
    }
}
