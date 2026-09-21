//! Exact machine routes authenticated before their cookie-CSRF exemption.

use axum::{
    extract::Request,
    http::{Method, StatusCode, header},
};
use std::sync::Arc;
use subtle::ConstantTimeEq;

/// Authentication category used by an explicitly mounted machine endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MachineAuthentication {
    /// Explicit authorization header checked by a credential or current-state verifier.
    Bearer,
    /// Provider-specific cryptographic signature and freshness verification.
    SignedWebhook,
    /// Identity established by the trusted TLS transport.
    MutualTls,
}

/// Redacted configuration/authentication failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MachineEndpointError {
    /// Unsupported method, wildcard or noncanonical route.
    #[error("machine endpoint requires an exact canonical path and a write method")]
    InvalidRoute,
    /// Missing, weak or structurally unsafe credential.
    #[error("machine bearer credential must contain 32–200 non-whitespace ASCII bytes")]
    InvalidCredential,
    /// Ambiguous or oversized endpoint registry.
    #[error("machine endpoint policy exceeds 32 routes or contains a duplicate")]
    InvalidPolicy,
    /// Request proof was not accepted by the verifier.
    #[error("machine request authentication failed")]
    Unauthorized,
}

/// Host-owned verifier for authoritative bearer credentials, signed webhooks or
/// transport-authenticated mTLS identities. Implementations must fail closed, bind the exact request,
/// and enforce provider freshness/replay policy where applicable. Never accept
/// a browser-supplied header as proof of a client certificate.
///
/// The policy supplies a buffered body of at most 1 MiB. Return the authenticated
/// request with that body preserved for the handler.
#[async_trait::async_trait]
pub trait MachineRequestVerifier: Send + Sync {
    /// Authenticates and returns the same method, URI and readable body.
    async fn verify(&self, request: Request) -> Result<Request, MachineEndpointError>;
}

enum Credential {
    Bearer([u8; 32]),
    Verified(Arc<dyn MachineRequestVerifier>),
}

/// One exact method/path pair and its mandatory authentication mechanism.
pub struct MachineEndpoint {
    method: Method,
    path: String,
    kind: MachineAuthentication,
    credential: Credential,
}

impl MachineEndpoint {
    /// Returns the configured authentication category, without credentials.
    pub fn authentication(&self) -> MachineAuthentication {
        self.kind
    }

    /// Registers a strong bearer credential. Only its SHA-256 digest is retained.
    pub fn bearer(
        method: Method,
        path: impl Into<String>,
        token: impl Into<String>,
    ) -> Result<Self, MachineEndpointError> {
        let token = token.into();
        if !(32..=200).contains(&token.len()) || !token.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(MachineEndpointError::InvalidCredential);
        }
        Self::new(
            method,
            path.into(),
            MachineAuthentication::Bearer,
            Credential::Bearer(digest(token.as_bytes())),
        )
    }

    /// Registers an exact machine route with an authoritative bearer verifier.
    /// The verifier must require one Authorization header and validate current
    /// revocation/scope state. Ambient browser credentials remain rejected.
    pub fn verified_bearer(
        method: Method,
        path: impl Into<String>,
        verifier: impl MachineRequestVerifier + 'static,
    ) -> Result<Self, MachineEndpointError> {
        Self::new(
            method,
            path.into(),
            MachineAuthentication::Bearer,
            Credential::Verified(Arc::new(verifier)),
        )
    }

    /// Registers an exact signed-webhook route with a mandatory verifier.
    pub fn signed_webhook(
        method: Method,
        path: impl Into<String>,
        verifier: impl MachineRequestVerifier + 'static,
    ) -> Result<Self, MachineEndpointError> {
        Self::new(
            method,
            path.into(),
            MachineAuthentication::SignedWebhook,
            Credential::Verified(Arc::new(verifier)),
        )
    }

    /// Registers an exact route with a trusted transport-identity verifier.
    pub fn mutual_tls(
        method: Method,
        path: impl Into<String>,
        verifier: impl MachineRequestVerifier + 'static,
    ) -> Result<Self, MachineEndpointError> {
        Self::new(
            method,
            path.into(),
            MachineAuthentication::MutualTls,
            Credential::Verified(Arc::new(verifier)),
        )
    }

    fn new(
        method: Method,
        path: String,
        kind: MachineAuthentication,
        credential: Credential,
    ) -> Result<Self, MachineEndpointError> {
        if !matches!(
            method,
            Method::POST | Method::PUT | Method::PATCH | Method::DELETE
        ) || !path.starts_with('/')
            || path.len() > 512
            || path.contains("//")
            || path.split('/').any(|part| matches!(part, "." | ".."))
            || !path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.'))
        {
            return Err(MachineEndpointError::InvalidRoute);
        }
        Ok(Self {
            method,
            path,
            kind,
            credential,
        })
    }
}

fn digest(value: &[u8]) -> [u8; 32] {
    let hash = ring::digest::digest(&ring::digest::SHA256, value);
    let mut result = [0; 32];
    result.copy_from_slice(hash.as_ref());
    result
}

/// Immutable bounded policy installed through `Server::with_machine_endpoints`
/// or `apply_security_baseline_with_machine_endpoints`.
#[derive(Clone)]
pub struct MachineEndpointPolicy(Arc<[MachineEndpoint]>);

impl std::fmt::Debug for MachineEndpointPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MachineEndpointPolicy")
            .field("routes", &self.0.len())
            .finish()
    }
}

#[derive(Clone)]
struct MachineAuthenticated {
    method: Method,
    path: String,
    policy: MachineEndpointPolicy,
}

impl MachineEndpointPolicy {
    /// Rejects duplicate method/path pairs and registries exceeding 32 entries.
    pub fn new(endpoints: Vec<MachineEndpoint>) -> Result<Self, MachineEndpointError> {
        if endpoints.len() > 32
            || endpoints.iter().enumerate().any(|(i, endpoint)| {
                endpoints[..i]
                    .iter()
                    .any(|prior| prior.method == endpoint.method && prior.path == endpoint.path)
            })
        {
            return Err(MachineEndpointError::InvalidPolicy);
        }
        Ok(Self(endpoints.into()))
    }

    pub(crate) fn matches(&self, request: &Request) -> bool {
        self.0
            .iter()
            .any(|e| e.method == request.method() && e.path == request.uri().path())
    }

    pub(crate) async fn authenticate(&self, mut request: Request) -> Result<Request, StatusCode> {
        if request
            .extensions()
            .get::<MachineAuthenticated>()
            .is_some_and(|proof| {
                proof.method == request.method()
                    && proof.path == request.uri().path()
                    && Arc::ptr_eq(&proof.policy.0, &self.0)
            })
        {
            return Ok(request);
        }
        let endpoint = self
            .0
            .iter()
            .find(|e| e.method == request.method() && e.path == request.uri().path())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        // A machine exception must never become an ambient-cookie browser path.
        if request.headers().contains_key(header::COOKIE)
            || request.headers().contains_key(header::ORIGIN)
            || request.headers().contains_key("sec-fetch-site")
        {
            return Err(StatusCode::UNAUTHORIZED);
        }
        if let Credential::Bearer(expected) = &endpoint.credential {
            let mut values = request.headers().get_all(header::AUTHORIZATION).iter();
            let token = values
                .next()
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .ok_or(StatusCode::UNAUTHORIZED)?;
            if values.next().is_some()
                || !(32..=200).contains(&token.len())
                || !bool::from(digest(token.as_bytes()).ct_eq(expected))
            {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        let method = request.method().clone();
        let uri = request.uri().clone();
        let (parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, 1024 * 1024)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        request = Request::from_parts(parts, axum::body::Body::from(bytes));
        if let Credential::Verified(verifier) = &endpoint.credential {
            request = verifier
                .verify(request)
                .await
                .map_err(|_| StatusCode::UNAUTHORIZED)?;
            if request.method() != method || request.uri() != &uri {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        request.extensions_mut().insert(MachineAuthenticated {
            method,
            path: uri.path().to_owned(),
            policy: self.clone(),
        });
        Ok(request)
    }
}

pub(crate) async fn authenticate_machine_request(
    request: Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if let Some(policy) = request.extensions().get::<MachineEndpointPolicy>().cloned()
        && policy.matches(&request)
    {
        return match policy.authenticate(request).await {
            Ok(request) => next.run(request).await,
            Err(status) => (status, "Machine request rejected").into_response(),
        };
    }
    next.run(request).await
}
