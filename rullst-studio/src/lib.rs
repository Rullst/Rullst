#![cfg_attr(mutants, mutants::skip)]
extern crate rullst_core as rullst;
use axum::Router;
use rullst_core::Queue;
use std::sync::Arc;
use utoipa::openapi::OpenApi;

pub mod ai_playground;
pub mod api_playground;
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

    pub fn into_router(self) -> Router {
        let logger_state = Arc::new(logger::LoggerState::new());
        let mut router = data_browser::router()
            .nest("/requests", logger::router(logger_state.clone()))
            .layer(axum::middleware::from_fn_with_state(
                logger_state,
                logger::logger_middleware,
            ))
            .nest("/env", env_viewer::router())
            .nest("/features", feature_flags::router())
            .nest("/er", er_diagram::router())
            .merge(security_radar::stats_router());

        if let Some(openapi) = self.openapi {
            router = router.nest("/api", api_playground::router(openapi));
        }

        if let Some(queue) = self.queue {
            router = router.nest("/jobs", jobs_monitor::router(queue));
        }

        router
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
        let router = studio.into_router();
        let security_page = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/studio/security")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("security page response");
        assert_eq!(security_page.status(), axum::http::StatusCode::OK);

        let security_stats = router
            .oneshot(
                Request::builder()
                    .uri("/studio/security/stats")
                    .body(Body::empty())
                    .expect("valid request"),
            )
            .await
            .expect("security stats response");
        assert_eq!(security_stats.status(), axum::http::StatusCode::OK);

        let queue = Queue::sqlite("sqlite::memory:").await.unwrap();
        let openapi = OpenApi::default();

        let full_studio = Studio::default().with_openapi(openapi).with_horizon(queue);
        let _ = full_studio.into_router();
    }
}
