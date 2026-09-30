//! Shared Redis connection for [`super::RedisRateLimiter`].
//!
//! A limiter and its clones keep one lazily established
//! [`MultiplexedConnection`] and clone it for every check instead of opening
//! a new TCP connection. A command error that leaves the connection unusable
//! discards it, so the failing check still returns its error and the next one
//! reconnects.

use redis::aio::MultiplexedConnection;
use redis::{RedisError, RedisResult};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Lazily connected, reusable multiplexed connection owned by one limiter.
pub(super) struct SharedConnection {
    client: redis::Client,
    cached: Mutex<CachedConnection>,
}

#[derive(Default)]
struct CachedConnection {
    generation: u64,
    connection: Option<MultiplexedConnection>,
}

/// Clone of the shared connection used for one check, tagged with the
/// generation it belongs to.
pub(super) struct ConnectionHandle {
    pub(super) generation: u64,
    pub(super) connection: MultiplexedConnection,
}

impl SharedConnection {
    /// Wraps a client without opening a network connection.
    pub(super) fn new(client: redis::Client) -> Self {
        Self {
            client,
            cached: Mutex::new(CachedConnection::default()),
        }
    }

    /// Returns a handle to the shared connection, connecting on first use or
    /// after the previous connection was discarded.
    ///
    /// Connecting happens without holding the lock, so an unavailable server
    /// fails every waiting check within the client's connection timeout
    /// instead of serializing them. If several checks connect at once, the
    /// first stored connection wins and the others are dropped.
    pub(super) async fn connection(&self) -> RedisResult<ConnectionHandle> {
        if let Some(handle) = self.cached_handle() {
            return Ok(handle);
        }
        let connection = self.client.get_multiplexed_async_connection().await?;
        let mut cached = self.lock();
        if let Some(existing) = &cached.connection {
            return Ok(ConnectionHandle {
                generation: cached.generation,
                connection: existing.clone(),
            });
        }
        cached.generation = cached.generation.wrapping_add(1);
        cached.connection = Some(connection.clone());
        Ok(ConnectionHandle {
            generation: cached.generation,
            connection,
        })
    }

    fn cached_handle(&self) -> Option<ConnectionHandle> {
        let cached = self.lock();
        let connection = cached.connection.clone()?;
        Some(ConnectionHandle {
            generation: cached.generation,
            connection,
        })
    }

    /// Discards the connection a failed command used, unless it was already
    /// replaced, when the error means the connection cannot be reused.
    pub(super) fn discard_if_broken(&self, generation: u64, error: &RedisError) {
        if !(error.is_unrecoverable_error() || error.is_connection_dropped()) {
            return;
        }
        let mut cached = self.lock();
        if cached.generation == generation {
            cached.connection = None;
        }
    }

    fn lock(&self) -> MutexGuard<'_, CachedConnection> {
        // The guarded state is a plain slot that is always valid, so a
        // poisoned lock is safe to reuse.
        self.cached.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
