//! Dirty tracking for `#[derive(Orm)]` models.
//!
//! A model opts in by declaring one `#[sqlx(skip)]` field of type
//! [`RowSnapshot`]. Records loaded from the database then remember a
//! fingerprint of every column that `save()` writes, and `save()` updates only
//! the columns whose value changed since the record was loaded or last saved.
//! Two requests that edit different fields of the same row therefore both
//! persist.
//!
//! A fingerprint is a 64-bit SipHash, keyed with a random per-process key, of
//! the value's `serde_json` serialization: plaintext for `#[orm(encrypted)]`
//! fields and `SecretString` values, and the canonical decoded document (object
//! keys sorted) for `#[sqlx(json)]` and `Json<T>` fields. The snapshot holds 8
//! bytes per tracked column and never the values themselves.

use serde::Serialize;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::OnceLock;

/// The fingerprint of a value that could not be serialized. It never matches,
/// so such a column is always written.
const UNKNOWN: u64 = 0;

/// Column fingerprints of a model handle, as loaded from or last written to
/// the database.
///
/// Declare it on a model to make `save()` write only the columns that changed:
///
/// ```ignore
/// #[derive(Debug, Clone, Orm)]
/// #[orm(table = "posts")]
/// pub struct Post {
///     pub id: i32,
///     pub title: String,
///     pub body: String,
///     #[sqlx(skip)]
///     pub snapshot: rullst_orm::RowSnapshot,
/// }
/// ```
///
/// A record built in code holds the default, untracked snapshot: `save()`
/// inserts it (`id == 0`) or writes every column (`id != 0`). The snapshot is
/// bookkeeping, not data: every two snapshots compare equal and hash alike, so
/// a derived `PartialEq` or `Hash` on the model looks only at its columns.
/// Mark the field `#[serde(skip)]` when the model derives serde traits.
#[derive(Clone, Default)]
pub struct RowSnapshot {
    fingerprints: Option<Box<[u64]>>,
}

impl RowSnapshot {
    /// Whether this handle knows the stored values of its columns: `true`
    /// after a database read or a save, `false` for a record built in code or
    /// after `mark_all_dirty()`.
    pub fn is_tracked(&self) -> bool {
        self.fingerprints.is_some()
    }

    #[doc(hidden)]
    pub fn __from_fingerprints(fingerprints: &[u64]) -> Self {
        Self {
            fingerprints: Some(fingerprints.into()),
        }
    }

    /// Whether the column at `index` must be written: its stored fingerprint
    /// is unknown or differs from `fingerprint`.
    #[doc(hidden)]
    pub fn __is_changed(&self, index: usize, fingerprint: u64) -> bool {
        let stored = self
            .fingerprints
            .as_deref()
            .and_then(|fingerprints| fingerprints.get(index));
        match stored {
            // An `UNKNOWN` fingerprint never equals a known stored one, so only
            // the stored side needs its own check.
            Some(&stored) => stored == UNKNOWN || stored != fingerprint,
            None => true,
        }
    }

    /// Whether any column of `fingerprints` must be written.
    #[doc(hidden)]
    pub fn __any_changed(&self, fingerprints: &[u64]) -> bool {
        fingerprints
            .iter()
            .enumerate()
            .any(|(index, fingerprint)| self.__is_changed(index, *fingerprint))
    }
}

impl std::fmt::Debug for RowSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RowSnapshot")
            .field("tracked", &self.is_tracked())
            .finish()
    }
}

impl PartialEq for RowSnapshot {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for RowSnapshot {}

impl PartialOrd for RowSnapshot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RowSnapshot {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

impl std::hash::Hash for RowSnapshot {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

/// Streams serialized bytes into the hasher without buffering them.
struct HashWriter<'a>(&'a mut std::hash::DefaultHasher);

impl std::io::Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.write(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn process_keys() -> &'static RandomState {
    static KEYS: OnceLock<RandomState> = OnceLock::new();
    KEYS.get_or_init(RandomState::new)
}

/// Fingerprints one column value for dirty tracking. `SecretString` values
/// contribute their plaintext; a value that cannot be serialized yields a
/// fingerprint that never matches.
#[doc(hidden)]
pub fn fingerprint<T: Serialize + ?Sized>(value: &T) -> u64 {
    let mut hasher = process_keys().build_hasher();
    let serialized = crate::privacy::with_secret_fingerprints(|| {
        serde_json::to_writer(HashWriter(&mut hasher), value)
    });
    match serialized {
        // `UNKNOWN` is reserved for values that could not be serialized.
        Ok(()) => hasher.finish().max(1),
        Err(_) => UNKNOWN,
    }
}

/// Fingerprints a JSON column by its canonical value: object keys are sorted
/// at every level, so a reordered but equal document is unchanged.
#[doc(hidden)]
pub fn fingerprint_json<T: Serialize + ?Sized>(value: &T) -> u64 {
    let Ok(mut value) = crate::privacy::with_secret_fingerprints(|| serde_json::to_value(value))
    else {
        return UNKNOWN;
    };
    let mut pending = vec![&mut value];
    while let Some(node) = pending.pop() {
        match node {
            serde_json::Value::Object(map) => {
                map.sort_keys();
                pending.extend(map.values_mut());
            }
            serde_json::Value::Array(items) => pending.extend(items.iter_mut()),
            _ => {}
        }
    }
    fingerprint(&value)
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
