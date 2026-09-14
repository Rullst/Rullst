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
mod assets;
pub mod cache_inspector;
pub mod data_browser;
pub use data_browser::run_studio;
pub mod distributed_traces;
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
    cache: Option<rullst_core::Cache>,
    distributed_traces: distributed_traces::DistributedTraceStore,
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
            cache: None,
            distributed_traces: distributed_traces::DistributedTraceStore::default(),
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

    /// Supplies a cache for metadata-only inspection and individual local
    /// invalidation. Cached values and bulk flush are never exposed by Studio.
    pub fn with_cache(mut self, cache: rullst_core::Cache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Supplies the bounded store shared with a separately mounted,
    /// authenticated distributed trace ingestion endpoint.
    pub fn with_distributed_traces(
        mut self,
        store: distributed_traces::DistributedTraceStore,
    ) -> Self {
        self.distributed_traces = store;
        self
    }

    /// Builds Studio behind an explicit debug-only loopback access capability.
    pub fn into_router(self, access: LocalStudioAccess) -> Result<Router, StudioBuildError> {
        let logger_state = Arc::new(logger::LoggerState::new());
        let cache_router = cache_inspector::router(self.cache)?;
        let mut router = data_browser::router_with_trace_store(self.distributed_traces)
            .nest("/studio/requests", logger::router(logger_state.clone()))
            .nest("/studio/env", env_viewer::router())
            .nest("/studio/features", feature_flags::router())
            .nest("/studio/cache", cache_router)
            .nest("/studio/assets", assets::router())
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
    #[cfg(debug_assertions)]
    use axum::{body::Body, http::Request};
    #[cfg(debug_assertions)]
    use tower::ServiceExt;

    #[cfg(debug_assertions)]
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

    #[cfg(debug_assertions)]
    #[tokio::test]
    async fn configured_cache_and_trace_routes_render_real_bounded_state() {
        let cache = rullst_core::Cache::memory();
        cache
            .put("private:learner:7", "sensitive-value", Some(60))
            .await
            .expect("cache fixture");
        let trace_store =
            distributed_traces::DistributedTraceStore::new(8).expect("bounded trace store");
        trace_store
            .insert_batch(
                "lms-api",
                1_800_000_000,
                vec![distributed_traces::DistributedTraceSpanV1 {
                    trace_id: "0123456789abcdef0123456789abcdef".to_string(),
                    span_id: "0123456789abcdef".to_string(),
                    parent_span_id: None,
                    operation: "lessons.list".to_string(),
                    kind: distributed_traces::DistributedTraceKind::Sql,
                    started_at_unix_us: 1_800_000_000_000_000,
                    duration_us: 700,
                    status: distributed_traces::DistributedTraceStatus::Ok,
                }],
            )
            .expect("trace fixture");
        let router = Studio::new()
            .with_cache(cache)
            .with_distributed_traces(trace_store)
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

        let cache_response = router
            .clone()
            .oneshot(request("/studio/cache"))
            .await
            .expect("cache response");
        assert_eq!(cache_response.status(), axum::http::StatusCode::OK);
        let cache_body = axum::body::to_bytes(cache_response.into_body(), 512 * 1024)
            .await
            .expect("cache body");
        let cache_body = String::from_utf8(cache_body.to_vec()).expect("UTF-8 cache body");
        assert!(cache_body.contains("Cache Inspector"));
        assert!(cache_body.contains("15 bytes"));
        assert!(!cache_body.contains("private:learner:7"));
        assert!(!cache_body.contains("sensitive-value"));

        let trace_response = router
            .oneshot(request("/studio/traces"))
            .await
            .expect("trace response");
        assert_eq!(trace_response.status(), axum::http::StatusCode::OK);
        let trace_body = axum::body::to_bytes(trace_response.into_body(), 512 * 1024)
            .await
            .expect("trace body");
        let trace_body = String::from_utf8(trace_body.to_vec()).expect("UTF-8 trace body");
        assert!(trace_body.contains("Trace Inspector"));
        assert!(trace_body.contains("lms-api"));
        assert!(trace_body.contains("lessons.list"));
        assert!(trace_body.contains("1 retained"));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn studio_builder_fails_closed_in_release() {
        assert!(matches!(
            Studio::new().into_router(LocalStudioAccess::loopback_only()),
            Err(StudioBuildError::LocalAccessRequiresDebugBuild)
        ));
        assert!(matches!(
            Studio::default()
                .with_openapi(OpenApi::default())
                .into_router(LocalStudioAccess::loopback_only()),
            Err(StudioBuildError::LocalAccessRequiresDebugBuild)
        ));
    }
}
