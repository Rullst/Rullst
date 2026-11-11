//! The storage adapters 13.0.0-alpha.5 removed (`V13-ORM-ADAPTERS-REMOVED`):
//! MongoDB (`MongoDbStore`) and DuckDB (`DuckDbStore` with its analytics
//! types). [`super::files`] locates their Cargo features; [`super::tokens`]
//! and [`super::rust`] call these with the identifiers and paths they read.
//! The adapters lived in `rullst_orm::polyglot` (also `rullst::orm::polyglot`),
//! so their names count only in a file that names that module: an
//! application's own `AnalyticsRow` or `MongoDbStore` elsewhere is left alone.

use super::Rule;
use super::catalog::ORM_ADAPTERS_REMOVED;

/// Public names of the removed adapters.
const REMOVED_IDENTS: &[&str] = &[
    "MongoDbStore",
    "DuckDbStore",
    "AnalyticsRepository",
    "AnalyticsRow",
    "AnalyticsValue",
    "AnalyticsTimeUnit",
    "QueryLimit",
];

/// Removed module paths and the variant only the removed adapters produced.
const REMOVED_PATHS: &[[&str; 2]] = &[
    ["polyglot", "mongodb"],
    ["polyglot", "duckdb"],
    ["PolyglotError", "Worker"],
];

/// Variants of the ORM's `Backend` and `Capability` enums that only the removed
/// adapters declared; both enums are also exported at the crate root.
const REMOVED_VARIANTS: &[[&str; 2]] = &[
    ["Backend", "MongoDb"],
    ["Backend", "DuckDb"],
    ["Capability", "Analytics"],
];

fn names_polyglot(text: &str) -> bool {
    text.contains("polyglot")
}

fn names_orm(text: &str) -> bool {
    names_polyglot(text) || text.contains("rullst_orm") || text.contains("orm::")
}

fn has_pair(segments: &[String], pairs: &[[&str; 2]]) -> bool {
    segments.windows(2).any(|window| {
        pairs
            .iter()
            .any(|pair| window[0] == pair[0] && window[1] == pair[1])
    })
}

/// The rule for an identifier in a file whose text names `polyglot`.
pub(super) fn ident_rule(name: &str, text: &str) -> Option<&'static Rule> {
    (REMOVED_IDENTS.contains(&name) && names_polyglot(text)).then_some(&ORM_ADAPTERS_REMOVED)
}

/// The rule for a path such as `polyglot::mongodb` or `Backend::DuckDb`; the
/// enum variants only in a file that names the ORM.
pub(super) fn path_rule(segments: &[String], text: &str) -> Option<&'static Rule> {
    (has_pair(segments, REMOVED_PATHS) || (has_pair(segments, REMOVED_VARIANTS) && names_orm(text)))
        .then_some(&ORM_ADAPTERS_REMOVED)
}
