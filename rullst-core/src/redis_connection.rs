//! Shared Redis connection for the built-in cache and queue drivers.
//!
//! Each driver keeps one lazily established [`MultiplexedConnection`] and
//! clones it for every operation instead of opening a new TCP connection. A
//! command error that leaves the connection unusable discards it, so the
//! failing operation still returns its error and the next one reconnects.

use redis::aio::{ConnectionLike, MultiplexedConnection};
use redis::{AsyncConnectionConfig, Cmd, Pipeline, RedisError, RedisFuture, RedisResult, Value};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

#[cfg(test)]
#[path = "redis_connection_tests.rs"]
mod tests;

/// Lazily connected, reusable multiplexed connection owned by one driver.
pub(crate) struct SharedRedisConnection {
    client: redis::Client,
    config: AsyncConnectionConfig,
    cached: Mutex<CachedConnection>,
}

#[derive(Default)]
struct CachedConnection {
    generation: u64,
    connection: Option<MultiplexedConnection>,
}

impl SharedRedisConnection {
    /// Wraps a client without opening a network connection. Commands use the
    /// redis-rs default response timeout (500 ms).
    #[cfg_attr(not(feature = "cache-redis"), allow(dead_code))]
    pub(crate) fn new(client: redis::Client) -> Self {
        Self::with_config(client, AsyncConnectionConfig::new())
    }

    /// Like [`Self::new`], waiting up to `timeout` for each response.
    #[cfg_attr(not(feature = "queue-redis"), allow(dead_code))]
    pub(crate) fn with_response_timeout(client: redis::Client, timeout: Duration) -> Self {
        Self::with_config(
            client,
            AsyncConnectionConfig::new().set_response_timeout(Some(timeout)),
        )
    }

    fn with_config(client: redis::Client, config: AsyncConnectionConfig) -> Self {
        Self {
            client,
            config,
            cached: Mutex::new(CachedConnection::default()),
        }
    }

    /// Returns a handle to the shared connection, connecting on first use or
    /// after the previous connection was discarded.
    ///
    /// Connecting happens without holding the lock, so an unavailable server
    /// fails every waiting operation within the client's connection timeout
    /// instead of serializing them. If several tasks connect at once, the
    /// first stored connection wins and the others are dropped.
    pub(crate) async fn connection(&self) -> RedisResult<RedisConnection<'_>> {
        if let Some(handle) = self.cached_handle() {
            return Ok(handle);
        }
        let connection = self
            .client
            .get_multiplexed_async_connection_with_config(&self.config)
            .await?;
        let mut cached = self.lock();
        let (generation, inner) = match &cached.connection {
            Some(existing) => (cached.generation, existing.clone()),
            None => {
                cached.generation = cached.generation.wrapping_add(1);
                cached.connection = Some(connection.clone());
                (cached.generation, connection)
            }
        };
        Ok(RedisConnection {
            owner: self,
            generation,
            inner,
        })
    }

    fn cached_handle(&self) -> Option<RedisConnection<'_>> {
        let cached = self.lock();
        let inner = cached.connection.clone()?;
        Some(RedisConnection {
            owner: self,
            generation: cached.generation,
            inner,
        })
    }

    /// Discards the connection a failed command used, unless it was already
    /// replaced, when the error means the connection cannot be reused.
    fn discard_if_broken(&self, generation: u64, error: &RedisError) {
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

/// Clone of the shared connection used for one driver operation.
pub(crate) struct RedisConnection<'a> {
    owner: &'a SharedRedisConnection,
    generation: u64,
    inner: MultiplexedConnection,
}

impl ConnectionLike for RedisConnection<'_> {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        Box::pin(async move {
            let result = self.inner.req_packed_command(cmd).await;
            if let Err(error) = &result {
                self.owner.discard_if_broken(self.generation, error);
            }
            result
        })
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        pipeline: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        Box::pin(async move {
            let result = self
                .inner
                .req_packed_commands(pipeline, offset, count)
                .await;
            if let Err(error) = &result {
                self.owner.discard_if_broken(self.generation, error);
            }
            result
        })
    }

    fn get_db(&self) -> i64 {
        self.inner.get_db()
    }
}
