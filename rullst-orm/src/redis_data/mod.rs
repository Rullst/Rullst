//! Bounded Redis Hash, Set, and Sorted Set adapter.

use std::time::Duration;

use async_trait::async_trait;
use redis::{IntoConnectionInfo, aio::ConnectionManagerConfig};

use crate::polyglot::{Backend, BackendCapabilities, Capability, PolyglotError};

mod config;
mod mock;
#[cfg(test)]
mod tests;
mod types;

pub use config::RedisDataConfig;
use config::ValidatedRedisDataConfig;
use mock::MockRedisData;
use types::validate_score;
pub use types::{
    RedisDataKey, RedisField, RedisMember, RedisScanLimit, RedisStructure, RedisValue,
    ScoredRedisMember,
};

const MAX_SCAN_ROUNDS: usize = 128;

/// Explicit native Redis structures with bounded inputs and reads.
#[async_trait]
pub trait RedisStructuresRepository: Send + Sync {
    /// Sets one field and reports whether it was newly inserted.
    async fn hash_set(
        &self,
        key: &RedisDataKey,
        field: &RedisField,
        value: &RedisValue,
    ) -> Result<bool, PolyglotError>;

    /// Gets one bounded hash value.
    async fn hash_get(
        &self,
        key: &RedisDataKey,
        field: &RedisField,
    ) -> Result<Option<RedisValue>, PolyglotError>;

    /// Atomically increments a signed 64-bit hash field.
    async fn hash_increment(
        &self,
        key: &RedisDataKey,
        field: &RedisField,
        amount: i64,
    ) -> Result<i64, PolyglotError>;

    /// Adds a set member and reports whether it was new.
    async fn set_add(
        &self,
        key: &RedisDataKey,
        member: &RedisMember,
    ) -> Result<bool, PolyglotError>;

    /// Checks exact set membership.
    async fn set_contains(
        &self,
        key: &RedisDataKey,
        member: &RedisMember,
    ) -> Result<bool, PolyglotError>;

    /// Scans at most `limit` set members; ordering is backend-defined.
    async fn set_scan(
        &self,
        key: &RedisDataKey,
        limit: RedisScanLimit,
    ) -> Result<Vec<RedisMember>, PolyglotError>;

    /// Adds or updates one finite sorted-set score.
    async fn sorted_set_add(
        &self,
        key: &RedisDataKey,
        member: &RedisMember,
        score: f64,
    ) -> Result<bool, PolyglotError>;

    /// Returns at most `limit` members ordered by descending score.
    async fn sorted_set_top(
        &self,
        key: &RedisDataKey,
        limit: RedisScanLimit,
    ) -> Result<Vec<ScoredRedisMember>, PolyglotError>;

    /// Deletes exactly one namespaced Redis structure.
    async fn delete(
        &self,
        key: &RedisDataKey,
        structure: RedisStructure,
    ) -> Result<bool, PolyglotError>;
}

struct LiveRedisData {
    manager: redis::aio::ConnectionManager,
    namespace: String,
}

enum RedisDataInner {
    Live(LiveRedisData),
    Mock(MockRedisData),
}

/// Native Redis datastore with deterministic empty/`mock_*` fallback.
pub struct RedisDataStore {
    inner: RedisDataInner,
}

impl RedisDataStore {
    /// Connects with bounded timeouts or selects the offline fallback.
    pub async fn connect_or_mock(config: RedisDataConfig) -> Result<Self, PolyglotError> {
        if config.requests_mock() {
            config.validate_mock_namespace()?;
            return Ok(Self {
                inner: RedisDataInner::Mock(MockRedisData::default()),
            });
        }
        let config = config.validate()?;
        Ok(Self {
            inner: RedisDataInner::Live(LiveRedisData::connect(config).await?),
        })
    }

    /// Declares the bounded Redis data-structure capability.
    pub const fn capabilities() -> BackendCapabilities {
        BackendCapabilities::new(Backend::Redis, &[Capability::KeyValueStructures])
    }

    /// Returns whether this instance uses the deterministic offline backend.
    pub const fn is_mock(&self) -> bool {
        matches!(&self.inner, RedisDataInner::Mock(_))
    }
}

#[async_trait]
impl RedisStructuresRepository for RedisDataStore {
    async fn hash_set(
        &self,
        key: &RedisDataKey,
        field: &RedisField,
        value: &RedisValue,
    ) -> Result<bool, PolyglotError> {
        let RedisDataInner::Live(live) = &self.inner else {
            return redis_mock(self)?.hash_set(key, field, value).await;
        };
        let inserted: i64 = redis::cmd("HSET")
            .arg(live.key(RedisStructure::Hash, key))
            .arg(field.as_str())
            .arg(value.as_str())
            .query_async(&mut live.manager.clone())
            .await
            .map_err(|_| redis_error())?;
        Ok(inserted == 1)
    }

    async fn hash_get(
        &self,
        key: &RedisDataKey,
        field: &RedisField,
    ) -> Result<Option<RedisValue>, PolyglotError> {
        let RedisDataInner::Live(live) = &self.inner else {
            return redis_mock(self)?.hash_get(key, field).await;
        };
        let value: Option<String> = redis::cmd("HGET")
            .arg(live.key(RedisStructure::Hash, key))
            .arg(field.as_str())
            .query_async(&mut live.manager.clone())
            .await
            .map_err(|_| redis_error())?;
        value.map(RedisValue::new).transpose()
    }

    async fn hash_increment(
        &self,
        key: &RedisDataKey,
        field: &RedisField,
        amount: i64,
    ) -> Result<i64, PolyglotError> {
        let RedisDataInner::Live(live) = &self.inner else {
            return redis_mock(self)?.hash_increment(key, field, amount).await;
        };
        redis::cmd("HINCRBY")
            .arg(live.key(RedisStructure::Hash, key))
            .arg(field.as_str())
            .arg(amount)
            .query_async(&mut live.manager.clone())
            .await
            .map_err(|_| redis_error())
    }

    async fn set_add(
        &self,
        key: &RedisDataKey,
        member: &RedisMember,
    ) -> Result<bool, PolyglotError> {
        let RedisDataInner::Live(live) = &self.inner else {
            return redis_mock(self)?.set_add(key, member).await;
        };
        let inserted: i64 = redis::cmd("SADD")
            .arg(live.key(RedisStructure::Set, key))
            .arg(member.as_str())
            .query_async(&mut live.manager.clone())
            .await
            .map_err(|_| redis_error())?;
        Ok(inserted == 1)
    }

    async fn set_contains(
        &self,
        key: &RedisDataKey,
        member: &RedisMember,
    ) -> Result<bool, PolyglotError> {
        let RedisDataInner::Live(live) = &self.inner else {
            return redis_mock(self)?.set_contains(key, member).await;
        };
        redis::cmd("SISMEMBER")
            .arg(live.key(RedisStructure::Set, key))
            .arg(member.as_str())
            .query_async(&mut live.manager.clone())
            .await
            .map_err(|_| redis_error())
    }

    async fn set_scan(
        &self,
        key: &RedisDataKey,
        limit: RedisScanLimit,
    ) -> Result<Vec<RedisMember>, PolyglotError> {
        let RedisDataInner::Live(live) = &self.inner else {
            return redis_mock(self)?.set_scan(key, limit).await;
        };
        live.set_scan(key, limit).await
    }

    async fn sorted_set_add(
        &self,
        key: &RedisDataKey,
        member: &RedisMember,
        score: f64,
    ) -> Result<bool, PolyglotError> {
        let RedisDataInner::Live(live) = &self.inner else {
            return redis_mock(self)?.sorted_set_add(key, member, score).await;
        };
        validate_score(score)?;
        let inserted: i64 = redis::cmd("ZADD")
            .arg(live.key(RedisStructure::SortedSet, key))
            .arg(score)
            .arg(member.as_str())
            .query_async(&mut live.manager.clone())
            .await
            .map_err(|_| redis_error())?;
        Ok(inserted == 1)
    }

    async fn sorted_set_top(
        &self,
        key: &RedisDataKey,
        limit: RedisScanLimit,
    ) -> Result<Vec<ScoredRedisMember>, PolyglotError> {
        let RedisDataInner::Live(live) = &self.inner else {
            return redis_mock(self)?.sorted_set_top(key, limit).await;
        };
        let stop = i64::from(limit.get()) - 1;
        let rows: Vec<(String, f64)> = redis::cmd("ZREVRANGE")
            .arg(live.key(RedisStructure::SortedSet, key))
            .arg(0)
            .arg(stop)
            .arg("WITHSCORES")
            .query_async(&mut live.manager.clone())
            .await
            .map_err(|_| redis_error())?;
        if rows.len() > usize::from(limit.get()) {
            return Err(redis_error());
        }
        rows.into_iter()
            .map(|(member, score)| ScoredRedisMember::new(RedisMember::new(member)?, score))
            .collect()
    }

    async fn delete(
        &self,
        key: &RedisDataKey,
        structure: RedisStructure,
    ) -> Result<bool, PolyglotError> {
        let RedisDataInner::Live(live) = &self.inner else {
            return redis_mock(self)?.delete(key, structure).await;
        };
        let deleted: i64 = redis::cmd("UNLINK")
            .arg(live.key(structure, key))
            .query_async(&mut live.manager.clone())
            .await
            .map_err(|_| redis_error())?;
        Ok(deleted == 1)
    }
}

impl LiveRedisData {
    async fn connect(config: ValidatedRedisDataConfig) -> Result<Self, PolyglotError> {
        let mut connection = config
            .endpoint
            .as_str()
            .into_connection_info()
            .map_err(|_| invalid_config("endpoint could not configure the Redis driver"))?;
        if let Some((username, password)) = config.credentials {
            let settings = connection
                .redis_settings()
                .clone()
                .set_username(username)
                .set_password(password);
            connection = connection.set_redis_settings(settings);
        }
        let client = redis::Client::open(connection).map_err(|_| redis_error())?;
        let manager_config = ConnectionManagerConfig::new()
            .set_number_of_retries(1)
            .set_connection_timeout(Some(Duration::from_secs(5)))
            .set_response_timeout(Some(Duration::from_secs(5)));
        let manager = redis::aio::ConnectionManager::new_with_config(client, manager_config)
            .await
            .map_err(|_| redis_error())?;
        Ok(Self {
            manager,
            namespace: config.namespace,
        })
    }

    fn key(&self, structure: RedisStructure, key: &RedisDataKey) -> String {
        let kind = match structure {
            RedisStructure::Hash => "hash",
            RedisStructure::Set => "set",
            RedisStructure::SortedSet => "zset",
        };
        format!(
            "rullst:orm:v1:{}:data:{kind}:{}",
            self.namespace,
            key.as_str()
        )
    }

    async fn set_scan(
        &self,
        key: &RedisDataKey,
        limit: RedisScanLimit,
    ) -> Result<Vec<RedisMember>, PolyglotError> {
        let mut cursor = 0_u64;
        let mut members = Vec::new();
        for _ in 0..MAX_SCAN_ROUNDS {
            let (next, chunk): (u64, Vec<String>) = redis::cmd("SSCAN")
                .arg(self.key(RedisStructure::Set, key))
                .arg(cursor)
                .arg("COUNT")
                .arg(limit.get())
                .query_async(&mut self.manager.clone())
                .await
                .map_err(|_| redis_error())?;
            for member in chunk {
                members.push(RedisMember::new(member)?);
                if members.len() == usize::from(limit.get()) {
                    return Ok(members);
                }
            }
            if next == 0 {
                return Ok(members);
            }
            cursor = next;
        }
        Err(redis_error())
    }
}

fn redis_mock(store: &RedisDataStore) -> Result<&MockRedisData, PolyglotError> {
    match &store.inner {
        RedisDataInner::Mock(mock) => Ok(mock),
        RedisDataInner::Live(_) => Err(redis_error()),
    }
}

pub(super) fn redis_error() -> PolyglotError {
    PolyglotError::Driver {
        backend: "Redis",
        message: "Redis operation failed".to_owned(),
    }
}

fn invalid_config(reason: &'static str) -> PolyglotError {
    PolyglotError::InvalidConfiguration {
        backend: "Redis",
        reason,
    }
}
