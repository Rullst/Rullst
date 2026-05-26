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
    pub fn new() -> Self {
        Router {
            inner: AxumRouter::new(),
        }
    }

    pub fn route<T>(self, path: &str, method_router: T) -> Self
    where
        T: Into<axum::routing::MethodRouter>,
    {
        Router {
            inner: self.inner.route(path, method_router.into()),
        }
    }

    pub fn ws<H, T>(self, path: &str, handler: H) -> Self
    where
        T: 'static,
        H: axum::handler::Handler<T, ()>,
    {
        Router {
            inner: self.inner.route(path, axum::routing::get(handler)),
        }
    }

    pub fn into_axum(self) -> AxumRouter {
        self.inner
    }

    pub fn nest(self, path: &str, router: Router) -> Self {
        Router {
            inner: self.inner.nest(path, router.inner),
        }
    }

    pub fn nest_axum(self, path: &str, router: AxumRouter) -> Self {
        Router {
            inner: self.inner.nest(path, router),
        }
    }

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

pub fn ws<H, T>(handler: H) -> axum::routing::MethodRouter
where
    T: 'static,
    H: axum::handler::Handler<T, ()>,
{
    axum::routing::get(handler)
}

#[macro_export]
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
}
