use std::cell::RefCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
/// Strategy used to extract the active tenant ID from an incoming request.
pub enum TenantStrategy {
    /// Select a tenant from the request host subdomain (e.g. `tenant.example.com`).
    /// The host is the `Host` header or, when it is absent (HTTP/2), the
    /// request `:authority`; a `www` label is not a tenant. The selection is
    /// accepted only when authenticated membership allows it.
    Subdomain,
    /// Select a tenant from a custom HTTP header. The header is an untrusted
    /// hint and is accepted only when authenticated membership allows it.
    Header,
    /// Extract the tenant ID from query parameters.
    ///
    /// # Security Warning
    /// Using query parameters (`?tenant_id=...`) can cause tenant IDs to leak in:
    /// - Server access logs (clear-text)
    /// - Browser history
    /// - `Referer` headers sent to third-party assets
    /// - CDN/proxy cache headers
    ///
    /// For sensitive or regulated tenant IDs, prefer `TenantStrategy::Header` or `TenantStrategy::Subdomain`.
    /// Every strategy still requires a trusted [`crate::security::TenantMembership`]
    /// request extension from authentication middleware.
    Parameter,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
/// Configuration settings for multitenancy extraction.
pub struct TenantConfig {
    /// The extraction strategy to be used.
    pub strategy: TenantStrategy,
    /// The name of the custom HTTP header (used only with `TenantStrategy::Header`).
    pub header_name: String,
    /// The name of the query parameter (used only with `TenantStrategy::Parameter`).
    pub parameter_name: String,
    /// Tenant ID requested by the `Subdomain` strategy when the request host
    /// has no tenant subdomain or no host is present. It is still accepted
    /// only when authenticated membership allows it. The `Header` and
    /// `Parameter` strategies ignore it: without their input they use the
    /// membership's default tenant.
    pub domain_fallback: Option<String>,
    /// Domain the `Subdomain` strategy strips from the host, such as
    /// `escola.com.br` or `example.co.uk`. When set, the tenant is the label
    /// immediately to its left (`acme.escola.com.br` and
    /// `www.acme.escola.com.br` both select `acme`), while the domain itself,
    /// `www.` plus the domain and hosts outside it have no tenant subdomain.
    /// When unset, the first label of a host with at least three labels is
    /// the tenant. Unpublished v13 API.
    pub base_domain: Option<String>,
}

impl TenantConfig {
    /// Creates a new TenantConfig with default values:
    /// - Header Name: X-Tenant-ID
    /// - Parameter Name: tenant_id
    /// - Domain Fallback: None
    /// - Base Domain: None
    pub fn new(strategy: TenantStrategy) -> Self {
        Self {
            strategy,
            header_name: "X-Tenant-ID".to_string(),
            parameter_name: "tenant_id".to_string(),
            domain_fallback: None,
            base_domain: None,
        }
    }

    /// Set a custom HTTP header name to extract the tenant ID from.
    pub fn with_header_name<S: Into<String>>(mut self, name: S) -> Self {
        self.header_name = name.into();
        self
    }

    /// Set a custom Query Parameter name to extract the tenant ID from.
    pub fn with_parameter_name<S: Into<String>>(mut self, name: S) -> Self {
        self.parameter_name = name.into();
        self
    }

    /// Set the tenant ID the `Subdomain` strategy requests when the host has no
    /// tenant subdomain. Other strategies ignore it.
    pub fn with_domain_fallback<S: Into<String>>(mut self, fallback: S) -> Self {
        self.domain_fallback = Some(fallback.into());
        self
    }

    /// Set the domain the `Subdomain` strategy strips from the host (see
    /// [`TenantConfig::base_domain`]). Required when the application's apex
    /// is under a multi-label public suffix such as `.com.br`. Unpublished v13
    /// API.
    pub fn with_base_domain<S: Into<String>>(mut self, domain: S) -> Self {
        self.base_domain = Some(domain.into());
        self
    }
}

tokio::task_local! {
    /// Request-scoped, thread-safe context for storing the active tenant ID.
    pub static TENANT_CONTEXT: RefCell<Option<String>>;
}

/// Retrieve the active request's tenant ID if configured.
pub fn current_tenant_id() -> Option<String> {
    TENANT_CONTEXT
        .try_with(|ctx| ctx.borrow().clone())
        .unwrap_or(None)
}

/// Dynamically sets or replaces the tenant ID for the duration of the current task context.
pub fn set_tenant_id(tenant_id: Option<String>) {
    let _ = TENANT_CONTEXT.try_with(|ctx| {
        *ctx.borrow_mut() = tenant_id;
    });
}

/// Target host of a request: the `Host` header or, when it is absent (as in
/// HTTP/2, where hyper exposes `:authority` only through the URI), the URI
/// authority's host. A present `Host` header always takes precedence.
fn request_host<B>(req: &axum::http::Request<B>) -> Option<&str> {
    match req.headers().get(axum::http::header::HOST) {
        Some(value) => value.to_str().ok(),
        None => req.uri().authority().map(|authority| authority.host()),
    }
}

/// Extracts the tenant subdomain from a request host.
///
/// Without a base domain, the first label of a host with at least three
/// labels is the tenant (`tenant1.example.com` -> `tenant1`). With one, the
/// tenant is the label immediately to the left of it. IP addresses, shorter
/// hosts, hosts outside the base domain and a `www` label have no tenant
/// subdomain, so `domain_fallback` applies. An empty label (a malformed host
/// such as `.example.com`) is returned as an empty request, which membership
/// selection rejects.
fn extract_subdomain(host: &str, base_domain: Option<&str>) -> Option<String> {
    let host_only = host.split(':').next()?;
    if host_only.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    let label = match base_domain {
        Some(base_domain) => {
            let host_only = host_only.strip_suffix('.').unwrap_or(host_only);
            let base_domain = base_domain.trim_matches('.');
            let prefix_len = host_only.len().checked_sub(base_domain.len() + 1)?;
            let suffix = host_only.get(prefix_len..)?;
            if base_domain.is_empty()
                || !suffix.starts_with('.')
                || !suffix[1..].eq_ignore_ascii_case(base_domain)
            {
                return None;
            }
            host_only.get(..prefix_len)?.rsplit('.').next()?
        }
        None => {
            let parts: Vec<&str> = host_only.split('.').collect();
            if parts.len() < 3 {
                return None;
            }
            parts[0]
        }
    };
    // A `www` label means "no tenant subdomain". An empty label comes from a
    // malformed host such as `.example.com`; it is returned as a requested
    // (empty) tenant so membership selection rejects it instead of falling
    // back to the caller's default tenant.
    if label.eq_ignore_ascii_case("www") {
        return None;
    }
    Some(label.to_string())
}

/// The declarative custom Tower Layer for tenant identification
#[derive(Clone, Debug)]
pub struct TenantLayer {
    config: TenantConfig,
}

impl TenantLayer {
    /// Creates a new `TenantLayer` with the specified `TenantConfig`.
    pub fn new(config: TenantConfig) -> Self {
        Self { config }
    }
}

impl<S> tower_layer::Layer<S> for TenantLayer {
    type Service = TenantService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TenantService {
            inner,
            config: self.config.clone(),
        }
    }
}

/// The declarative custom Tower Service for tenant identification
#[derive(Clone, Debug)]
pub struct TenantService<S> {
    inner: S,
    config: TenantConfig,
}

impl<S, ReqBody, ResBody> tower_service::Service<axum::http::Request<ReqBody>> for TenantService<S>
where
    S: tower_service::Service<
            axum::http::Request<ReqBody>,
            Response = axum::http::Response<ResBody>,
        > + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Default + Send + 'static,
{
    type Response = axum::http::Response<ResBody>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: axum::http::Request<ReqBody>) -> Self::Future {
        let config = self.config.clone();
        // `poll_ready` readied `self.inner`, so that instance must handle the
        // request: services such as `ConcurrencyLimit` reserve capacity there
        // and reject a call on a fresh clone. Leave the clone for the next call.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let requested_tenant_id = match config.strategy {
                TenantStrategy::Header => req
                    .headers()
                    .get(&config.header_name)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string()),
                TenantStrategy::Subdomain => request_host(&req)
                    .and_then(|host| extract_subdomain(host, config.base_domain.as_deref()))
                    .or_else(|| config.domain_fallback.clone()),
                TenantStrategy::Parameter => {
                    let query = req.uri().query().unwrap_or("");
                    serde_urlencoded::from_str::<std::collections::HashMap<String, String>>(query)
                        .ok()
                        .and_then(|params| params.get(&config.parameter_name).cloned())
                }
            };

            let selected_context = req
                .extensions()
                .get::<crate::security::TenantMembership>()
                .and_then(|membership| match requested_tenant_id.as_deref() {
                    Some(requested) => membership.select(requested).ok(),
                    None => membership.default_context(),
                });

            let Some(selected_context) = selected_context else {
                let mut response = axum::http::Response::new(ResBody::default());
                *response.status_mut() = axum::http::StatusCode::FORBIDDEN;
                return Ok(response);
            };
            let tenant_id = selected_context.tenant_id.clone();
            req.extensions_mut().insert(selected_context);
            let cell = RefCell::new(Some(tenant_id));
            TENANT_CONTEXT
                .scope(cell, async move { inner.call(req).await })
                .await
        })
    }
}

/// Fluent helper to return a Tower Middleware Layer using TenantConfig
pub fn tenant_layer(config: TenantConfig) -> TenantLayer {
    TenantLayer::new(config)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "multitenant_tests.rs"]
mod tests;
