use axum::{
    Form, Json,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::collections::HashMap;
pub use validator::Validate;

#[derive(Debug)]
pub enum ValidationError {
    ExtractionError {
        message: String,
        is_htmx: bool,
    },
    ValidationError {
        errors: validator::ValidationErrors,
        is_htmx: bool,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::ExtractionError { message, .. } => {
                write!(f, "Extraction error: {}", message)
            }
            ValidationError::ValidationError { errors, .. } => {
                write!(f, "Validation error: {:?}", errors)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

fn format_errors(errors: &validator::ValidationErrors) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for (field, field_errors) in errors.field_errors() {
        let messages: Vec<String> = field_errors
            .iter()
            .map(|fe| {
                fe.message
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| format!("Invalid value for field '{}'", field))
            })
            .collect();
        map.insert(field.to_string(), messages);
    }
    map
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        match self {
            ValidationError::ExtractionError { message, is_htmx } => {
                if is_htmx {
                    let html_error = format!(
                        r#"<div class="p-4 mb-4 rounded-lg bg-red-950/50 border border-red-500/30 text-red-200 text-sm">
                            <span class="font-semibold text-red-400">Request Error:</span> {}
                        </div>"#,
                        message
                    );
                    (StatusCode::BAD_REQUEST, Html(html_error)).into_response()
                } else {
                    let mut err_map = HashMap::new();
                    err_map.insert("error".to_string(), vec![message]);
                    (StatusCode::BAD_REQUEST, Json(err_map)).into_response()
                }
            }
            ValidationError::ValidationError { errors, is_htmx } => {
                let formatted = format_errors(&errors);
                if is_htmx {
                    // Render premium visual UI list of validation errors
                    let mut list_items = String::new();
                    for (field, msgs) in &formatted {
                        for msg in msgs {
                            list_items.push_str(&format!(
                                r#"<li><span class="font-semibold text-red-300 capitalize">{}</span>: {}</li>"#,
                                field, msg
                            ));
                        }
                    }

                    let html_content = format!(
                        r#"<div class="p-4 mb-4 rounded-lg bg-red-950/50 border border-red-500/30 text-red-200 text-sm animate-pulse-subtle">
                            <div class="flex items-center gap-2 mb-2 font-semibold text-red-400">
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
                                </svg>
                                <span>Validation Failed</span>
                            </div>
                            <ul class="list-disc list-inside space-y-1">
                                {}
                            </ul>
                        </div>"#,
                        list_items
                    );

                    (StatusCode::UNPROCESSABLE_ENTITY, Html(html_content)).into_response()
                } else {
                    let mut response_body = HashMap::new();
                    response_body.insert("errors", formatted);
                    (StatusCode::UNPROCESSABLE_ENTITY, Json(response_body)).into_response()
                }
            }
        }
    }
}

/// Extractor for validating form payloads
#[derive(Debug)]
pub struct ValidatedForm<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedForm<T>
where
    T: validator::Validate + serde::de::DeserializeOwned + 'static,
    S: Send + Sync,
{
    type Rejection = ValidationError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let is_htmx = req
            .headers()
            .get("HX-Request")
            .and_then(|v| v.to_str().ok())
            .map(|v| v == "true")
            .unwrap_or(false);

        let Form(value) = Form::<T>::from_request(req, state).await.map_err(|e| {
            ValidationError::ExtractionError {
                message: e.to_string(),
                is_htmx,
            }
        })?;

        value
            .validate()
            .map_err(|errors| ValidationError::ValidationError { errors, is_htmx })?;

        Ok(ValidatedForm(value))
    }
}

/// Extractor for validating JSON payloads
#[derive(Debug)]
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: validator::Validate + serde::de::DeserializeOwned + 'static,
    S: Send + Sync,
{
    type Rejection = ValidationError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let is_htmx = req
            .headers()
            .get("HX-Request")
            .and_then(|v| v.to_str().ok())
            .map(|v| v == "true")
            .unwrap_or(false);

        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|e| {
            ValidationError::ExtractionError {
                message: e.to_string(),
                is_htmx,
            }
        })?;

        value
            .validate()
            .map_err(|errors| ValidationError::ValidationError { errors, is_htmx })?;

        Ok(ValidatedJson(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate, Clone)]
    struct TestPayload {
        #[validate(length(min = 3, message = "Username too short"))]
        username: String,
        #[validate(email(message = "Must be a valid email"))]
        email: String,
    }

    #[tokio::test]
    async fn test_validation_success() {
        let _payload = TestPayload {
            username: "venelouis".to_string(),
            email: "vene@rullst.dev".to_string(),
        };

        // Form success
        let req = Request::builder()
            .method("POST")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(axum::body::Body::from(
                "username=venelouis&email=vene%40rullst.dev",
            ))
            .unwrap();

        let validated = ValidatedForm::<TestPayload>::from_request(req, &())
            .await
            .unwrap();
        assert_eq!(validated.0.username, "venelouis");
        assert_eq!(validated.0.email, "vene@rullst.dev");

        // Json success
        let req_json = Request::builder()
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                r#"{"username": "venelouis", "email": "vene@rullst.dev"}"#,
            ))
            .unwrap();

        let validated_json = ValidatedJson::<TestPayload>::from_request(req_json, &())
            .await
            .unwrap();
        assert_eq!(validated_json.0.username, "venelouis");
    }

    #[tokio::test]
    async fn test_validation_failure_json() {
        let req = Request::builder()
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                r#"{"username": "ab", "email": "invalid-email"}"#,
            ))
            .unwrap();

        let err = ValidatedJson::<TestPayload>::from_request(req, &())
            .await
            .unwrap_err();

        match err {
            ValidationError::ValidationError { errors, is_htmx } => {
                assert!(!is_htmx);
                let formatted = format_errors(&errors);
                assert!(formatted.contains_key("username"));
                assert!(formatted.contains_key("email"));
                assert_eq!(formatted.get("username").unwrap()[0], "Username too short");
                assert_eq!(formatted.get("email").unwrap()[0], "Must be a valid email");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_validation_failure_htmx() {
        let req = Request::builder()
            .method("POST")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("HX-Request", "true")
            .body(axum::body::Body::from("username=ab&email=invalid-email"))
            .unwrap();

        let err = ValidatedForm::<TestPayload>::from_request(req, &())
            .await
            .unwrap_err();

        match err {
            ValidationError::ValidationError { errors, is_htmx } => {
                assert!(is_htmx);
                let response = ValidationError::ValidationError { errors, is_htmx }.into_response();
                assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

                let body_bytes = axum::body::to_bytes(response.into_body(), 10000)
                    .await
                    .unwrap();
                let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
                assert!(body_str.contains("Validation Failed"));
                assert!(body_str.contains("username"));
                assert!(body_str.contains("email"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }
}
