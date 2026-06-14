use axum::Router as AxumRouter;

/// A wrapper around Axum's Router to provide clean integration with Rullst features
#[non_exhaustive]
pub struct Router {
    inner: AxumRouter,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    /// Creates a new empty `Router` with no routes configured.
    pub fn new() -> Self {
        Router {
            inner: AxumRouter::new(),
        }
    }

    /// Registers an HTTP route at the given `path` with the given method router.
    /// `method_router` is typically the result of `get(handler)`, `post(handler)`, etc.
    pub fn route<T>(self, path: &str, method_router: T) -> Self
    where
        T: Into<axum::routing::MethodRouter>,
    {
        Router {
            inner: self.inner.route(path, method_router.into()),
        }
    }

    /// Registers a WebSocket upgrade route at the given `path`.
    /// Internally maps to `GET path` since WebSocket upgrades happen over HTTP GET.
    pub fn ws<H, T>(self, path: &str, handler: H) -> Self
    where
        T: 'static,
        H: axum::handler::Handler<T, ()>,
    {
        Router {
            inner: self.inner.route(path, axum::routing::get(handler)),
        }
    }

    /// Unwraps the inner [`axum::Router`]. Use this to hand the router off to `axum::serve`
    /// or to interoperate with Tower middleware that requires a raw Axum router.
    pub fn into_axum(self) -> AxumRouter {
        self.inner
    }

    /// Nests another Rullst `Router` under a path prefix.
    /// All routes defined in `router` will be reachable under `path/...`.
    pub fn nest(self, path: &str, router: Router) -> Self {
        Router {
            inner: self.inner.nest(path, router.inner),
        }
    }

    /// Nests a raw [`axum::Router`] under a path prefix.
    /// Useful for integrating third-party Axum routers (e.g. from `utoipa`, `aide`) without wrapping.
    pub fn nest_axum(self, path: &str, router: AxumRouter) -> Self {
        Router {
            inner: self.inner.nest(path, router),
        }
    }

    /// Merges a raw [`axum::Router`] into this router at the root.
    ///
    /// Equivalent to `Router::merge` on the inner `axum::Router`. This is the
    /// recommended way to compose a fully-built Axum router (for example, one
    /// produced by `utoipa_axum::OpenApiRouter::split_for_parts`) into a Rullst
    /// `Router`, since axum 0.8+ no longer allows nesting at the root path.
    pub fn merge_axum(self, router: AxumRouter) -> Self {
        Router {
            inner: self.inner.merge(router),
        }
    }

    /// Applies a Tower middleware layer to all routes in this router.
    /// The layer is cloned once per request and must be `Send + Sync + 'static`.
    pub fn layer<L>(self, layer: L) -> Self
    where
        L: tower_layer::Layer<axum::routing::Route> + Clone + Send + Sync + 'static,
        L::Service: tower_service::Service<
                axum::extract::Request,
                Response = axum::response::Response,
                Error = std::convert::Infallible,
            > + Clone
            + Send
            + Sync
            + 'static,
        <L::Service as tower_service::Service<axum::extract::Request>>::Future: Send + 'static,
    {
        Router {
            inner: self.inner.layer(layer),
        }
    }
}

pub use axum::routing::delete;
pub use axum::routing::get;
pub use axum::routing::patch;
pub use axum::routing::post;
pub use axum::routing::put;

/// Returns an Axum `MethodRouter` that upgrades `GET` requests to a WebSocket connection.
/// Used inside `routes!` as `ws("/chat" => handler)` or directly on a `Router::ws()` call.
pub fn ws<H, T>(handler: H) -> axum::routing::MethodRouter
where
    T: 'static,
    H: axum::handler::Handler<T, ()>,
{
    axum::routing::get(handler)
}

#[macro_export]
/// Declarative macro for building a [`Router`] from a list of HTTP route definitions.
///
/// # Example
/// ```rust,no_run
/// use rullst::{routes, routing::{get, post}};
///
/// async fn home_handler() -> &'static str { "home" }
/// async fn create_user() -> &'static str { "created" }
///
/// let router = routes![
///     get("/" => home_handler),
///     post("/users" => create_user),
/// ];
/// ```
macro_rules! routes {
    ( $($method:ident ( $path:expr => $handler:expr )),* $(,)? ) => {
        {
            let mut router = $crate::Router::new();
            $(
                router = router.route($path, $crate::routing::$method($handler));
            )*
            router
        }
    };
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn mock_handler() -> &'static str {
        "hello"
    }

    #[tokio::test]
    async fn test_router_routes() {
        let router = Router::new()
            .route("/", get(mock_handler))
            .route("/post", post(mock_handler));

        let app = router.into_axum();

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_routes_macro() {
        let router = routes![
            get("/" => mock_handler),
            post("/submit" => mock_handler),
        ];

        let app = router.into_axum();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/submit")
                    .method("POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_router_nesting() {
        let api_router = Router::new().route("/users", get(mock_handler));
        let router = Router::new().nest("/api", api_router);
        let app = router.into_axum();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_nest_axum() {
        let axum_router = AxumRouter::new().route("/raw", axum::routing::get(mock_handler));
        let router = Router::new().nest_axum("/api", axum_router);
        let app = router.into_axum();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/raw")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ws_routing() {
        let router = Router::new().ws("/chat", mock_handler);
        let app = router.into_axum();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/chat")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_merge_axum() {
        let axum_router = AxumRouter::new().route("/merged", axum::routing::get(mock_handler));
        let router = Router::new().merge_axum(axum_router);
        let app = router.into_axum();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/merged")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
