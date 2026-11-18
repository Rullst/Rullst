//! 13.0.0-alpha.6 struct literals and tuple-struct calls of the Rullst
//! structs that became `#[non_exhaustive]` ([`super::super::alpha6_non_exhaustive`]).

use super::super::Rule;
use super::super::alpha6_non_exhaustive::{call_rule, literal_rule};
use super::line;
use syn::Expr;

impl super::Visitor<'_, '_> {
    /// Reports a [`literal`].
    pub(super) fn non_exhaustive_literal(&mut self, expr: &syn::ExprStruct) {
        if let Some((rule, at)) = literal(expr, self.context.text) {
            self.hit(rule, at);
        }
    }

    /// Reports a call of a non-exhaustive tuple struct, such as
    /// `RagRetrievalError(message)`.
    pub(super) fn non_exhaustive_call(&mut self, expr: &syn::ExprCall) {
        if let Expr::Path(path) = &*expr.func
            && let Some(last) = path.path.segments.last()
            && let Some(rule) = call_rule(&last.ident.to_string(), self.context.text)
        {
            self.hit(rule, line(last.ident.span()));
        }
    }
}

/// The rule and line of a literal of a non-exhaustive Rullst struct.
pub(super) fn literal(expr: &syn::ExprStruct, text: &str) -> Option<(&'static Rule, usize)> {
    // `crate::mail::Message { .. }`: a type of the application's own module.
    let first = expr.path.segments.first()?;
    if first.ident == "crate" || first.ident == "self" || first.ident == "super" {
        return None;
    }
    let last = expr.path.segments.last()?;
    let fields = expr
        .fields
        .iter()
        .filter_map(|field| match &field.member {
            syn::Member::Named(name) => Some(name.to_string()),
            syn::Member::Unnamed(_) => None,
        })
        .collect::<Vec<_>>();
    literal_rule(&last.ident.to_string(), &fields, text).map(|rule| (rule, line(last.ident.span())))
}
