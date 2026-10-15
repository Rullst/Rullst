//! How `#[memoize]` stores a function's result in the memory cache.

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::is_exact_json;
use crate::html::Markup;

/// A result type that `#[memoize]` can cache. Support for the `#[memoize]`
/// expansion only.
///
/// Serde types are stored as JSON, unless reading them back would change
/// them (see [`is_exact_json`]). [`Markup`] is stored as its HTML and read
/// back as the same `Markup`, so memoized views can return `html!` output.
#[doc(hidden)]
pub trait MemoValue: Sized {
    /// The cached form of `self`, or `None` when it must not be cached.
    fn to_memo(&self) -> Option<String>;
    /// Reads a value written by [`MemoValue::to_memo`] for the same type.
    fn from_memo(cached: &str) -> Option<Self>;
}

impl<T: Serialize + DeserializeOwned> MemoValue for T {
    fn to_memo(&self) -> Option<String> {
        if is_exact_json(self) {
            serde_json::to_string(self).ok()
        } else {
            None
        }
    }

    fn from_memo(cached: &str) -> Option<Self> {
        serde_json::from_str(cached).ok()
    }
}

impl MemoValue for Markup {
    fn to_memo(&self) -> Option<String> {
        Some(self.as_str().to_owned())
    }

    // Only the memoized function that wrote the entry reads it back, and it
    // wrote the HTML of a `Markup`.
    fn from_memo(cached: &str) -> Option<Self> {
        Some(Markup::from_trusted(cached))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn serde_results_round_trip_unless_json_would_change_them() {
        assert_eq!(Some(2.5_f64).to_memo().as_deref(), Some("2.5"));
        assert_eq!(Some(f64::NAN).to_memo(), None);
        assert_eq!(
            <Option<f64> as MemoValue>::from_memo("2.5"),
            Some(Some(2.5))
        );
        assert_eq!(<u8 as MemoValue>::from_memo("not json"), None);
    }

    #[test]
    fn markup_is_stored_as_its_html() {
        let markup = Markup::from_trusted("<b>&lt;x&gt;</b>");
        assert_eq!(markup.to_memo().as_deref(), Some("<b>&lt;x&gt;</b>"));
        assert_eq!(
            <Markup as MemoValue>::from_memo("<i>y</i>"),
            Some(Markup::from_trusted("<i>y</i>"))
        );
    }
}
