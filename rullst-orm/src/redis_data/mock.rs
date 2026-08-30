use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use tokio::sync::RwLock;

use super::{
    RedisDataKey, RedisField, RedisMember, RedisScanLimit, RedisStructure, RedisValue,
    ScoredRedisMember, redis_error, validate_score,
};
use crate::polyglot::PolyglotError;

#[derive(Default)]
pub(super) struct MockRedisData {
    hashes: RwLock<BTreeMap<String, BTreeMap<String, String>>>,
    sets: RwLock<BTreeMap<String, BTreeSet<String>>>,
    sorted_sets: RwLock<BTreeMap<String, BTreeMap<String, f64>>>,
}

impl MockRedisData {
    pub(super) async fn hash_set(
        &self,
        key: &RedisDataKey,
        field: &RedisField,
        value: &RedisValue,
    ) -> Result<bool, PolyglotError> {
        let mut hashes = self.hashes.write().await;
        let inserted = hashes
            .entry(key.as_str().to_owned())
            .or_default()
            .insert(field.as_str().to_owned(), value.as_str().to_owned())
            .is_none();
        Ok(inserted)
    }

    pub(super) async fn hash_get(
        &self,
        key: &RedisDataKey,
        field: &RedisField,
    ) -> Result<Option<RedisValue>, PolyglotError> {
        self.hashes
            .read()
            .await
            .get(key.as_str())
            .and_then(|hash| hash.get(field.as_str()))
            .cloned()
            .map(RedisValue::new)
            .transpose()
    }

    pub(super) async fn hash_increment(
        &self,
        key: &RedisDataKey,
        field: &RedisField,
        amount: i64,
    ) -> Result<i64, PolyglotError> {
        let mut hashes = self.hashes.write().await;
        let hash = hashes.entry(key.as_str().to_owned()).or_default();
        let current = hash.get(field.as_str()).map_or(Ok(0), |value| {
            value.parse::<i64>().map_err(|_| redis_error())
        })?;
        let updated = current.checked_add(amount).ok_or_else(redis_error)?;
        hash.insert(field.as_str().to_owned(), updated.to_string());
        Ok(updated)
    }

    pub(super) async fn set_add(
        &self,
        key: &RedisDataKey,
        member: &RedisMember,
    ) -> Result<bool, PolyglotError> {
        Ok(self
            .sets
            .write()
            .await
            .entry(key.as_str().to_owned())
            .or_default()
            .insert(member.as_str().to_owned()))
    }

    pub(super) async fn set_contains(
        &self,
        key: &RedisDataKey,
        member: &RedisMember,
    ) -> Result<bool, PolyglotError> {
        Ok(self
            .sets
            .read()
            .await
            .get(key.as_str())
            .is_some_and(|set| set.contains(member.as_str())))
    }

    pub(super) async fn set_scan(
        &self,
        key: &RedisDataKey,
        limit: RedisScanLimit,
    ) -> Result<Vec<RedisMember>, PolyglotError> {
        self.sets
            .read()
            .await
            .get(key.as_str())
            .into_iter()
            .flat_map(|set| set.iter())
            .take(usize::from(limit.get()))
            .cloned()
            .map(RedisMember::new)
            .collect()
    }

    pub(super) async fn sorted_set_add(
        &self,
        key: &RedisDataKey,
        member: &RedisMember,
        score: f64,
    ) -> Result<bool, PolyglotError> {
        validate_score(score)?;
        Ok(self
            .sorted_sets
            .write()
            .await
            .entry(key.as_str().to_owned())
            .or_default()
            .insert(member.as_str().to_owned(), score)
            .is_none())
    }

    pub(super) async fn sorted_set_top(
        &self,
        key: &RedisDataKey,
        limit: RedisScanLimit,
    ) -> Result<Vec<ScoredRedisMember>, PolyglotError> {
        let sorted_sets = self.sorted_sets.read().await;
        let mut rows = sorted_sets
            .get(key.as_str())
            .into_iter()
            .flat_map(|set| set.iter())
            .map(|(member, score)| (member.clone(), *score))
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        rows.truncate(usize::from(limit.get()));
        rows.into_iter()
            .map(|(member, score)| ScoredRedisMember::new(RedisMember::new(member)?, score))
            .collect()
    }

    pub(super) async fn delete(
        &self,
        key: &RedisDataKey,
        structure: RedisStructure,
    ) -> Result<bool, PolyglotError> {
        let removed = match structure {
            RedisStructure::Hash => self.hashes.write().await.remove(key.as_str()).is_some(),
            RedisStructure::Set => self.sets.write().await.remove(key.as_str()).is_some(),
            RedisStructure::SortedSet => self
                .sorted_sets
                .write()
                .await
                .remove(key.as_str())
                .is_some(),
        };
        Ok(removed)
    }
}
