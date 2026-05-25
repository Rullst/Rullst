use crate::Router;
use std::net::SocketAddr;
use rust_eloquent::Eloquent;

pub struct Server {
    router: Router,
    db_url: Option<String>,
}

impl Server {
    pub fn new(router: Router) -> Self {
        Server {
            router,
            db_url: None,
        }
    }

    /// Set a database URL to automatically initialize the Eloquent connection pool at startup
    pub fn with_db<S: Into<String>>(mut self, db_url: S) -> Self {
        self.db_url = Some(db_url.into());
        self
    }

    /// Start the HTTP server on the specified port
    pub async fn run(mut self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        if self.db_url.is_none() {
            if let Ok(toml_content) = std::fs::read_to_string("Rullst.toml") {
                for line in toml_content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("url") {
                        if let Some(val) = trimmed.split('=').nth(1) {
                            self.db_url = Some(val.trim().trim_matches('"').to_string());
                        }
                    }
                }
            }
        }

        if let Some(db_url) = self.db_url {
            println!("Initializing Eloquent database pool...");
            Eloquent::init(&db_url).await?;
            println!("Database initialized successfully.");
        }

        let app = self.router.into_axum();
        
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        println!("Rullst framework serving on http://{}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}
