use super::*;

#[test]
fn test_extract_subdomain() {
    assert_eq!(
        extract_subdomain("tenant1.example.com"),
        Some("tenant1".to_string())
    );
    assert_eq!(
        extract_subdomain("tenant-a.app.co.uk"),
        Some("tenant-a".to_string())
    );
    assert_eq!(extract_subdomain("localhost:3000"), None);
    assert_eq!(extract_subdomain("127.0.0.1"), None);
}

#[test]
fn test_tenant_config_builder() {
    let config = TenantConfig::new(TenantStrategy::Header)
        .with_header_name("X-Custom-Tenant")
        .with_parameter_name("t_id")
        .with_domain_fallback("default");

    assert_eq!(config.strategy, TenantStrategy::Header);
    assert_eq!(config.header_name, "X-Custom-Tenant");
    assert_eq!(config.parameter_name, "t_id");
    assert_eq!(config.domain_fallback, Some("default".to_string()));
}

#[tokio::test]
async fn test_task_local_storage() {
    let cell = RefCell::new(Some("tenant123".to_string()));

    TENANT_CONTEXT
        .scope(cell, async {
            assert_eq!(current_tenant_id(), Some("tenant123".to_string()));

            // Set dynamic value mid-request
            set_tenant_id(Some("super-tenant".to_string()));
            assert_eq!(current_tenant_id(), Some("super-tenant".to_string()));

            set_tenant_id(None);
            assert_eq!(current_tenant_id(), None);
        })
        .await;

    // Outside scope, it should return None
    assert_eq!(current_tenant_id(), None);
}
#[tokio::test]
async fn test_current_tenant_id_uninitialized() {
    assert_eq!(current_tenant_id(), None);
}

#[tokio::test]
async fn test_current_tenant_id_initialized() {
    let cell = RefCell::new(Some("tenant-456".to_string()));
    TENANT_CONTEXT
        .scope(cell, async {
            assert_eq!(current_tenant_id(), Some("tenant-456".to_string()));
        })
        .await;
}

#[tokio::test]
async fn test_tenant_layer_header_and_query() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::get;
    use tower::ServiceExt;

    async fn handler() -> impl IntoResponse {
        let tenant = current_tenant_id().unwrap_or_else(|| "none".to_string());
        (StatusCode::OK, tenant)
    }

    // 1. Header strategy
    let config_header = TenantConfig::new(TenantStrategy::Header);
    let app_header = axum::Router::new()
        .route("/test", get(handler))
        .layer(tenant_layer(config_header))
        .layer(axum::Extension(
            crate::security::TenantMembership::try_new(["acme-corp"]).unwrap(),
        ));

    let req = Request::builder()
        .uri("/test")
        .header("X-Tenant-ID", "acme-corp")
        .body(Body::empty())
        .unwrap();

    let resp = app_header.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 1000).await.unwrap();
    assert_eq!(String::from_utf8(body.to_vec()).unwrap(), "acme-corp");

    // 2. Query param strategy
    let config_param = TenantConfig::new(TenantStrategy::Parameter);
    let app_param = axum::Router::new()
        .route("/test", get(handler))
        .layer(tenant_layer(config_param))
        .layer(axum::Extension(
            crate::security::TenantMembership::try_new(["beta-inc"]).unwrap(),
        ));

    let req_param = Request::builder()
        .uri("/test?tenant_id=beta-inc")
        .body(Body::empty())
        .unwrap();

    let resp_param = app_param.oneshot(req_param).await.unwrap();
    assert_eq!(resp_param.status(), StatusCode::OK);
    let body_param = axum::body::to_bytes(resp_param.into_body(), 1000)
        .await
        .unwrap();
    assert_eq!(String::from_utf8(body_param.to_vec()).unwrap(), "beta-inc");

    // A client cannot switch to a tenant absent from authenticated claims.
    let config_rejected = TenantConfig::new(TenantStrategy::Header);
    let rejected_app = axum::Router::new()
        .route("/test", get(handler))
        .layer(tenant_layer(config_rejected))
        .layer(axum::Extension(
            crate::security::TenantMembership::try_new(["acme-corp"]).unwrap(),
        ));
    let rejected = rejected_app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header("X-Tenant-ID", "other-tenant")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);
}

/// Inner service that accepts a call only on the instance that was
/// readied, like tower's `ConcurrencyLimit`, `RateLimit` and `Buffer`.
#[derive(Default)]
struct ReadiedOnly {
    ready: bool,
}

impl Clone for ReadiedOnly {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl tower_service::Service<axum::http::Request<axum::body::Body>> for ReadiedOnly {
    type Response = axum::http::Response<axum::body::Body>;
    type Error = &'static str;
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.ready = true;
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: axum::http::Request<axum::body::Body>) -> Self::Future {
        let readied = std::mem::take(&mut self.ready);
        std::future::ready(if readied {
            Ok(axum::http::Response::new(axum::body::Body::empty()))
        } else {
            Err("called without poll_ready")
        })
    }
}

#[tokio::test]
async fn tenant_service_calls_the_inner_service_it_readied() {
    use tower::ServiceExt;
    use tower_layer::Layer;

    let service =
        tenant_layer(TenantConfig::new(TenantStrategy::Header)).layer(ReadiedOnly::default());
    let mut request = axum::http::Request::builder()
        .uri("/")
        .header("X-Tenant-ID", "acme-corp")
        .body(axum::body::Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(crate::security::TenantMembership::try_new(["acme-corp"]).unwrap());

    let status = service
        .oneshot(request)
        .await
        .map(|response| response.status());
    assert_eq!(status, Ok(axum::http::StatusCode::OK));
}
