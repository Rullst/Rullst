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
                return failure_response(status, client_request_id(&body), failure);
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

    /// Echoes the caller's validated `request_id` when a rejected envelope
    /// still carries one (for example an unsupported version or a payload of
    /// the wrong shape), so the client correlates the failure and sees its real
    /// code instead of `rpc.correlation_mismatch`. Otherwise a fresh
    /// framework identifier is used.
    fn client_request_id(body: &[u8]) -> RequestId {
        #[derive(serde::Deserialize)]
        struct Correlation {
            request_id: RequestId,
        }
        serde_json::from_slice::<Correlation>(body)
            .map(|correlation| correlation.request_id)
            .unwrap_or_else(|_| server_request_id())
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

#[cfg(all(test, not(target_arch = "wasm32")))]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::client_contract::ClientContractPolicy;

    async fn failure_for(body: &str) -> (u16, String, String) {
        let request = axum::http::Request::post("/api/rpc/test")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body.to_string()))
            .unwrap();
        let response =
            handle_request::<u32, u32, _, _>(request, |value| async move { Ok(value) }).await;
        let status = response.status().as_u16();
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let failure = ClientContractPolicy::default()
            .decode_failure(&bytes)
            .expect("a framework failure envelope");
        (
            status,
            failure.request_id().as_str().to_string(),
            failure.error().code().as_str().to_string(),
        )
    }

    #[tokio::test]
    async fn rejected_envelopes_echo_the_client_request_id() {
        let (status, id, code) = failure_for(
            r#"{"contract":"rullst.client","version":99,"request_id":"rpc_client1","payload":1}"#,
        )
        .await;
        assert_eq!(
            (status, id.as_str(), code.as_str()),
            (409, "rpc_client1", "rpc.version_unsupported")
        );

        let (status, id, code) = failure_for(
            r#"{"contract":"rullst.client","version":1,"request_id":"rpc_client2","payload":"x"}"#,
        )
        .await;
        assert_eq!(
            (status, id.as_str(), code.as_str()),
            (400, "rpc_client2", "rpc.request_invalid")
        );

        // Without a valid identifier the server still answers with its own.
        let (status, id, code) = failure_for(r#"{"request_id":"has space"}"#).await;
        assert_eq!((status, code.as_str()), (400, "rpc.request_invalid"));
        assert!(id.starts_with("rpc_") && id != "has space", "{id}");
    }
}
