//! Composition of the static server's framework-owned middleware stack.

use super::builder::{Server, ServerError, development_console_enabled};
use crate::config::{Environment, SecurityConfig};
use crate::lifecycle::apply_lifecycle;
use crate::security::{TrustedProxyConfig, TrustedProxyLayer};
use std::path::{Path, PathBuf};

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
    /// (development telemetry recorder) → (monitor recorder) → trusted proxy
    /// → access log of refused requests → security baseline → lifecycle →
    /// Traffic Shield → rate limit (both skipped for exact health probes) →
    /// development/static/access log layers → `Vary: HX-Request` on HTML
    /// responses → application routes. When none of the layers between the
    /// baseline and the routes is mounted, the baseline runs the `Vary` step
    /// itself ([`InnerLayers::vary_in_baseline`]). The development reload and telemetry
    /// routes and the telemetry recorder exist only in a supervised debug
    /// Development process; the monitor route and its recorder only with
    /// [`Server::with_monitor`] and a configured credential.
    pub(super) fn into_static_app(
        self,
        security: SecurityConfig,
        environment: Environment,
    ) -> Result<axum::Router, ServerError> {
        let trusted_proxy = self.trusted_proxy_layer();
        let is_dev = environment.allows_development_tools();
        let monitor = self
            .monitor
            .map(|monitor| crate::monitor::Installed::prepare(monitor, environment))
            .transpose()
            .map_err(|error| ServerError::Configuration(error.to_string()))?
            .flatten();
        let mut app = self.router.into_axum();
        if let Some(monitor) = &monitor {
            app = monitor.mount(app);
        }
        let generation = std::env::var("RULLST_DEV_GENERATION").ok();
        let dev_reload = super::dev_reload::is_enabled(is_dev, generation.as_deref());
        app = super::dev_reload::mount(app, is_dev, generation.clone());
        // Mounted after the reload layer, which would otherwise wrap it.
        app = super::dev_telemetry::mount(app, is_dev, generation, self.dev_queue);

        // A page and the htmx fragment of the same URL are cached apart
        // (`Vary: HX-Request`); one layer also writes the access log, which
        // `RULLST_ACCESS_LOG=off` leaves out.
        let access_log = (self.access_log_recording != super::console::Recording::Off).then_some(
            super::console::AccessLog {
                log_health_probes: self.log_health_probes,
                route_paths: self.access_log_route_paths,
                recording: self.access_log_recording,
            },
        );
        let vary_in_baseline = InnerLayers {
            access_log: access_log.is_some(),
            console: development_console_enabled(cfg!(debug_assertions), environment),
            pii_masking: crate::security::masks_pii(&security, environment),
        }
        .vary_in_baseline();
        if let Some(access_log) = access_log {
            app = app.layer(axum::middleware::from_fn_with_state(
                access_log,
                super::console::access_log_and_vary_middleware,
            ));
        } else if !vary_in_baseline {
            app = app.layer(axum::middleware::from_fn(crate::htmx::vary_on_htmx_request));
        }

        let static_mounted = Path::new("static").exists();
        if static_mounted {
            app = super::static_cache::mount_static_assets(app);
        } else {
            let warning = missing_static_warning(&working_static_directory());
            super::console::stderr_line(format_args!("{warning}"));
        }

        if development_console_enabled(cfg!(debug_assertions), environment) {
            // Bound here so the read-only route stays on one line for `audit --idor`.
            let error_context = super::dev_errors::serve;
            app = app
                .route(
                    "/_rullst/explain",
                    axum::routing::get(crate::error_console::handle_explain),
                )
                .route(
                    "/_rullst/autofix",
                    axum::routing::post(crate::error_console::handle_autofix),
                )
                // rullst-access: public — development-only loopback console route; the id is a random capability shown only on the loopback error page.
                .route("/_rullst/errors/{id}", axum::routing::get(error_context))
                .layer(axum::middleware::from_fn(
                    crate::error_console::catch_panic_middleware,
                ))
                // Lets the console record panics and name their context path.
                .layer(axum::Extension(
                    crate::error_console::middleware::ErrorContextServed,
                ));
        }

        // Exact GET/HEAD `/health` and `/ready` probes (and the development
        // reload poll, when mounted) bypass both controls.
        app = super::traffic::apply_traffic_controls(app, self.limiter, self.shield, dev_reload);

        if let Some(lifecycle) = self.lifecycle {
            app = apply_lifecycle(app, lifecycle);
        }

        app = crate::security::apply_server_baseline(app, security, environment, vary_in_baseline)
            .map_err(|error| ServerError::Configuration(error.to_string()))?;
        // As `apply_security_baseline_with_machine_endpoints` does.
        if let Some(policy) = self.machine_endpoints {
            app = app.layer(axum::Extension(policy));
        }

        // The requests that the layers above refuse themselves (CSRF, the
        // WAF, Traffic Shield, the rate limiter, lifecycle admission) never
        // reach the access log inside them; this writes their lines.
        if let Some(access_log) = access_log {
            app = app.layer(axum::middleware::from_fn_with_state(
                access_log,
                super::console::refusal_log_middleware,
            ));
        }

        // Outermost: every inner layer observes the resolved client address.
        if let Some(layer) = trusted_proxy {
            app = app.layer(layer);
        }
        // Around the framework layers, so their refusals are counted too.
        if let Some(monitor) = &monitor {
            app = monitor.record(app);
        }
        // Around everything, so development telemetry also counts panics
        // answered by the console and responses of the layers above.
        if dev_reload {
            app = super::dev_telemetry::record_responses(app, static_mounted);
        }
        Ok(app)
    }
}

/// The layers of [`Server::into_static_app`] between the security baseline
/// and the application's routes that answer or change a response the
/// `Vary: HX-Request` step must not see, or must see first.
///
/// The others leave it to the baseline: lifecycle admission, Traffic Shield
/// and the rate limiter pass the routes' responses on unchanged and refuse
/// with plain text, which the step never changes; the `/static` mount,
/// merged beside the routes, marks its responses so that the step leaves
/// them alone.
#[derive(Clone, Copy, Debug, Default)]
struct InnerLayers {
    /// The access log, which runs the `Vary` step itself.
    access_log: bool,
    /// The development error console: its routes and its panic page.
    console: bool,
    /// Response PII masking, inside the baseline, which rewrites the
    /// response after the step.
    pii_masking: bool,
}

impl InnerLayers {
    /// Whether the baseline adds `Vary: HX-Request` to the routes' HTML
    /// responses itself, which saves a middleware per request: only when
    /// none of these layers is mounted, so that the step sees exactly the
    /// responses a separate layer on the routes would, in the same state.
    fn vary_in_baseline(self) -> bool {
        !(self.access_log || self.console || self.pii_masking)
    }
}

/// The `static/` directory of the working directory, which the server
/// serves under `/static`, as an absolute path when the working directory is
/// known.
fn working_static_directory() -> PathBuf {
    std::env::current_dir().map_or_else(|_| PathBuf::from("static"), |dir| dir.join("static"))
}

/// The startup warning when `directory` (the working directory's `static/`)
/// does not exist, so nothing is served under `/static`.
fn missing_static_warning(directory: &Path) -> String {
    format!(
        "⚠️  Rullst: no static directory at {}, so nothing is served under /static/. \
         Pages from rullst::htmx::render_page load /static/htmx-2.0.11.min.js (or the \
         htmx-1.9.12.min.js of an application created before 13.0.0-alpha.5) and \
         /static/rullst.css from it: start the server in the directory that contains \
         static/ (an application without pages can ignore this).",
        directory.display()
    )
}

#[cfg(test)]
#[path = "stack_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "stack_headers_tests.rs"]
mod headers_tests;

#[cfg(test)]
#[path = "monitor_stack_tests.rs"]
mod monitor_tests;

#[cfg(test)]
#[path = "refusal_log_tests.rs"]
mod refusal_log_tests;
