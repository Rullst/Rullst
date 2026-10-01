//! Startup and serving steps of [`Server`]: configuration loading, database,
//! scheduler and Traffic Shield start-up, listen-address resolution, and the
//! static or development hot-reload HTTP loop.

use super::environment::{
    development_console_enabled, listen_address, read_optional_environment_variable,
    resolve_environment, resolve_hot_reload_token,
};
use super::shutdown::{mark_lifecycle_ready, mark_lifecycle_stopped, shutdown_with_lifecycle};
use super::{Server, ServerError};
use crate::scheduler::{Scheduler, SchedulerHandle};
use crate::server::dylib_loader::load_dylib_router;
use crate::server::hotswap::{HotSwapService, PeerAwareHotSwap};
#[cfg(feature = "orm")]
use rullst_orm::Orm;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};

impl Server {
    pub(super) async fn run_inner<F>(mut self, port: u16, shutdown: F) -> Result<(), ServerError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        // Uptime counts from process start-up, not from the first probe.
        crate::health::init_health_boot_time_if_unset();
        crate::radar::init_radar_if_unset();
        let dotenv = Self::load_dotenv_values().await?;
        #[cfg(feature = "orm")]
        crate::artisan::runner::intercept_artisan_command(None, self.db_url.as_deref()).await;
        let _ = crate::telemetry::init_telemetry();
        let app_config = Self::load_config().await?;
        let environment = resolve_environment(&app_config, &dotenv)?;
        // `RullstConfig::environment` reports the same environment.
        crate::config::record_project_environment_selector(
            dotenv
                .get("RULLST_ENV")
                .or_else(|| dotenv.get("APP_ENV"))
                .cloned(),
        );
        self.resolve_trusted_proxy(&app_config.security)?;

        self.init_database(&app_config, &dotenv).await?;
        let addr = Self::setup_networking(port, app_config.app.port, environment, &dotenv)?;
        let mut scheduler_handle = self.start_scheduler()?;
        let shield_lifecycle = self.start_traffic_shield()?;

        let server = async move {
            if let Some(lib_path) = self.hot_reload_lib.take() {
                self.run_hot_reload(lib_path, addr, environment, shutdown)
                    .await
            } else {
                self.run_static(app_config, addr, environment, shutdown)
                    .await
            }
        };
        // The server owns the only scheduler handle: log task failures as
        // they happen and keep them out of the clean-shutdown result.
        let server_result = crate::server::scheduler_supervision::serve_while_draining(
            server,
            scheduler_handle.as_mut(),
        )
        .await;

        if let Some(shield) = shield_lifecycle {
            shield.shutdown();
        }
        let scheduler_result =
            crate::server::scheduler_supervision::stop_scheduler(scheduler_handle).await;

        match server_result {
            Err(error) => Err(error),
            Ok(()) => scheduler_result,
        }
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn load_dotenv_values() -> Result<HashMap<String, String>, ServerError> {
        crate::server::database_url::load_dotenv_file(std::path::Path::new(".env")).await
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn load_config() -> Result<crate::config::RullstConfig, ServerError> {
        let app_config =
            crate::server::database_url::load_config_file(std::path::Path::new("Rullst.toml"))
                .await?;

        app_config
            .validate()
            .map_err(|error| ServerError::Configuration(error.to_string()))?;
        let _ = crate::config::RullstConfig::set_global(app_config.clone());
        Ok(app_config)
    }

    #[cfg(feature = "orm")]
    #[cfg_attr(mutants, mutants::skip)]
    pub(super) async fn init_database(
        &mut self,
        app_config: &crate::config::RullstConfig,
        dotenv: &HashMap<String, String>,
    ) -> Result<(), ServerError> {
        if rullst_orm::Orm::try_pool().is_ok() {
            return Ok(());
        }

        if self.db_url.is_none() {
            self.db_url = crate::server::database_url::resolve_database_url(
                None,
                read_optional_environment_variable,
                dotenv,
                app_config,
            )?;
        }

        if let Some(db_url) = &self.db_url {
            println!("Initializing Orm database pool...");
            Orm::init(db_url)
                .await
                .map_err(|error| ServerError::Database(error.to_string()))?;
            println!("Database initialized successfully.");
        }

        Ok(())
    }

    #[cfg(not(feature = "orm"))]
    #[cfg_attr(mutants, mutants::skip)]
    pub(super) async fn init_database(
        &mut self,
        app_config: &crate::config::RullstConfig,
        dotenv: &HashMap<String, String>,
    ) -> Result<(), ServerError> {
        let database_requested = crate::server::database_url::resolve_database_url(
            self.db_url.as_deref(),
            read_optional_environment_variable,
            dotenv,
            app_config,
        )?
        .is_some();

        if database_requested {
            return Err(ServerError::Database(
                "rullst-core was compiled without the `orm` feature".to_string(),
            ));
        }

        Ok(())
    }

    #[cfg_attr(mutants, mutants::skip)]
    pub(super) fn start_scheduler(&mut self) -> Result<Option<SchedulerHandle>, ServerError> {
        self.scheduler
            .take()
            .map(Scheduler::start)
            .transpose()
            .map_err(ServerError::from)
    }

    #[cfg_attr(mutants, mutants::skip)]
    pub(super) fn start_traffic_shield(
        &self,
    ) -> Result<Option<crate::resilience::TrafficShield>, ServerError> {
        let Some(shield) = self.shield.clone() else {
            return Ok(None);
        };
        shield.start().map_err(ServerError::from)?;
        Ok(Some(shield))
    }

    #[cfg_attr(mutants, mutants::skip)]
    pub(super) fn setup_networking(
        fallback_port: u16,
        configured_port: Option<u16>,
        environment: crate::config::Environment,
        dotenv: &HashMap<String, String>,
    ) -> Result<SocketAddr, ServerError> {
        let host_str = read_optional_environment_variable("HOST")?
            .or(read_optional_environment_variable("RULLST_HOST")?)
            .or_else(|| dotenv.get("HOST").cloned())
            .or_else(|| dotenv.get("RULLST_HOST").cloned())
            .unwrap_or_else(|| {
                if environment.requires_secure_defaults() {
                    "0.0.0.0".to_string()
                } else {
                    "127.0.0.1".to_string()
                }
            });

        let env_port_value =
            read_optional_environment_variable("PORT")?.or_else(|| dotenv.get("PORT").cloned());
        let env_port = match env_port_value {
            Some(value) => Some(value.parse::<u16>().map_err(|_| {
                ServerError::Configuration(format!("PORT must be a valid u16, got `{value}`"))
            })?),
            None => None,
        };
        let port = env_port.or(configured_port).unwrap_or(fallback_port);

        let addr = listen_address(&host_str, port).ok_or(ServerError::InvalidAddress {
            host: host_str,
            port,
        })?;

        if development_console_enabled(cfg!(debug_assertions), environment)
            && addr.ip().is_unspecified()
        {
            eprintln!(
                "⚠️  Rullst Dev: Self-Healing Console mounted on /_rullst/*\n\
                   Set RULLST_ENV=production to disable before deploying."
            );
        }

        Ok(addr)
    }

    #[cfg_attr(mutants, mutants::skip)]
    pub(super) async fn run_hot_reload<F>(
        self,
        lib_path: String,
        addr: SocketAddr,
        environment: crate::config::Environment,
        shutdown: F,
    ) -> Result<(), ServerError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        if !cfg!(debug_assertions) || !environment.allows_development_tools() {
            return Err(ServerError::HotReloadDisabled);
        }

        let is_dev = true;
        let reload_token = resolve_hot_reload_token()?;

        println!(
            "\x1b[36mRullst: initializing the authenticated development-library reload boundary...\x1b[0m"
        );

        let (initial_router, library) = match load_dylib_router(&lib_path, is_dev) {
            Ok(r) => r,
            Err(e) => {
                println!(
                    "\x1b[31m❌ Failed to load initial dylib: {}. Make sure the dynamic library was compiled by running 'cargo build --lib'.\x1b[0m",
                    e
                );
                return Err(ServerError::HotReload(e.to_string()));
            }
        };

        let current_router = Arc::new(RwLock::new(initial_router));
        let active_libraries = Arc::new(Mutex::new(vec![library]));
        let (hmr_sender, _receiver) = tokio::sync::broadcast::channel(32);

        let hotswap_service = HotSwapService {
            current_router: current_router.clone(),
            active_libraries: active_libraries.clone(),
            hmr_sender,
            reload_lock: Arc::new(tokio::sync::Mutex::new(())),
            reload_token,
            lib_path: lib_path.clone(),
            is_dev,
            shield: self.shield.clone(),
            limiter: self.limiter.clone(),
            lifecycle: self.lifecycle.clone(),
            trusted_proxy: self.trusted_proxy_layer(),
            machine_endpoints: self.machine_endpoints.clone(),
        };

        println!(
            "Rullst framework serving on http://{} (authenticated development hot reload)",
            addr
        );
        println!(
            "🚀 Visit: http://localhost:{} to see the result!",
            addr.port()
        );

        let listener = tokio::net::TcpListener::bind(addr).await?;
        mark_lifecycle_ready(self.lifecycle.as_ref())?;
        let lifecycle = self.lifecycle.clone();
        let result = axum::serve(listener, PeerAwareHotSwap(hotswap_service))
            .with_graceful_shutdown(shutdown_with_lifecycle(shutdown, lifecycle.clone()))
            .await
            .map_err(ServerError::from);
        mark_lifecycle_stopped(lifecycle.as_ref());
        result
    }

    #[cfg_attr(mutants, mutants::skip)]
    async fn run_static<F>(
        self,
        app_config: crate::config::RullstConfig,
        addr: SocketAddr,
        environment: crate::config::Environment,
        shutdown: F,
    ) -> Result<(), ServerError>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let lifecycle = self.lifecycle.clone();
        let app = self.into_static_app(app_config.security, environment)?;

        println!("Rullst framework serving on http://{}", addr);
        println!(
            "🚀 Visit: http://localhost:{} to see the result!",
            addr.port()
        );

        let listener = tokio::net::TcpListener::bind(addr).await?;
        mark_lifecycle_ready(lifecycle.as_ref())?;
        let result = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_with_lifecycle(shutdown, lifecycle.clone()))
        .await
        .map_err(ServerError::from);
        mark_lifecycle_stopped(lifecycle.as_ref());
        result
    }
}
