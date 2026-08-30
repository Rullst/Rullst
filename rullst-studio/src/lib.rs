#![cfg_attr(mutants, mutants::skip)]
extern crate rullst_core as rullst;
use axum::Router;
use rullst_core::Queue;
use std::sync::Arc;
use utoipa::openapi::OpenApi;

pub mod access;
pub mod ai_playground;
pub mod api_playground;
pub use access::{LocalStudioAccess, StudioBuildError};
pub mod data_browser;
pub use data_browser::run_studio;
pub mod env_viewer;
pub mod er_diagram;
pub mod feature_flags;
pub mod jobs_monitor;
pub mod logger;
pub mod migration_manager;
pub mod radar_visualizer;
pub mod revenue_dashboard;
pub mod security_radar;
pub mod traces_visualizer;

pub struct Studio {
    openapi: Option<OpenApi>,
    queue: Option<Queue>,
}

impl Default for Studio {
    fn default() -> Self {
        Self::new()
    }
}

impl Studio {
    pub fn new() -> Self {
        Self {
            openapi: None,
            queue: None,
        }
    }

    pub fn with_openapi(mut self, openapi: OpenApi) -> Self {
        self.openapi = Some(openapi);
        self
    }

    pub fn with_horizon(mut self, queue: Queue) -> Self {
        self.queue = Some(queue);
        self
    }

    /// Builds Studio behind an explicit debug-only loopback access capability.
    pub fn into_router(self, access: LocalStudioAccess) -> Result<Router, StudioBuildError> {
        let logger_state = Arc::new(logger::LoggerState::new());
        let mut router = data_browser::router()
            .nest("/studio/requests", logger::router(logger_state.clone()))
            .nest("/studio/env", env_viewer::router())
            .nest("/studio/features", feature_flags::router())
            .nest("/studio/er", er_diagram::router())
            .merge(security_radar::stats_router());

        if let Some(openapi) = self.openapi {
            router = router.merge(api_playground::router(openapi));
        }

        if let Some(queue) = self.queue {
            router = router.nest("/studio/jobs", jobs_monitor::router(queue));
        }

        router = router.layer(axum::middleware::from_fn_with_state(
            logger_state,
            logger::logger_middleware,
        ));
        access.protect_router(router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_studio_builder_and_routes() {
        let studio = Studio::new();
        let router = studio
            .into_router(LocalStudioAccess::loopback_only())
            .expect("debug Studio router");
        let request = |uri: &'static str| {
            let mut request = Request::builder()
                .uri(uri)
                .header(axum::http::header::HOST, "127.0.0.1:5555")
                .body(Body::empty())
                .expect("valid request");
            request.extensions_mut().insert(axum::extract::ConnectInfo(
                "127.0.0.1:42000"
                    .parse::<std::net::SocketAddr>()
                    .expect("loopback peer"),
            ));
            request
        };
        let security_page = router
            .clone()
            .oneshot(request("/studio/security"))
            .await
            .expect("security page response");
        assert_eq!(security_page.status(), axum::http::StatusCode::OK);

        let security_stats = router
            .oneshot(request("/studio/security/stats"))
            .await
            .expect("security stats response");
        assert_eq!(security_stats.status(), axum::http::StatusCode::OK);

        let queue = Queue::sqlite("sqlite::memory:").await.unwrap();
        let openapi = OpenApi::default();

        let full_studio = Studio::default().with_openapi(openapi).with_horizon(queue);
        let _ = full_studio
            .into_router(LocalStudioAccess::loopback_only())
            .expect("debug full Studio router");
    }
}
