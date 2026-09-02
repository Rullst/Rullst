//! Versioned transport primitives for `#[server_function]`.
//!
//! This module owns serialization, size limits, correlation, and redacted
//! failures. Authentication, authorization, tenant scope, idempotency, and
//! domain validation remain server-side application policy. Production routes
//! must be composed with Rullst's security baseline and the application's
//! identity/authorization layers.

use crate::client_contract::{ClientContractError, FailureCode, FailureDetail};

/// A typed result returned by every bounded Rullst server function.
pub type RpcResult<T> = Result<T, RpcFailure>;

/// Machine-readable RPC failure with no arbitrary provider or debug message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("RPC failed with code `{}`", code.as_str())]
#[non_exhaustive]
pub struct RpcFailure {
    code: FailureCode,
    retryable: bool,
}

impl RpcFailure {
    /// Creates an application failure from the bounded lowercase dotted-code
    /// grammar, such as `counter.limit_reached`.
    pub fn application(
        code: impl Into<String>,
        retryable: bool,
    ) -> Result<Self, ClientContractError> {
        Ok(Self {
            code: FailureCode::new(code)?,
            retryable,
        })
    }

    /// Returns the stable failure code.
    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    /// Whether a retry may be appropriate. This never authorizes replaying a
    /// state-changing operation.
    pub const fn retryable(&self) -> bool {
        self.retryable
    }

    pub(crate) fn framework(code: &'static str, retryable: bool) -> Self {
        Self {
            code: FailureCode::framework(code),
            retryable,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn from_detail(detail: &FailureDetail) -> Self {
        Self {
            code: detail.code().clone(),
            retryable: detail.retryable(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn detail(&self) -> FailureDetail {
        FailureDetail::new(self.code.clone(), self.retryable)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod server {
    use super::{RpcFailure, RpcResult};
    use crate::Router;
    use crate::client_contract::{
        CURRENT_CLIENT_CONTRACT_VERSION, ClientContractError, ClientContractPolicy, RequestId,
        ServerFailure, ServerResponse,
    };
    use axum::body::{Body, to_bytes};
    use axum::extract::Request;
    use axum::http::{StatusCode, header};
    use axum::response::{IntoResponse, Response};
    use serde::Serialize;
    use serde::de::DeserializeOwned;
    use std::future::Future;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Builds the single POST route generated for a server function.
    ///
    /// The returned router intentionally has no implicit identity policy. Merge
    /// it into the application before installing the production security,
    /// session, authentication, tenant, authorization, and rate-limit layers.
    pub fn route<H, T>(path: &'static str, handler: H) -> Router
    where
        T: 'static,
        H: axum::handler::Handler<T, ()>,
    {
        Router::new().route(path, axum::routing::post(handler))
    }

    /// Decodes one bounded request, invokes the generated typed adapter, and
    /// emits the matching versioned success or failure envelope.
    pub async fn handle_request<Req, Output, Invoke, InvokeFuture>(
        request: Request,
        invoke: Invoke,
    ) -> Response
    where
        Req: DeserializeOwned + Send + 'static,
        Output: Serialize,
        Invoke: FnOnce(Req) -> InvokeFuture,
        InvokeFuture: Future<Output = RpcResult<Output>>,
    {
        let content_type = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if !is_json_content_type(content_type) {
            return failure_response(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                server_request_id(),
                RpcFailure::framework("rpc.content_type_required", false),
            );
        }

        let (_, body) = request.into_parts();
        let policy = ClientContractPolicy::default();
        let body = match to_bytes(body, policy.max_body_bytes()).await {
            Ok(body) => body,
            Err(_) => {
                return failure_response(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    server_request_id(),
                    RpcFailure::framework("rpc.request_too_large", false),
                );
            }
        };
        let envelope = match policy.decode_request::<Req>(&body) {
            Ok(envelope) => envelope,
            Err(error) => {
                let (status, failure) = request_contract_failure(&error);
                return failure_response(status, server_request_id(), failure);
            }
        };
        let version = envelope.version();
        let request_id = envelope.request_id().clone();

        match invoke(envelope.into_payload()).await {
            Ok(data) => {
                let response =
                    ServerResponse::new(version, request_id.clone(), now_epoch_ms(), data);
                match policy.encode_response(&response) {
                    Ok(body) => json_response(StatusCode::OK, body),
                    Err(_) => failure_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        request_id,
                        RpcFailure::framework("rpc.response_encoding", false),
                    ),
                }
            }
            Err(failure) => failure_response(StatusCode::UNPROCESSABLE_ENTITY, request_id, failure),
        }
    }

    fn request_contract_failure(error: &ClientContractError) -> (StatusCode, RpcFailure) {
        match error {
            ClientContractError::BodyTooLarge { .. } => (
                StatusCode::PAYLOAD_TOO_LARGE,
                RpcFailure::framework("rpc.request_too_large", false),
            ),
            ClientContractError::UnsupportedVersion { .. } => (
                StatusCode::CONFLICT,
                RpcFailure::framework("rpc.version_unsupported", false),
            ),
            _ => (
                StatusCode::BAD_REQUEST,
                RpcFailure::framework("rpc.request_invalid", false),
            ),
        }
    }

    fn failure_response(
        status: StatusCode,
        request_id: RequestId,
        failure: RpcFailure,
    ) -> Response {
        let policy = ClientContractPolicy::default();
        let response = ServerFailure::new(
            CURRENT_CLIENT_CONTRACT_VERSION,
            request_id,
            now_epoch_ms(),
            failure.detail(),
        );
        match policy.encode_failure(&response) {
            Ok(body) => json_response(status, body),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                Body::from(
                    r#"{"contract":"rullst.client","version":1,"error":{"code":"rpc.internal","retryable":false}}"#,
                ),
            )
                .into_response(),
        }
    }

    fn json_response(status: StatusCode, body: Vec<u8>) -> Response {
        (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
    }

    fn server_request_id() -> RequestId {
        let value = format!("rpc_{}", uuid::Uuid::new_v4().simple());
        RequestId::framework(value)
    }

    fn now_epoch_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| u64::try_from(duration.as_millis()).ok())
            .unwrap_or(0)
    }

    fn is_json_content_type(content_type: &str) -> bool {
        content_type
            .split(';')
            .next()
            .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("application/json"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use server::{handle_request, route};
