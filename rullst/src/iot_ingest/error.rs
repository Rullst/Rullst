//! Typed refusals of the telemetry receiver and their HTTP answers.

use rullst_core::web::axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use std::fmt;
use std::time::Duration;

/// Why the receiver refused a telemetry request. Each variant has a stable
/// [`Self::code`] and HTTP status; responses are JSON
/// `{"error": "<code>"}` with `Cache-Control: no-store`.
///
/// Added in v13.0.0-alpha.5.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IngestError {
    /// `401`: the request reached the handler without a device identity.
    /// The route is not registered as a machine endpoint, or the verifier
    /// was bypassed; the receiver never accepts anonymous telemetry.
    Unauthenticated,
    /// `415`: the body is not `application/json`.
    UnsupportedMediaType,
    /// `413`: the body is larger than the configured limit.
    PayloadTooLarge,
    /// `413`: more readings than the configured limit in one request.
    TooManyReadings,
    /// `400`: the body is not one reading or an array of readings.
    Malformed,
    /// `400`: the firmware header is longer than 64 bytes or not printable
    /// ASCII.
    InvalidFirmware,
    /// `403`: a reading names another device than the token's.
    DeviceMismatch {
        /// Index of the first such reading.
        index: usize,
    },
    /// `422`: a reading has an invalid metric, a non-finite value or a
    /// timestamp outside the accepted window.
    InvalidReading {
        /// Index of the first invalid reading.
        index: usize,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// `429`: the device sent more requests than its window allows.
    RateLimited {
        /// When the device may send again.
        retry_after: Duration,
    },
    /// `503`: the device directory could not record the delivery.
    Unavailable,
}

impl IngestError {
    /// The stable machine-readable code sent as `error`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unauthenticated => "unauthenticated",
            Self::UnsupportedMediaType => "unsupported_media_type",
            Self::PayloadTooLarge => "payload_too_large",
            Self::TooManyReadings => "too_many_readings",
            Self::Malformed => "malformed",
            Self::InvalidFirmware => "invalid_firmware",
            Self::DeviceMismatch { .. } => "device_mismatch",
            Self::InvalidReading { .. } => "invalid_reading",
            Self::RateLimited { .. } => "rate_limited",
            Self::Unavailable => "unavailable",
        }
    }

    /// The HTTP status of the refusal.
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::PayloadTooLarge | Self::TooManyReadings => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Malformed | Self::InvalidFirmware => StatusCode::BAD_REQUEST,
            Self::DeviceMismatch { .. } => StatusCode::FORBIDDEN,
            Self::InvalidReading { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl fmt::Display for IngestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceMismatch { index } => {
                write!(formatter, "reading {index} belongs to another device")
            }
            Self::InvalidReading { index, reason } => {
                write!(formatter, "reading {index} is invalid: {reason}")
            }
            Self::RateLimited { retry_after } => write!(
                formatter,
                "rate limited; retry after {} s",
                retry_after.as_secs().max(1)
            ),
            other => formatter.write_str(other.code()),
        }
    }
}

impl std::error::Error for IngestError {}

impl IntoResponse for IngestError {
    fn into_response(self) -> Response {
        let mut body = serde_json::json!({ "error": self.code() });
        match &self {
            Self::DeviceMismatch { index } => body["index"] = (*index).into(),
            Self::InvalidReading { index, reason } => {
                body["index"] = (*index).into();
                body["reason"] = (*reason).into();
            }
            _ => {}
        }
        let mut response = (self.status(), Json(body)).into_response();
        let headers = response.headers_mut();
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        if let Self::RateLimited { retry_after } = &self
            && let Ok(value) = HeaderValue::from_str(&retry_after.as_secs().max(1).to_string())
        {
            headers.insert(header::RETRY_AFTER, value);
        }
        response
    }
}
