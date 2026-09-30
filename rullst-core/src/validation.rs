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
/// or standard JSON responses for REST clients: `422` for a payload that fails
/// its `validator` constraints, and for an extraction failure `413` (body too
/// large), `415` (unsupported content type) or `400` (any other unreadable or
/// mistyped payload, including Axum's `422` data errors, so that `422` always
/// means a constraint failure).
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
///
/// `Display` and `Debug` list only field paths and validator codes. The
/// `validator` derive stores each rejected input as a `value` parameter, so
/// the raw errors (which can hold a password or other personal data) are
/// never formatted.
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
                f.write_str("Validation error: ")?;
                for (index, (path, codes)) in error_codes(errors).iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{path} ({})", codes.join(", "))?;
                }
                Ok(())
            }
        }
    }
}

impl std::fmt::Debug for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::ExtractionError { message, is_htmx } => f
                .debug_struct("ExtractionError")
                .field("message", message)
                .field("is_htmx", is_htmx)
                .finish(),
            ValidationError::ValidationError { errors, is_htmx } => f
                .debug_struct("ValidationError")
                .field("errors", &error_codes(errors))
                .field("is_htmx", is_htmx)
                .finish(),
        }
    }
}

/// Field paths and their validator codes, sorted; never parameters or values.
fn error_codes(
    errors: &validator::ValidationErrors,
) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut codes = std::collections::BTreeMap::<String, Vec<String>>::new();
    visit_field_errors(errors, "", &mut |path, field_error| {
        codes
            .entry(path.to_string())
            .or_default()
            .push(field_error.code.to_string());
    });
    codes
}

impl std::error::Error for ValidationError {}

/// REST status of an extraction failure, recovered from its fixed message.
/// Any other message, such as one built by application code, maps to `400`.
fn extraction_status(message: &str) -> StatusCode {
    [
        StatusCode::PAYLOAD_TOO_LARGE,
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
    ]
    .into_iter()
    .find(|status| extraction_failure_message(*status) == message)
    .unwrap_or(StatusCode::BAD_REQUEST)
}

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

/// Visits every field error, including those of `#[validate(nested)]`
/// structs (`address.zip`) and lists (`items[0].name`), with its field path.
fn visit_field_errors(
    errors: &validator::ValidationErrors,
    prefix: &str,
    visit: &mut impl FnMut(&str, &validator::ValidationError),
) {
    for (field, kind) in errors.errors() {
        let path = if prefix.is_empty() {
            field.to_string()
        } else {
            format!("{prefix}.{field}")
        };
        match kind {
            validator::ValidationErrorsKind::Field(field_errors) => {
                for field_error in field_errors {
                    visit(&path, field_error);
                }
            }
            validator::ValidationErrorsKind::Struct(inner) => {
                visit_field_errors(inner, &path, visit);
            }
            validator::ValidationErrorsKind::List(items) => {
                for (index, inner) in items {
                    visit_field_errors(inner, &format!("{path}[{index}]"), visit);
                }
            }
        }
    }
}

fn format_errors(errors: &validator::ValidationErrors) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    visit_field_errors(errors, "", &mut |path, field_error| {
        let message = field_error
            .message
            .as_ref()
            .map(|message| message.to_string())
            .unwrap_or_else(|| format!("Invalid value for field '{path}'"));
        map.entry(path.to_string()).or_default().push(message);
    });
    map
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        match self {
            ValidationError::ExtractionError { message, is_htmx } => {
                let status = extraction_status(&message);
                if is_htmx {
                    let html_error = format!(
                        r#"<div class="p-4 mb-4 rounded-lg bg-red-950/50 border border-red-500/30 text-red-200 text-sm">
                            <span class="font-semibold text-red-400">Request Error:</span> {}
                        </div>"#,
                        crate::html::escape_str(&message)
                    );
                    htmx_fragment(status, html_error)
                } else {
                    let mut err_map = HashMap::new();
                    err_map.insert("error".to_string(), vec![message]);
                    (status, Json(err_map)).into_response()
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
#[path = "validation_tests.rs"]
mod tests;
