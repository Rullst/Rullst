use std::collections::HashMap;
use std::hash::Hash;

/// An extension trait that brings Laravel-style Collection methods natively to Rust's Vec<T>.
pub trait RullstCollection<T> {
    /// Keys the collection by the given closure's return value
    fn key_by<K, F>(self, f: F) -> HashMap<K, T>
    where
        F: Fn(&T) -> K,
        K: Hash + Eq;

    /// Splits the collection into chunks of the given size
    /// Maps each item using the given closure
    fn map<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U;

    /// Filters items using the given closure
    fn filter<F>(self, f: F) -> Vec<T>
    where
        F: FnMut(&T) -> bool;

    fn chunk(self, size: usize) -> Vec<Vec<T>>;

    /// Joins the items into a single string using the given separator and closure
    fn implode<F>(&self, separator: &str, f: F) -> String
    where
        F: Fn(&T) -> String;

    /// Sums up the values returned by the closure
    fn sum_by<N, F>(&self, f: F) -> N
    where
        F: Fn(&T) -> N,
        N: std::iter::Sum;

    /// Finds the maximum value returned by the closure
    fn max_by_key<K, F>(&self, f: F) -> Option<&T>
    where
        F: Fn(&T) -> K,
        K: Ord;

    /// Finds the minimum value returned by the closure
    fn min_by_key<K, F>(&self, f: F) -> Option<&T>
    where
        F: Fn(&T) -> K,
        K: Ord;

    /// Serializes the entire collection using an ApiResource transformer
    fn collection_resource(&self) -> serde_json::Value
    where
        T: crate::resource::ApiResource;
}

impl<T> RullstCollection<T> for Vec<T> {
    fn key_by<K, F>(self, f: F) -> HashMap<K, T>
    where
        F: Fn(&T) -> K,
        K: Hash + Eq,
    {
        let mut map = HashMap::with_capacity(self.len());
        for item in self {
            map.insert(f(&item), item);
        }
        map
    }

    fn map<U, F>(self, f: F) -> Vec<U>
    where
        F: FnMut(T) -> U,
    {
        self.into_iter().map(f).collect()
    }

    fn filter<F>(self, f: F) -> Vec<T>
    where
        F: FnMut(&T) -> bool,
    {
        self.into_iter().filter(f).collect()
    }

    fn chunk(self, size: usize) -> Vec<Vec<T>> {
        if self.is_empty() {
            return vec![];
        }
        if size == 0 {
            return vec![self];
        }

        let mut remaining = self.len();
        let mut chunks = Vec::with_capacity(remaining.div_ceil(size));
        let mut iter = self.into_iter();

        while let Some(first) = iter.next() {
            let chunk_cap = std::cmp::min(size, remaining);
            let mut chunk = Vec::with_capacity(chunk_cap);
            chunk.push(first);
            remaining -= 1;
            for _ in 1..size {
                if let Some(item) = iter.next() {
                    chunk.push(item);
                    remaining -= 1;
                } else {
                    break;
                }
            }
            chunks.push(chunk);
        }

        chunks
    }

    fn implode<F>(&self, separator: &str, f: F) -> String
    where
        F: Fn(&T) -> String,
    {
        // Pre-allocate to reduce intermediate allocations
        let mut result = String::with_capacity(self.len() * 16);
        let mut iter = self.iter();
        if let Some(first) = iter.next() {
            result.push_str(&f(first));
            for item in iter {
                result.push_str(separator);
                result.push_str(&f(item));
            }
        }
        result
    }

    fn sum_by<N, F>(&self, f: F) -> N
    where
        F: Fn(&T) -> N,
        N: std::iter::Sum,
    {
        self.iter().map(f).sum()
    }

    fn max_by_key<K, F>(&self, f: F) -> Option<&T>
    where
        F: Fn(&T) -> K,
        K: Ord,
    {
        self.iter().max_by_key(|item| f(*item))
    }

    fn min_by_key<K, F>(&self, f: F) -> Option<&T>
    where
        F: Fn(&T) -> K,
        K: Ord,
    {
        self.iter().min_by_key(|item| f(*item))
    }

    #[cfg_attr(test, mutants::skip)]
    fn collection_resource(&self) -> serde_json::Value
    where
        T: crate::resource::ApiResource,
    {
        crate::resource::ResourceCollection::new(self).resolve()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_by() {
        let v = vec![(1u32, "a"), (2, "b"), (3, "c")];
        let map = v.key_by(|(k, _)| *k);
        assert_eq!(map[&1].1, "a");
        assert_eq!(map[&3].1, "c");
    }

    #[test]
    fn test_map() {
        let v = vec![1, 2, 3];
        let mapped = v.map(|x| x * 2);
        assert_eq!(mapped, vec![2, 4, 6]);
    }

    #[test]
    fn test_filter() {
        let v = vec![1, 2, 3, 4];
        let filtered = v.filter(|x| x % 2 == 0);
        assert_eq!(filtered, vec![2, 4]);
    }

    #[test]
    fn test_chunk_even() {
        let v = vec![1, 2, 3, 4];
        let chunks = v.chunk(2);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0], vec![1, 2]);
        assert_eq!(chunks[1], vec![3, 4]);
    }

    #[test]
    fn test_chunk_with_remainder() {
        let v = vec![1, 2, 3, 4, 5];
        let chunks = v.chunk(2);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[2], vec![5]);
        assert_eq!(chunks[0].capacity(), 2);
        assert_eq!(chunks[1].capacity(), 2);
        assert_eq!(chunks[2].capacity(), 1);
    }

    #[test]
    fn test_chunk_zero_returns_all() {
        let v = vec![1, 2, 3];
        let chunks = v.chunk(0);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], vec![1, 2, 3]);
    }

    #[test]
    fn test_implode() {
        let v = vec![1, 2, 3];
        let result = v.implode(", ", |n| n.to_string());
        assert_eq!(result, "1, 2, 3");
    }

    #[test]
    fn test_implode_single_element() {
        let v = vec![42];
        let result = v.implode(", ", |n| n.to_string());
        assert_eq!(result, "42");
    }

    #[test]
    fn test_sum_by() {
        let v = vec![1, 2, 3, 4];
        let sum: i32 = v.sum_by(|n| *n);
        assert_eq!(sum, 10);
    }

    #[test]
    fn test_max_by_key() {
        let v = vec![3, 1, 4, 1, 5, 9];
        let max = v.max_by_key(|n| *n);
        assert_eq!(max, Some(&9));
    }

    #[test]
    fn test_min_by_key() {
        let v = vec![3, 1, 4, 1, 5, 9];
        let min = v.min_by_key(|n| *n);
        assert_eq!(min, Some(&1));
    }

    #[test]
    fn test_empty_collection() {
        let v: Vec<i32> = vec![];
        assert!(v.max_by_key(|n| *n).is_none());
        assert!(v.min_by_key(|n| *n).is_none());
        let sum: i32 = v.sum_by(|n| *n);
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_chunk_larger_than_len() {
        let v = vec![1, 2];
        let chunks = v.chunk(5);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], vec![1, 2]);
    }

    #[test]
    fn test_chunk_empty() {
        let v: Vec<i32> = vec![];
        let chunks = v.chunk(2);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_usize_max() {
        let v = vec![1, 2, 3];
        let chunks = v.chunk(usize::MAX);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], vec![1, 2, 3]);
    }

    struct DummyResource {
        id: i32,
    }

    impl crate::resource::ApiResource for DummyResource {
        fn to_array(&self) -> serde_json::Value {
            crate::_serde_json::json!({ "id": self.id })
        }
    }

    #[test]
    fn test_collection_resource() {
        let vec = vec![DummyResource { id: 1 }, DummyResource { id: 2 }];
        let res = vec.collection_resource();
        assert_eq!(res, crate::_serde_json::json!([{"id": 1}, {"id": 2}]));
    }
}
