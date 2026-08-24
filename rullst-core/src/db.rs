//! Rullst Database Extensions (`rullst::db`)
//!
//! Provides configuration types for future distributed SQLite replication.
//! Remote synchronization is explicitly unsupported in this release and fails
//! closed instead of simulating a successful sync.

/// Configuration for distributed SQLite replica database sync.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct ReplicationConfig {
    /// Local SQLite file path representing the local database replica.
    pub replica_path: String,
    /// Remote SQLite database sync master URL.
    pub sync_url: Option<String>,
    /// Security authentication bearer token for connection validation.
    pub auth_token: Option<String>,
    /// Synchronization check interval in seconds (default is 10 seconds).
    pub sync_interval_secs: u64,
}

impl ReplicationConfig {
    /// Creates a new `ReplicationConfig` using the constructor and builder pattern.
    pub fn new(replica_path: impl Into<String>) -> Self {
        Self {
            replica_path: replica_path.into(),
            sync_url: None,
            auth_token: None,
            sync_interval_secs: 10,
        }
    }

    /// Attaches the remote sync master URL (e.g. "libsql://my-db.turso.io").
    pub fn with_sync_url(mut self, sync_url: impl Into<String>) -> Self {
        self.sync_url = Some(sync_url.into());
        self
    }

    /// Sets the remote connection authentication token.
    pub fn with_auth_token(mut self, auth_token: impl Into<String>) -> Self {
        self.auth_token = Some(auth_token.into());
        self
    }

    /// Sets the interval duration between sync queries in seconds.
    pub fn with_sync_interval(mut self, secs: u64) -> Self {
        self.sync_interval_secs = secs;
        self
    }
}

/// Fail-closed facade reserved for a future SQLite replication backend.
pub struct ReplicationManager;

/// Failures reported by the replication facade.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReplicationError {
    /// Remote replication has no production backend in this release.
    #[error("SQLite replication backend is not implemented for `{sync_url}`")]
    Unsupported {
        /// Requested remote replication endpoint.
        sync_url: String,
    },
}

impl ReplicationManager {
    /// Validates replication configuration.
    ///
    /// Remote synchronization is deliberately fail-closed until a real libSQL/D1
    /// backend exists; this method never reports a simulated synchronization as
    /// successful.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn start(config: ReplicationConfig) -> Result<(), ReplicationError> {
        if let Some(sync_url) = config.sync_url {
            return Err(ReplicationError::Unsupported { sync_url });
        }
        Ok(())
    }
}

// ─── Dependency Shielding cascades (Roadmap Milestone 8) ────────────────────
#[cfg(all(not(target_arch = "wasm32"), feature = "orm"))]
pub use rullst_orm::{Orm, RullstModel, RullstPool, async_trait, schema};
#[cfg(all(not(target_arch = "wasm32"), feature = "orm"))]
pub use sqlx;
#[cfg(all(not(target_arch = "wasm32"), feature = "orm"))]
pub use sqlx::FromRow;

/// Safely retrieves the database pool, returning `None` if uninitialized.
#[cfg(all(not(target_arch = "wasm32"), feature = "orm"))]
#[cfg_attr(mutants, mutants::skip)]
pub fn safe_pool() -> Option<&'static rullst_orm::RullstPool> {
    rullst_orm::Orm::try_pool().ok()
}

/// Safely retrieves the database driver name, returning `None` if uninitialized.
#[cfg(all(not(target_arch = "wasm32"), feature = "orm"))]
#[cfg_attr(mutants, mutants::skip)]
pub fn safe_driver() -> Option<&'static str> {
    rullst_orm::Orm::try_driver().ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_db_get_pool() {
        let config = ReplicationConfig::new("test.db")
            .with_sync_interval(20)
            .with_auth_token("secret");
        assert_eq!(config.replica_path, "test.db");
        assert_eq!(config.sync_interval_secs, 20);
        assert_eq!(config.auth_token, Some("secret".to_string()));
    }

    #[test]
    fn test_replication_config_with_sync_url() {
        let config = ReplicationConfig::new("test.db").with_sync_url("http://sync");
        assert_eq!(config.sync_url, Some("http://sync".to_string()));
    }

    #[test]
    fn test_replication_config_with_auth_token() {
        let config = ReplicationConfig::new("test.db").with_auth_token("token123");
        assert_eq!(config.auth_token, Some("token123".to_string()));
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), feature = "orm"))]
    fn test_safe_pool_uninitialized() {
        // Assuming Orm isn't initialized in this isolated test, safe_pool should safely return None
        let pool = safe_pool();
        assert!(pool.is_none());
    }

    #[test]
    #[cfg(all(not(target_arch = "wasm32"), feature = "orm"))]
    fn test_safe_driver_uninitialized() {
        let driver = safe_driver();
        assert!(driver.is_none());
    }

    #[test]
    fn test_replication_manager_start() {
        let config = ReplicationConfig::new("test.db")
            .with_sync_url("https://sync.rullst.dev")
            .with_sync_interval(1);
        assert_eq!(
            ReplicationManager::start(config),
            Err(ReplicationError::Unsupported {
                sync_url: "https://sync.rullst.dev".to_string(),
            })
        );
    }

    #[test]
    fn test_replication_manager_start_no_url() {
        let config = ReplicationConfig::new("test.db");
        assert!(ReplicationManager::start(config).is_ok());
    }
}
