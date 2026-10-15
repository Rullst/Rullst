//! Compile-time translation catalogs: `catalog!` and the `__translate!`
//! helper behind the `t!` macro that `catalog!` defines.

mod catalog;
mod emit;
mod tag;
mod template;
mod translate;

pub(crate) use emit::expand_catalog;
pub(crate) use translate::expand_translate;

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod catalog_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
