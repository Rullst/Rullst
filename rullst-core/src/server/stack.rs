//! Composition of the static server's framework-owned middleware stack.

use super::builder::{Server, ServerError, development_console_enabled};
use crate::config::{Environment, SecurityConfig};
use crate::lifecycle::apply_lifecycle;
use crate::security::{TrustedProxyConfig, TrustedProxyLayer};
use crate::server::server_middleware::zstd_static_middleware;

impl Server {
    /// Selects the effective trusted-proxy policy. An explicit
    /// [`Server::trusted_proxies`] policy replaces the `[security]` settings of
    /// `Rullst.toml`; otherwise an enabled TOML policy is used.
    pub(super) fn resolve_trusted_proxy(
        &mut self,
        security: &SecurityConfig,
    ) -> Result<(), ServerError> {
        if self.trusted_proxy.is_none() {
            let config = TrustedProxyConfig::from_security_config(security)
                .map_err(|error| ServerError::Configuration(error.to_string()))?;
            self.trusted_proxy = Some(config);
        }
        Ok(())
    }

    /// Enabled trusted-proxy layer, if any.
    pub(super) fn trusted_proxy_layer(&self) -> Option<TrustedProxyLayer> {
        self.trusted_proxy
            .clone()
            .filter(TrustedProxyConfig::is_enabled)
            .map(TrustedProxyLayer::new)
    }

    /// Builds the static application. Outer-to-inner request order:
    /// trusted proxy → security baseline → lifecycle → Traffic Shield → rate
    /// limit (both skipped for exact health probes) → development/static/access
    /// log layers → application routes.
    pub(super) fn into_static_app(
        self,
        security: SecurityConfig,
        environment: Environment,
    ) -> Result<axum::Router, ServerError> {
        let trusted_proxy = self.trusted_proxy_layer();
        let is_dev = environment.allows_development_tools();
        let mut app = self.router.into_axum();
        let generation = std::env::var("RULLST_DEV_GENERATION").ok();
        let dev_reload = super::dev_reload::is_enabled(is_dev, generation.as_deref());
        app = super::dev_reload::mount(app, is_dev, generation);

        app = app.layer(axum::middleware::from_fn(
            super::console::access_log_middleware,
        ));

        if std::path::Path::new("static").exists() {
            app = app
                .nest_service(
                    "/static",
                    tower_http::services::ServeDir::new("static").precompressed_br(),
                )
                .layer(axum::middleware::from_fn(zstd_static_middleware));
        }

        if development_console_enabled(cfg!(debug_assertions), environment) {
            app = app
                .route(
                    "/_rullst/explain",
                    axum::routing::get(crate::error_console::handle_explain),
                )
                .route(
                    "/_rullst/autofix",
                    axum::routing::post(crate::error_console::handle_autofix),
                )
                .layer(axum::middleware::from_fn(
                    crate::error_console::catch_panic_middleware,
                ));
        }

        // Exact GET/HEAD `/health` and `/ready` probes (and the development
        // reload poll, when mounted) bypass both controls.
        app = super::traffic::apply_traffic_controls(app, self.limiter, self.shield, dev_reload);

        if let Some(lifecycle) = self.lifecycle {
            app = apply_lifecycle(app, lifecycle);
        }

        app = if let Some(policy) = self.machine_endpoints {
            crate::security::apply_security_baseline_with_machine_endpoints(
                app,
                security,
                environment,
                policy,
            )
        } else {
            crate::security::apply_security_baseline(app, security, environment)
        }
        .map_err(|error| ServerError::Configuration(error.to_string()))?;

        // Outermost: every inner layer observes the resolved client address.
        if let Some(layer) = trusted_proxy {
            app = app.layer(layer);
        }
        Ok(app)
    }
}

#[cfg(test)]
#[path = "stack_tests.rs"]
mod tests;
