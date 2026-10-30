//! The `rullst.orm.query` span around generated async methods.
//!
//! `#[tracing::instrument]` expands to `::tracing::…` paths, which resolve
//! only in a crate that depends on `tracing` itself, so every application that
//! derived a model had to declare it. Generated methods build the same span
//! through `rullst_orm::_tracing` instead: `tracing`'s macros reach their own
//! items through `$crate`, so the expansion needs nothing beyond `rullst_orm`.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

/// Runs `body`, the statements of a generated async method, inside the span
/// that `#[instrument(name = "rullst.orm.query", target = "rullst_orm",
/// skip(..), fields(orm.model, orm.table, orm.operation))]` created, so traces
/// keep their shape:
///
/// - info level, target `rullst_orm`, name `rullst.orm.query`;
/// - first the parameters `instrument` did not skip, named in `recorded`
///   (all primitive, so recorded by value as `instrument` did), then
///   `orm.model`, `orm.table` and `orm.operation`;
/// - the body is an `async move` block entered only while it is polled, so
///   no span guard is held across a suspension, and a disabled span is not
///   entered at all.
///
/// The span is created before the block captures the parameters, and `return`
/// and `?` in `body` leave the block, as they did under `instrument`.
pub(crate) fn instrument_query(
    model: &Ident,
    table_name: &str,
    operation: &str,
    recorded: &[&str],
    body: TokenStream,
) -> TokenStream {
    let fields = recorded
        .iter()
        .map(|parameter| Ident::new(parameter, Span::call_site()))
        .collect::<Vec<_>>();
    let values = &fields;
    quote! {
        let __rullst_query_span = rullst_orm::_tracing::info_span!(
            target: "rullst_orm",
            "rullst.orm.query",
            #(#fields = #values,)*
            orm.model = stringify!(#model),
            orm.table = #table_name,
            orm.operation = #operation
        );
        let __rullst_query = async move { #body };
        if __rullst_query_span.is_disabled() {
            __rullst_query.await
        } else {
            rullst_orm::_tracing::Instrument::instrument(__rullst_query, __rullst_query_span).await
        }
    }
}

#[cfg(test)]
#[path = "query_span_tests.rs"]
mod tests;
