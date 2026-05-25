use crate::Router;
use crate::scheduler::Scheduler;
use rust_eloquent::Eloquent;
use std::net::SocketAddr;

#[non_exhaustive]
pub struct Server {
    router: Router,
    db_url: Option<String>,
    scheduler: Option<Scheduler>,
}

impl Server {
    pub fn new(router: Router) -> Self {
        Server {
            router,
            db_url: None,
            scheduler: None,
        }
    }

    /// Set a database URL to automatically initialize the Eloquent connection pool at startup
    pub fn with_db<S: Into<String>>(mut self, db_url: S) -> Self {
        self.db_url = Some(db_url.into());
        self
    }

    /// Attach a task scheduler that runs alongside the HTTP server.
    ///
    /// # Example
    /// ```rust,ignore
    /// use rullst::scheduler::Scheduler;
    ///
    /// let scheduler = Scheduler::new()
    ///     .task("0 0 * * *", || async { cleanup().await });
    ///
    /// Server::new(router)
    ///     .schedule(scheduler)
    ///     .run(3000)
    ///     .await?;
    /// ```
    pub fn schedule(mut self, scheduler: Scheduler) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    /// Start the HTTP server on the specified port
    pub async fn run(mut self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        if self.db_url.is_none()
            && let Ok(toml_content) = std::fs::read_to_string("Rullst.toml")
        {
            for line in toml_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("url")
                    && let Some(val) = trimmed.split('=').nth(1)
                {
                    self.db_url = Some(val.trim().trim_matches('"').to_string());
                }
            }
        }

        if let Some(db_url) = self.db_url {
            println!("Initializing Eloquent database pool...");
            Eloquent::init(&db_url).await?;
            println!("Database initialized successfully.");
        }

        // Start the scheduler if one was attached
        if let Some(scheduler) = self.scheduler.take() {
            scheduler.start();
        }

        let mut app = self.router.into_axum();

        // ─── AI-Powered Self-Healing Dev Console ─────────────────────────────
        let is_dev =
            std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()) != "production";
        if is_dev {
            // Safety: We do NOT call `set_var` at runtime (unsound in multi-threaded tokio).
            // Instead, we warn the developer to set RUST_BACKTRACE in their environment.
            if std::env::var("RUST_BACKTRACE").is_err() {
                eprintln!(
                    "⚠️  Rullst Dev: Set RUST_BACKTRACE=1 in your environment for richer error traces."
                );
            }
            app = app.route(
                "/_rullst/explain",
                axum::routing::get(crate::error_console::handle_explain),
            );
            app = app.route(
                "/_rullst/autofix",
                axum::routing::post(crate::error_console::handle_autofix),
            );
            app = app.layer(axum::middleware::from_fn(
                crate::error_console::catch_panic_middleware,
            ));
        }

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        println!("Rullst framework serving on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Router;
    use crate::scheduler::Scheduler;

    #[test]
    fn test_server_builder() {
        let router = Router::new();
        let server = Server::new(router).with_db("sqlite://test.db");

        assert_eq!(server.db_url, Some("sqlite://test.db".to_string()));
        assert!(server.scheduler.is_none());
    }

    #[test]
    fn test_server_scheduler_attach() {
        let router = Router::new();
        let scheduler = Scheduler::new();
        let server = Server::new(router).schedule(scheduler);

        assert!(server.scheduler.is_some());
    }
}
