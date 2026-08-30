use std::{cmp::Ordering, collections::BTreeMap};

use tokio::sync::RwLock;

use super::{
    VectorCollectionName, VectorDimensions, VectorMatch, VectorPoint, VectorQueryLimit,
    validate_payload, validate_vector,
};
use crate::polyglot::PolyglotError;

#[derive(Default)]
pub(super) struct MockQdrant {
    collections: RwLock<BTreeMap<String, MockCollection>>,
}

struct MockCollection {
    dimensions: VectorDimensions,
    points: BTreeMap<u64, VectorPoint>,
}

impl MockQdrant {
    pub(super) async fn create_collection(
        &self,
        collection: &VectorCollectionName,
        dimensions: VectorDimensions,
    ) -> Result<(), PolyglotError> {
        let mut collections = self.collections.write().await;
        if collections.contains_key(collection.as_str()) {
            return Err(PolyglotError::Conflict);
        }
        collections.insert(
            collection.as_str().to_owned(),
            MockCollection {
                dimensions,
                points: BTreeMap::new(),
            },
        );
        Ok(())
    }

    pub(super) async fn upsert(
        &self,
        collection: &VectorCollectionName,
        point: VectorPoint,
    ) -> Result<(), PolyglotError> {
        let mut collections = self.collections.write().await;
        let target = collections
            .get_mut(collection.as_str())
            .ok_or(PolyglotError::NotFound)?;
        validate_vector(point.vector(), Some(target.dimensions))?;
        target.points.insert(point.id(), point);
        Ok(())
    }

    pub(super) async fn delete(
        &self,
        collection: &VectorCollectionName,
        id: u64,
    ) -> Result<(), PolyglotError> {
        let mut collections = self.collections.write().await;
        let target = collections
            .get_mut(collection.as_str())
            .ok_or(PolyglotError::NotFound)?;
        target.points.remove(&id);
        Ok(())
    }

    pub(super) async fn search(
        &self,
        collection: &VectorCollectionName,
        query: &[f32],
        limit: VectorQueryLimit,
    ) -> Result<Vec<VectorMatch>, PolyglotError> {
        let collections = self.collections.read().await;
        let target = collections
            .get(collection.as_str())
            .ok_or(PolyglotError::NotFound)?;
        validate_vector(query, Some(target.dimensions))?;
        let mut matches = target
            .points
            .values()
            .map(|point| {
                VectorMatch::new(
                    point.id(),
                    cosine_similarity(query, point.vector()),
                    point.payload().clone(),
                )
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            right
                .score()
                .partial_cmp(&left.score())
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.id().cmp(&right.id()))
        });
        matches.truncate(usize::from(limit.get()));
        for result in &matches {
            validate_payload(result.payload())?;
        }
        Ok(matches)
    }
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| f64::from(*left) * f64::from(*right))
        .sum::<f64>();
    let left_norm = left
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt();
    let right_norm = right
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt();
    (dot / (left_norm * right_norm)) as f32
}
