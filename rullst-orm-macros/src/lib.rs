//! Derive and attribute macros of `rullst-orm`: `#[derive(Orm)]`,
//! `#[derive(TursoModel)]`, `#[derive(Enum)]`, `#[derive(PersonalData)]`,
//! `#[derive(Nexus)]` and the `#[rullst_orm::test]` sandbox attribute.
//!
//! **Maturity: Core**
//! ([tiers](https://rullst.github.io/Rullst/book/maturity.html)). Depend on
//! `rullst-orm` (or on the `rullst` facade with its `orm` feature) rather than
//! on this crate: the generated code calls the `rullst-orm` runtime, which
//! re-exports these macros and enables the matching features of this crate.
//! `#[derive(Nexus)]` also needs the `rullst` facade with its `nexus` feature.
//!
//! The derives reject unknown or duplicate options and unsupported model
//! shapes at compile time. The
//! [README](https://github.com/Rullst/Rullst/blob/main/rullst-orm-macros/README.md)
//! documents the options and their limits, and the
//! [ORM guide](https://rullst.github.io/Rullst/book/crates/orm.html) shows them
//! in use.
extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod builder;
mod enums;
mod factory_observer;
mod feature_gates;
mod models;
mod nexus;
mod parser;
mod privacy;
mod query_span;
mod relationships;
mod sandbox_test;
mod turso;

#[cfg_attr(test, mutants::skip)]
#[proc_macro_attribute]
pub fn test(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as syn::ItemFn);
    TokenStream::from(sandbox_test::expand(&input_fn))
}

#[cfg_attr(test, mutants::skip)]
#[proc_macro_derive(PersonalData, attributes(privacy))]
pub fn derive_personal_data(input: TokenStream) -> TokenStream {
    privacy::derive_personal_data_impl(input)
}

#[cfg_attr(test, mutants::skip)]
/// Derives the bounded relational-enum label and codec contract.
///
/// Container attributes accept `type_name` and `rename_all = "lowercase"` or
/// `"snake_case"`; unit variants accept `rename`. Runtime DDL is emitted only
/// when the enum is passed to `rullst_orm::schema::Blueprint::native_enum`.
#[proc_macro_derive(Enum, attributes(rullst_enum))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    enums::derive_enum_impl(input.into()).into()
}

#[cfg_attr(test, mutants::skip)]
#[proc_macro_derive(Nexus, attributes(nexus))]
pub fn derive_nexus(input: TokenStream) -> TokenStream {
    nexus::derive_nexus_impl(input)
}

/// Derives the Active Record model: query builder, persistence, relations,
/// observers and, since 13.0.0-alpha.4, `sqlx::FromRow`.
///
/// The generated `FromRow` is generic over the row type like SQLx's derive
/// and gives field `#[sqlx(skip)]`, `#[sqlx(default)]`, `#[sqlx(json)]`,
/// `#[sqlx(json(nullable))]` and struct-level `#[sqlx(default)]` SQLx's
/// meaning; relation fields are never read and start from
/// `Default::default()`. Do not also derive `FromRow`, which would be a
/// second, conflicting implementation (E0119). A model that implements
/// `FromRow` itself, by hand or with SQLx's derive (SQLx's `derive` feature),
/// declares `#[orm(custom_from_row)]`.
#[cfg_attr(test, mutants::skip)]
#[proc_macro_derive(Orm, attributes(orm, sqlx))]
pub fn rullst_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Parse the input
    let parsed = match parser::parse(&input) {
        Ok(p) => p,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    if parsed.backend == "turso" {
        return match turso::generate(&parsed, &input) {
            Ok(expanded) => expanded.into(),
            Err(error) => error.to_compile_error().into(),
        };
    }

    // Generate relationships
    let rels = relationships::generate(&parsed);

    // Generate the builder
    let builder_code = builder::generate(
        &parsed,
        &rels.flags,
        &rels.inits,
        &rels.methods,
        &rels.eager_loads,
    );

    // Generate factory and observers
    let factory_observer_code = factory_observer::generate(&parsed);

    // Generate the model impl
    let model_code = models::generate(&parsed, &rels.model_methods);

    // `sqlx::FromRow`, unless `#[orm(custom_from_row)]`
    let from_row_code = models::from_row::generate(&parsed);

    // Combine
    let expanded = quote::quote! {
        #builder_code
        #factory_observer_code
        #model_code
        #from_row_code
    };

    TokenStream::from(expanded)
}

/// Derives the typed Turso/libSQL model contract without generating SQLx code.
#[cfg_attr(test, mutants::skip)]
#[proc_macro_derive(TursoModel, attributes(orm, sqlx))]
pub fn derive_turso_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let parsed = match parser::parse(&input) {
        Ok(parsed) => parsed,
        Err(error) => return error.to_compile_error().into(),
    };
    match turso::generate(&parsed, &input) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
