use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use futures_util::future::BoxFuture;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// A trait that user models can implement to integrate with Role-Based Access Control (RBAC).
pub trait HasRole {
    /// Checks if the user has the specified role.
    fn has_role(&self, role: &str) -> bool;
}

/// An Axum middleware layer that enforces a specific role for a route.
/// It expects the current user of type `U` to be present in the request extensions
/// (usually placed there by an authentication middleware) and checks if the user
/// implements `HasRole` and possesses the required role.
#[derive(Clone)]
pub struct RequireRoleLayer<U> {
    role: &'static str,
    _marker: std::marker::PhantomData<fn() -> U>,
}

impl<U> RequireRoleLayer<U> {
    /// Creates a new `RequireRoleLayer` that will require the given role.
    pub fn new(role: &'static str) -> Self {
        Self {
            role,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, U> Layer<S> for RequireRoleLayer<U> {
    type Service = RequireRoleService<S, U>;

    fn layer(&self, inner: S) -> RequireRoleService<S, U> {
        RequireRoleService {
            inner,
            role: self.role,
            _marker: std::marker::PhantomData,
        }
    }
}

/// The inner service for `RequireRoleLayer`.
#[derive(Clone)]
pub struct RequireRoleService<S, U> {
    inner: S,
    role: &'static str,
    _marker: std::marker::PhantomData<fn() -> U>,
}

impl<S, U> Service<Request<Body>> for RequireRoleService<S, U>
where
    S: Service<Request<Body>, Response = Response> + Send + Clone + 'static,
    S::Future: Send + 'static,
    U: HasRole + Clone + Send + Sync + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> BoxFuture<'static, Result<S::Response, S::Error>> {
        let role = self.role;
        let user = req.extensions().get::<U>();

        if let Some(user) = user {
            if !user.has_role(role) {
                let response = axum::response::IntoResponse::into_response((
                    StatusCode::FORBIDDEN,
                    "Forbidden: Insufficient privileges",
                ));
                return Box::pin(async move { Ok(response) });
            }
        } else {
            let response = axum::response::IntoResponse::into_response((
                StatusCode::UNAUTHORIZED,
                "Unauthorized: Authentication required",
            ));
            return Box::pin(async move { Ok(response) });
        }

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move { inner.call(req).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tower::{ServiceBuilder, ServiceExt};

    #[derive(Clone)]
    struct DummyUser {
        role: String,
    }

    impl HasRole for DummyUser {
        fn has_role(&self, role: &str) -> bool {
            self.role == role
        }
    }

    async fn dummy_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_require_role_authorized() {
        let router = axum::Router::new()
            .route("/", axum::routing::get(dummy_handler))
            .route_layer(RequireRoleLayer::<DummyUser>::new("Admin"));

        let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();
        req.extensions_mut().insert(DummyUser {
            role: "Admin".to_string(),
        });

        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_require_role_forbidden() {
        let router = axum::Router::new()
            .route("/", axum::routing::get(dummy_handler))
            .route_layer(RequireRoleLayer::<DummyUser>::new("Admin"));

        let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();
        req.extensions_mut().insert(DummyUser {
            role: "User".to_string(),
        });

        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_require_role_unauthorized() {
        let router = axum::Router::new()
            .route("/", axum::routing::get(dummy_handler))
            .route_layer(RequireRoleLayer::<DummyUser>::new("Admin"));

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();

        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
