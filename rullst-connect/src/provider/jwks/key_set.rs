//! The Rullst-owned key set that [`super::JwksCache`] returns.

use jsonwebtoken::jwk::{Jwk, JwkSet};
use std::fmt;
use std::sync::Arc;

/// A provider's JSON Web Key Set, as fetched and cached by
/// [`JwksCache`](super::JwksCache). Cloning shares the set.
///
/// The providers verify ID tokens with it; application code can see which
/// keys it holds. The JWT library's own key type is not exposed.
#[derive(Clone)]
pub struct JsonWebKeySet(Arc<JwkSet>);

impl JsonWebKeySet {
    pub(crate) fn new(keys: JwkSet) -> Self {
        Self(Arc::new(keys))
    }

    /// The key with this `kid`, for signature verification inside the crate.
    pub(crate) fn find(&self, kid: &str) -> Option<&Jwk> {
        self.0.find(kid)
    }

    pub(crate) fn jwk_set(&self) -> &JwkSet {
        &self.0
    }

    /// The key IDs (`kid`) of the keys that carry one, in the provider's order.
    pub fn key_ids(&self) -> Vec<&str> {
        self.0
            .keys
            .iter()
            .filter_map(|key| key.common.key_id.as_deref())
            .collect()
    }

    /// Whether the set holds a key with this `kid`.
    pub fn contains_key_id(&self, kid: &str) -> bool {
        self.0.find(kid).is_some()
    }

    /// The number of keys in the set.
    pub fn len(&self) -> usize {
        self.0.keys.len()
    }

    /// Whether the set holds no key.
    pub fn is_empty(&self) -> bool {
        self.0.keys.is_empty()
    }

    /// Whether both values share the same fetched set.
    #[cfg(test)]
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for JsonWebKeySet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("JsonWebKeySet")
            .field("key_ids", &self.key_ids())
            .finish()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn exposes_key_ids_and_counts_without_the_library_type() {
        let keys: JwkSet = serde_json::from_value(serde_json::json!({
            "keys": [
                {"kty": "oct", "k": "c2VjcmV0", "kid": "first"},
                {"kty": "oct", "k": "c2VjcmV0"},
                {"kty": "oct", "k": "c2VjcmV0", "kid": "second"}
            ]
        }))
        .unwrap();
        let set = JsonWebKeySet::new(keys);
        assert_eq!(set.key_ids(), ["first", "second"]);
        assert!(set.contains_key_id("second"));
        assert!(!set.contains_key_id("third"));
        assert_eq!(set.len(), 3);
        assert!(!set.is_empty());
        assert!(set.find("first").is_some());
        assert_eq!(set.jwk_set().keys.len(), 3);
        let shared = set.clone();
        assert!(set.ptr_eq(&shared));
        assert!(!set.ptr_eq(&JsonWebKeySet::new(JwkSet { keys: Vec::new() })));
        assert!(JsonWebKeySet::new(JwkSet { keys: Vec::new() }).is_empty());
        assert_eq!(
            format!("{set:?}"),
            r#"JsonWebKeySet { key_ids: ["first", "second"] }"#
        );
    }
}
