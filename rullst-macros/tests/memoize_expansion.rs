extern crate self as rullst;

// A per-thread stand-in for `rullst::cache::memory`, so each test sees only
// the entries its own calls wrote. The real store is tested by the facade.
pub mod cache {
    pub mod memory {
        use std::cell::RefCell;
        use std::collections::HashMap;

        thread_local! {
            static STORE: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
        }

        pub fn get(key: &str) -> Option<String> {
            STORE.with(|store| store.borrow().get(key).cloned())
        }

        pub fn set(key: &str, value: &str) {
            STORE.with(|store| store.borrow_mut().insert(key.to_owned(), value.to_owned()));
        }

        pub fn keys() -> Vec<String> {
            STORE.with(|store| store.borrow().keys().cloned().collect())
        }

        /// Stand-in for the JSON exactness probe: NaN has no exact JSON form.
        pub fn is_exact_json<T: std::fmt::Debug + ?Sized>(value: &T) -> bool {
            !format!("{value:?}").contains("NaN")
        }

        pub trait MemoValue: Sized {
            fn to_memo(&self) -> Option<String>;
            fn from_memo(cached: &str) -> Option<Self>;
        }

        impl MemoValue for i64 {
            fn to_memo(&self) -> Option<String> {
                Some(self.to_string())
            }

            fn from_memo(cached: &str) -> Option<Self> {
                cached.parse().ok()
            }
        }

        impl MemoValue for String {
            fn to_memo(&self) -> Option<String> {
                // A result the store must not keep.
                (!self.starts_with("uncacheable")).then(|| self.clone())
            }

            fn from_memo(cached: &str) -> Option<Self> {
                Some(cached.to_owned())
            }
        }
    }
}

use rullst_macros::memoize;
use std::sync::atomic::{AtomicUsize, Ordering};

static DOUBLE_CALLS: AtomicUsize = AtomicUsize::new(0);

#[memoize]
fn double(value: i64) -> i64 {
    DOUBLE_CALLS.fetch_add(1, Ordering::SeqCst);
    value * 2
}

#[test]
fn a_repeated_call_returns_the_cached_result() {
    assert_eq!(double(2), 4);
    assert_eq!(double(2), 4);
    assert_eq!(DOUBLE_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(double(3), 6);
    assert_eq!(DOUBLE_CALLS.load(Ordering::SeqCst), 2);

    // The key names the function and its arguments as JSON.
    let keys = cache::memory::keys();
    assert_eq!(keys.len(), 2, "{keys:?}");
    for key in &keys {
        assert!(key.starts_with("memoize_expansion::double@"), "{key}");
    }
    assert!(keys.iter().any(|key| key.ends_with(":[2]")), "{keys:?}");
    assert!(keys.iter().any(|key| key.ends_with(":[3]")), "{keys:?}");
}

static LABEL_CALLS: AtomicUsize = AtomicUsize::new(0);

#[memoize]
fn label(ratio: f64, name: String) -> String {
    LABEL_CALLS.fetch_add(1, Ordering::SeqCst);
    format!("{name}:{ratio}")
}

#[test]
fn inexact_arguments_and_uncacheable_results_run_every_time() {
    assert_eq!(label(f64::NAN, "a".to_string()), "a:NaN");
    assert_eq!(label(f64::NAN, "a".to_string()), "a:NaN");
    assert_eq!(LABEL_CALLS.load(Ordering::SeqCst), 2);
    assert!(cache::memory::keys().is_empty());

    assert_eq!(label(0.5, "uncacheable".to_string()), "uncacheable:0.5");
    assert_eq!(label(0.5, "uncacheable".to_string()), "uncacheable:0.5");
    assert_eq!(LABEL_CALLS.load(Ordering::SeqCst), 4);
    assert!(cache::memory::keys().is_empty());

    assert_eq!(label(0.5, "b".to_string()), "b:0.5");
    assert_eq!(label(0.5, "b".to_string()), "b:0.5");
    assert_eq!(LABEL_CALLS.load(Ordering::SeqCst), 5);
}

static TRIPLE_CALLS: AtomicUsize = AtomicUsize::new(0);

#[memoize]
fn triple(value: i64) -> i64 {
    TRIPLE_CALLS.fetch_add(1, Ordering::SeqCst);
    value * 3
}

#[test]
fn a_stored_value_that_cannot_be_read_back_reruns_the_body() {
    // An entry under the same key that `from_memo` rejects is ignored and
    // replaced.
    assert_eq!(triple(14), 42);
    let [key] = &cache::memory::keys()[..] else {
        panic!("expected one key");
    };
    cache::memory::set(key, "not a number");
    assert_eq!(triple(14), 42);
    assert_eq!(TRIPLE_CALLS.load(Ordering::SeqCst), 2);
    assert_eq!(cache::memory::get(key).as_deref(), Some("42"));
    assert_eq!(triple(14), 42);
    assert_eq!(TRIPLE_CALLS.load(Ordering::SeqCst), 2);
}
