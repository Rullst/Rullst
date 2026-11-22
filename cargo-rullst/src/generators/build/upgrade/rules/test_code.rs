//! Test code, which the rules about production behaviour skip. A finding of
//! one of these rules asks how the deployed application behaves: which CSRF
//! value a page embeds, which static paths skip the CSRF check, which layers
//! a router that `Server` serves adds again, whether request code bounds
//! password hashing, how a starter's handler pages, reads or reserves, which
//! session cookies sign-in issues and accepts. The same code in a test does
//! not run in production, so it is not a finding.
//!
//! Test code is a file below a `tests` directory or named `tests.rs`,
//! `*_tests.rs` or `*_test.rs`, and, in any file, an item marked
//! `#[cfg(test)]`, `#[test]` or another `…::test` attribute
//! (`#[tokio::test]`, `#[rullst_orm::test]`), with everything inside it: an
//! inline `mod tests { … }`, a test function, a `#[cfg(test)]` helper or
//! `impl`. Rules about code that no longer compiles, and about changed
//! behaviour that a test also observes (a removed probe field, new header
//! values, queue retry counts), still report test code.

use super::Rule;
use super::catalog::{
    AUTH_SESSION_EPOCH_CALLS, AUTH_SESSION_REGISTRY, CREDENTIAL_RATE_LIMIT, CSRF_COOKIE_IN_PAGE,
    CSRF_STATIC_PREFIXES, ERP_DASHBOARD_ACCESS, ERP_STORE_ORDER, LMS_RECORD_PROGRESS, PAGE_BOUNDS,
    PAGE_CSP_NONCE, PROFILE_FIND, ROUTER_BASELINE_LAYERS,
};
use std::path::{Component, Path};
use syn::Attribute;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

/// The rules about production behaviour, whose findings skip test code.
pub(super) const PRODUCTION_ONLY: &[&Rule] = &[
    &CSRF_COOKIE_IN_PAGE,
    &CSRF_STATIC_PREFIXES,
    &ROUTER_BASELINE_LAYERS,
    &CREDENTIAL_RATE_LIMIT,
    &LMS_RECORD_PROGRESS,
    &ERP_DASHBOARD_ACCESS,
    &ERP_STORE_ORDER,
    &PAGE_CSP_NONCE,
    &PAGE_BOUNDS,
    &PROFILE_FIND,
    &AUTH_SESSION_REGISTRY,
    &AUTH_SESSION_EPOCH_CALLS,
];

/// Whether `rule` is about production behaviour, so test code is skipped.
pub(super) fn production_only(rule: &Rule) -> bool {
    PRODUCTION_ONLY.iter().any(|known| known.code == rule.code)
}

/// Whether `path`, relative to its package root, is test code: below a
/// `tests` directory or a `tests.rs`/`*_tests.rs`/`*_test.rs` file.
pub(super) fn is_test_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    path.components()
        .any(|part| part == Component::Normal("tests".as_ref()))
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name.ends_with("_test.rs")
}

/// `#[test]`, `#[tokio::test]` and other `…::test` attributes, or
/// `#[cfg(test)]`.
pub(super) fn is_test_attribute(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| match &attribute.meta {
        syn::Meta::List(list) if list.path.is_ident("cfg") => list.tokens.to_string() == "test",
        meta => meta
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test"),
    })
}

/// The lines of one file that are test code.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct TestLines {
    /// The whole file is test code ([`is_test_path`]).
    whole_file: bool,
    /// The first and last line of each test item, attributes included.
    items: Vec<(usize, usize)>,
}

impl TestLines {
    /// The test code of `file`, at `path` relative to its package root.
    pub(super) fn of(file: &syn::File, path: &Path) -> Self {
        let mut finder = Finder::default();
        finder.visit_file(file);
        Self {
            whole_file: is_test_path(path),
            items: finder.items,
        }
    }

    /// Whether `line` is test code.
    pub(super) fn covers(&self, line: usize) -> bool {
        self.whole_file
            || self
                .items
                .iter()
                .any(|(first, last)| (*first..=*last).contains(&line))
    }
}

/// Collects the test items of a file; their contents are not visited.
#[derive(Default)]
struct Finder {
    items: Vec<(usize, usize)>,
}

impl Finder {
    /// Records `node` when `attributes` mark it as test code, and tells
    /// whether it did.
    fn test_item(&mut self, attributes: &[Attribute], node: &impl Spanned) -> bool {
        if !is_test_attribute(attributes) {
            return false;
        }
        let span = node.span();
        self.items.push((span.start().line, span.end().line));
        true
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attributes: &[Attribute] = match item {
            syn::Item::Const(item) => &item.attrs,
            syn::Item::Enum(item) => &item.attrs,
            syn::Item::Fn(item) => &item.attrs,
            syn::Item::Impl(item) => &item.attrs,
            syn::Item::Macro(item) => &item.attrs,
            syn::Item::Mod(item) => &item.attrs,
            syn::Item::Static(item) => &item.attrs,
            syn::Item::Struct(item) => &item.attrs,
            syn::Item::Trait(item) => &item.attrs,
            syn::Item::Use(item) => &item.attrs,
            _ => &[],
        };
        if !self.test_item(attributes, item) {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if !self.test_item(&item.attrs, item) {
            visit::visit_impl_item_fn(self, item);
        }
    }
}
