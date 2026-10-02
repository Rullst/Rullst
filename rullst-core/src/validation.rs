use axum::{
    Form, Json,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::collections::HashMap;
pub use validator::Validate;

/// Error type returned by [`ValidatedForm`] and [`ValidatedJson`] extractors.
/// Automatically renders HTMX-friendly HTML error components for HTMX requests,
/// or standard JSON `422`/`400` responses for REST clients.
///
/// HTMX 1.x and 2.x swap only successful responses by default, so an HTMX
/// request (`HX-Request: true`) receives the fragment with `200 OK` and the
/// `X-Rullst-Validation-Status` header set to the `400` or `422` a REST client
/// would receive. htmx swaps it into the request's `hx-target` with its
/// `hx-swap`; client scripts can read that header to tell a validation failure
/// from success.
///
/// Every message, field name and validator message is HTML-escaped before it is
/// placed in the HTMX fragment. The built-in extractors never copy the
/// deserializer's error text (which can echo request input) into
/// [`ValidationError::ExtractionError`]; they use a fixed message chosen from the
/// rejection's status and log the detail server-side at `debug` level on the
/// `rullst::validation` target.
#[derive(Debug)]
pub enum ValidationError {
    /// A deserialization error occurred before validation could run (e.g. malformed JSON body).
    ExtractionError {
        /// Human-readable description of the extraction failure.
        message: String,
        /// `true` if the request was triggered by HTMX (`HX-Request: true` header present).
        is_htmx: bool,
    },
    /// The payload was extracted successfully but failed `validator::Validate` constraints.
    ValidationError {
        /// The set of field-level validation errors returned by the `validator` crate.
        errors: validator::ValidationErrors,
        /// `true` if the request was triggered by HTMX (`HX-Request: true` header present).
        is_htmx: bool,
    },
}

impl std::fmt::Display for ValidationError {
    #[cfg_attr(mutants, mutants::skip)]
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

/// Fixed client-facing text for an extraction failure.
///
/// Rejection details from `serde` echo request input (for example
/// ``unknown variant `<payload>` ``), so they are logged server-side and never
/// returned to the client.
fn extraction_failure_message(status: StatusCode) -> &'static str {
    match status {
        StatusCode::UNSUPPORTED_MEDIA_TYPE => "Unsupported request content type.",
        StatusCode::PAYLOAD_TOO_LARGE => "Request body is too large.",
        _ => "The submitted data could not be read or has an invalid format.",
    }
}

fn extraction_error(status: StatusCode, detail: String, is_htmx: bool) -> ValidationError {
    tracing::debug!(
        target: "rullst::validation",
        status = status.as_u16(),
        detail = %detail,
        "request extraction failed"
    );
    ValidationError::ExtractionError {
        message: extraction_failure_message(status).to_string(),
        is_htmx,
    }
}

/// Response header carrying the REST status of an HTMX validation fragment.
const VALIDATION_STATUS_HEADER: &str = "x-rullst-validation-status";

/// Wraps an HTMX error fragment in a response htmx swaps by default.
fn htmx_fragment(status: StatusCode, html: String) -> Response {
    let mut response = (StatusCode::OK, Html(html)).into_response();
    response.headers_mut().insert(
        axum::http::HeaderName::from_static(VALIDATION_STATUS_HEADER),
        axum::http::HeaderValue::from(status.as_u16()),
    );
    response
}

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
                        crate::html::escape_str(&message)
                    );
                    htmx_fragment(StatusCode::BAD_REQUEST, html_error)
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
                            let _ = std::fmt::Write::write_fmt(
                                &mut list_items,
                                format_args!(
                                    r#"<li><span class="font-semibold text-red-300 capitalize">{}</span>: {}</li>"#,
                                    crate::html::escape_str(field),
                                    crate::html::escape_str(msg)
                                ),
                            );
                        }
                    }

                    let html_content = format!(
                        r#"<div class="p-4 mb-4 rounded-lg bg-red-950/50 border border-red-500/30 text-red-200 text-sm animate-pulse-subtle">
                            <div class="flex items-center gap-2 mb-2 font-semibold text-red-400">
                                <svg aria-hidden="true" class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
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

                    htmx_fragment(StatusCode::UNPROCESSABLE_ENTITY, html_content)
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

        let Form(value) = Form::<T>::from_request(req, state)
            .await
            .map_err(|e| extraction_error(e.status(), e.body_text(), is_htmx))?;

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

    #[cfg_attr(mutants, mutants::skip)]
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let is_htmx = req
            .headers()
            .get("HX-Request")
            .and_then(|v| v.to_str().ok())
            .map(|v| v == "true")
            .unwrap_or(false);

        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| extraction_error(e.status(), e.body_text(), is_htmx))?;

        value
            .validate()
            .map_err(|errors| ValidationError::ValidationError { errors, is_htmx })?;

        Ok(ValidatedJson(value))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
                // htmx swaps only 2xx/3xx responses, so the fragment is a 200.
                assert_eq!(response.status(), StatusCode::OK);
                assert_eq!(response.headers()[VALIDATION_STATUS_HEADER], "422");

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

    #[tokio::test]
    async fn test_extraction_error_responses() {
        let err_htmx = ValidationError::ExtractionError {
            message: "Malformed form data".to_string(),
            is_htmx: true,
        };
        assert!(format!("{}", err_htmx).contains("Malformed form data"));

        let resp_htmx = err_htmx.into_response();
        assert_eq!(resp_htmx.status(), StatusCode::OK);
        assert_eq!(resp_htmx.headers()[VALIDATION_STATUS_HEADER], "400");

        let err_json = ValidationError::ExtractionError {
            message: "Invalid JSON syntax".to_string(),
            is_htmx: false,
        };
        let resp_json = err_json.into_response();
        assert_eq!(resp_json.status(), StatusCode::BAD_REQUEST);
        assert!(!resp_json.headers().contains_key(VALIDATION_STATUS_HEADER));
    }

    #[derive(Debug, Deserialize, Validate)]
    struct RolePayload {
        #[allow(dead_code)]
        role: Role,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum Role {
        Admin,
    }

    async fn body_string(response: Response) -> String {
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn htmx_extraction_error_does_not_echo_request_input() {
        let payload = "%3Cimg%20src%3Dx%20onerror%3Dalert(1)%3E";
        let req = Request::builder()
            .method("POST")
            .header("content-type", "application/x-www-form-urlencoded")
            .header("HX-Request", "true")
            .body(axum::body::Body::from(format!("role={payload}")))
            .unwrap();
        let err = ValidatedForm::<RolePayload>::from_request(req, &())
            .await
            .unwrap_err();
        let body = body_string(err.into_response()).await;
        assert!(!body.contains("<img"), "{body}");
        assert!(!body.contains("onerror"), "{body}");
        assert!(body.contains("could not be read"), "{body}");

        let req = Request::builder()
            .header("content-type", "application/json")
            .header("HX-Request", "true")
            .body(axum::body::Body::from(r#"{"role":"<div hx-get=\"/x\">"}"#))
            .unwrap();
        let err = ValidatedJson::<RolePayload>::from_request(req, &())
            .await
            .unwrap_err();
        let body = body_string(err.into_response()).await;
        assert!(!body.contains("hx-get"), "{body}");

        let req = Request::builder()
            .header("content-type", "text/plain")
            .body(axum::body::Body::from("{}"))
            .unwrap();
        let err = ValidatedJson::<RolePayload>::from_request(req, &())
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Extraction error: Unsupported request content type."
        );
    }

    #[tokio::test]
    async fn htmx_fragments_escape_messages_and_fields() {
        let err = ValidationError::ExtractionError {
            message: "<script>alert(1)</script>".to_string(),
            is_htmx: true,
        };
        let body = body_string(err.into_response()).await;
        assert!(!body.contains("<script>"), "{body}");
        assert!(body.contains("&lt;script&gt;"), "{body}");

        let mut errors = validator::ValidationErrors::new();
        let mut field_error = validator::ValidationError::new("custom");
        field_error.message = Some("<b onmouseover=x>bad</b>".into());
        errors.add("name", field_error);
        let body = body_string(
            ValidationError::ValidationError {
                errors,
                is_htmx: true,
            }
            .into_response(),
        )
        .await;
        assert!(!body.contains("<b onmouseover"), "{body}");
        assert!(body.contains("&lt;b onmouseover=x&gt;"), "{body}");
    }
}
