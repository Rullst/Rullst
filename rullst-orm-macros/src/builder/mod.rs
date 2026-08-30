// src/builder/mod.rs — Procedural code generator for Active Record Query Builders.

use crate::parser::{ParsedModel, SoftDeleteConfig};
use proc_macro2::TokenStream;

pub mod chunking;
pub mod clauses;
pub mod execution;
pub mod magic_methods;
pub mod sql_assembly;
pub mod where_clauses;

pub use chunking::generate_chunk_methods;
pub use clauses::generate_builder_struct;
pub use execution::generate_execution_methods;
pub use magic_methods::generate_magic_methods;
pub use sql_assembly::generate_sql_assembly_methods;
pub use where_clauses::generate_where_clause_methods;

/// Identifies how a soft-delete value should be compared inside
/// `SELECT` / `restore` queries. The "literal" mode matches against
/// `<column> = <value>` while the "null" mode matches against
/// `<column> IS NULL` / `IS NOT NULL`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SoftDeleteCmp {
    NullSentinel,
    LiteralSentinel,
}

impl SoftDeleteCmp {
    pub fn for_value(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("null") || value.trim().is_empty() {
            SoftDeleteCmp::NullSentinel
        } else {
            SoftDeleteCmp::LiteralSentinel
        }
    }
}

/// Renders the `<column> = <value>` fragment used in `SELECT` queries
/// to filter non-deleted rows.
#[cfg_attr(test, mutants::skip)]
pub fn soft_delete_where_clause(cfg: &SoftDeleteConfig, is_trashed: bool) -> String {
    let cmp = SoftDeleteCmp::for_value(&cfg.value);
    match (cmp, is_trashed) {
        (SoftDeleteCmp::NullSentinel, false) => format!("{} IS NULL", cfg.column),
        (SoftDeleteCmp::NullSentinel, true) => format!("{} IS NOT NULL", cfg.column),
        (SoftDeleteCmp::LiteralSentinel, false) => format!("{} = {}", cfg.column, cfg.value),
        (SoftDeleteCmp::LiteralSentinel, true) => format!("{} != {}", cfg.column, cfg.value),
    }
}

pub fn generate(
    parsed: &ParsedModel,
    relation_flags: &[TokenStream],
    relation_inits: &[TokenStream],
    relation_methods: &[TokenStream],
    eager_loads: &TokenStream,
) -> TokenStream {
    let name = &parsed.name;
    let column_enum_name = quote::format_ident!("{}Column", name);
    let builder_name = quote::format_ident!("{}QueryBuilder", name);

    let soft_delete_filter_unset = parsed
        .soft_delete
        .as_ref()
        .map(|cfg| soft_delete_where_clause(cfg, false))
        .unwrap_or_else(|| "deleted_at IS NULL".to_string());
    let soft_delete_filter_set = parsed
        .soft_delete
        .as_ref()
        .map(|cfg| soft_delete_where_clause(cfg, true))
        .unwrap_or_else(|| "deleted_at IS NOT NULL".to_string());

    let where_clause_methods = generate_where_clause_methods(&column_enum_name);
    let sql_assembly_methods = generate_sql_assembly_methods(
        &parsed.table_name,
        parsed.has_soft_deletes,
        &soft_delete_filter_unset,
        &soft_delete_filter_set,
    );
    let mut execution_methods = generate_execution_methods(parsed, &builder_name, eager_loads);
    execution_methods.extend(generate_chunk_methods(parsed));
    let magic_methods = generate_magic_methods(parsed);

    generate_builder_struct(
        parsed,
        &builder_name,
        relation_flags,
        relation_inits,
        relation_methods,
        &where_clause_methods,
        &sql_assembly_methods,
        &execution_methods,
        &magic_methods,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soft_delete_where_clause() {
        let cfg_null = SoftDeleteConfig {
            column: "deleted_at".into(),
            value: "null".into(),
            delval: "1".into(),
        };
        assert_eq!(
            soft_delete_where_clause(&cfg_null, false),
            "deleted_at IS NULL"
        );
        assert_eq!(
            soft_delete_where_clause(&cfg_null, true),
            "deleted_at IS NOT NULL"
        );

        let cfg_lit = SoftDeleteConfig {
            column: "is_deleted".into(),
            value: "0".into(),
            delval: "1".into(),
        };
        assert_eq!(soft_delete_where_clause(&cfg_lit, false), "is_deleted = 0");
        assert_eq!(soft_delete_where_clause(&cfg_lit, true), "is_deleted != 0");
    }
}
