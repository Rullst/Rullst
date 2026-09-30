//! Small in-memory vector helpers.

use std::collections::HashMap;

/// Represents a document stored inside a vector memory index.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorDocument {
    /// Unique identifier of the document.
    pub id: String,
    /// High-dimensional floating point embedding vector.
    pub vector: Vec<f32>,
    /// Additional JSON payload containing document metadata.
    pub payload: serde_json::Value,
}

/// In-memory search index supporting cosine similarity vector lookup.
pub struct VectorIndex {
    documents: HashMap<String, VectorDocument>,
}

impl Default for VectorIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorIndex {
    /// Creates a new, empty `VectorIndex`.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Inserts or updates a document inside the vector index.
    pub fn add(&mut self, id: impl Into<String>, vector: Vec<f32>, payload: serde_json::Value) {
        let id = id.into();
        self.documents.insert(
            id.clone(),
            VectorDocument {
                id,
                vector,
                payload,
            },
        );
    }

    /// Searches the index returning the top matches sorted by cosine similarity descending.
    ///
    /// Scores come from [`cosine_similarity`], so a document whose similarity is
    /// not finite scores `0.0`. Ranking uses a total order and never panics.
    #[cfg_attr(mutants, mutants::skip)]
    pub fn search(&self, query_vector: &[f32], limit: usize) -> Vec<(f32, &VectorDocument)> {
        if query_vector.is_empty() || self.documents.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<_> = self
            .documents
            .values()
            .map(|document| (cosine_similarity(query_vector, &document.vector), document))
            .collect();
        results.sort_by(|left, right| right.0.total_cmp(&left.0));
        results.truncate(limit);
        results
    }
}

/// Calculates the cosine similarity score between two float vectors.
///
/// Returns `0.0` when the lengths differ, a vector is empty or has zero norm,
/// or the result is not finite (a NaN or infinite component, or components
/// whose squares overflow `f32`).
pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let mut dot_product = 0.0;
    let mut norm_left = 0.0;
    let mut norm_right = 0.0;
    for (left_value, right_value) in left.iter().zip(right.iter()) {
        dot_product += left_value * right_value;
        norm_left += left_value * left_value;
        norm_right += right_value * right_value;
    }
    if norm_left == 0.0 || norm_right == 0.0 {
        return 0.0;
    }
    let similarity = dot_product / (norm_left.sqrt() * norm_right.sqrt());
    if similarity.is_finite() {
        similarity
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_handles_orthogonal_and_invalid_vectors() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn vector_index_returns_the_closest_document() {
        let mut index = VectorIndex::new();
        index.add("x", vec![1.0, 0.0], serde_json::json!({}));
        index.add("y", vec![0.0, 1.0], serde_json::json!({}));
        let results = index.search(&[0.9, 0.1], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.id, "x");
    }

    #[test]
    fn non_finite_similarities_score_zero_and_rank_with_a_total_order() {
        assert_eq!(cosine_similarity(&[f32::NAN, 1.0], &[1.0, 0.0]), 0.0);
        assert_eq!(cosine_similarity(&[f32::INFINITY, 0.0], &[1.0, 0.0]), 0.0);
        // Finite components whose squares overflow f32 give inf / inf.
        assert_eq!(cosine_similarity(&[1e20, 1e20], &[1e20, 1e20]), 0.0);

        let mut index = VectorIndex::new();
        for position in 0..64_u8 {
            let vector = if position % 3 == 0 {
                vec![f32::NAN, 1.0]
            } else {
                vec![1.0, f32::from(position)]
            };
            index.add(format!("doc-{position}"), vector, serde_json::json!({}));
        }
        let results = index.search(&[1.0, 0.0], 64);
        assert_eq!(results.len(), 64);
        assert!(results.iter().all(|(score, _)| score.is_finite()));
        assert!(results.windows(2).all(|pair| pair[0].0 >= pair[1].0));
        assert_eq!(results[0].1.id, "doc-1");
    }
}
