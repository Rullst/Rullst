//! Relation-key normalization for generated relationship loaders.
//!
//! A relation key may be a nullable foreign key (`Option<i32>`) on either side
//! of the relation. Generated loaders compare the inner value: a `None` key
//! matches no row, like SQL `NULL`. The field type is only known after macro
//! expansion, so the generated code selects the conversion by autoref
//! dispatch: `(&Key(&field)).__rullst_relation_key()` resolves to [`Nullable`]
//! for an `Option<T>` field and to [`Required`] for any other key type.

/// Borrows one relation key field for autoref dispatch.
pub struct Key<'a, T>(pub &'a T);

/// Unwraps a nullable key; `None` matches no row.
pub trait Nullable {
    type Key;
    fn __rullst_relation_key(&self) -> Option<Self::Key>;
}

impl<T: Clone> Nullable for Key<'_, Option<T>> {
    type Key = T;
    fn __rullst_relation_key(&self) -> Option<T> {
        self.0.clone()
    }
}

/// Keeps a non-nullable key as it is.
pub trait Required {
    type Key;
    fn __rullst_relation_key(&self) -> Option<Self::Key>;
}

impl<T: Clone> Required for &Key<'_, T> {
    type Key = T;
    fn __rullst_relation_key(&self) -> Option<T> {
        Some(self.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{Key, Nullable as _, Required as _};

    // Generated code borrows every key the same way, whatever its type.
    #[allow(clippy::needless_borrow)]
    #[test]
    fn nullable_keys_unwrap_and_required_keys_pass_through() {
        let present: Option<i32> = Some(7);
        let missing: Option<i32> = None;
        let required = 9_i32;
        let text = String::from("slug");
        assert_eq!((&Key(&present)).__rullst_relation_key(), Some(7));
        assert_eq!((&Key(&missing)).__rullst_relation_key(), None);
        assert_eq!((&Key(&required)).__rullst_relation_key(), Some(9));
        assert_eq!(
            (&Key(&text)).__rullst_relation_key(),
            Some("slug".to_string())
        );
    }
}
