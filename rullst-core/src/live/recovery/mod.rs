//! Explicit authorization and complete-state recovery for server-driven Live UI.
mod config;
mod session;
#[cfg(test)]
mod tests;
mod types;

use axum::{
    extract::ws::WebSocketUpgrade,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
pub use config::LiveRecoveryConfig;
use std::sync::Arc;
use tokio::sync::Semaphore;
pub use types::{
    LiveCommand, LiveRecoveryError, LiveResult, LiveScope, LiveSnapshot, RecoverableLiveView,
};

/// Same-origin browser ES module. Serve as JavaScript through an explicit route;
/// load it with the application's CSP policy. It contains no third-party assets.
pub const LIVE_RECOVERY_MODULE: &str = include_str!("client.js");

/// Shared bounded upgrade handler; clones share their connection quota.
#[derive(Clone)]
pub struct LiveRecovery {
    config: LiveRecoveryConfig,
    admission: Arc<Semaphore>,
}

impl LiveRecovery {
    /// Creates the explicit handler. This does not mount any HTTP routes.
    pub fn new(config: LiveRecoveryConfig) -> Self {
        Self {
            admission: Arc::new(Semaphore::new(config.max_connections)),
            config,
        }
    }

    /// Checks exact origin, v1 subprotocol and current application authorization
    /// before upgrading. The route must resolve scope/session from trusted state
    /// and use the ordinary HTTP security baseline. The scope is never taken
    /// from incoming WebSocket commands.
    pub async fn upgrade<C: RecoverableLiveView>(
        &self,
        ws: WebSocketUpgrade,
        headers: &HeaderMap,
        scope: LiveScope,
        component: C,
    ) -> Response {
        if !self.config.permits(headers) {
            return StatusCode::FORBIDDEN.into_response();
        }
        let Ok(permit) = self.admission.clone().try_acquire_owned() else {
            return StatusCode::TOO_MANY_REQUESTS.into_response();
        };
        let authorized =
            tokio::time::timeout(self.config.operation_timeout, component.authorize(&scope)).await;
        match authorized {
            Ok(Ok(())) => (),
            Ok(Err(LiveRecoveryError::Unauthorized)) => {
                return StatusCode::UNAUTHORIZED.into_response();
            }
            _ => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
        let config = self.config.clone();
        ws.protocols(["rullst.live.v1"])
            .read_buffer_size(16384)
            .max_frame_size(16384)
            .max_message_size(16384)
            .write_buffer_size(16384)
            .max_write_buffer_size(1024 * 1024)
            .on_upgrade(move |socket| async move {
                let _permit = permit;
                session::run(socket, config, scope, component).await;
            })
    }
}
