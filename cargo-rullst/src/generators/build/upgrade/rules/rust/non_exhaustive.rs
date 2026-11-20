//! 13.0.0-alpha.6 struct literals and tuple-struct calls of the Rullst
//! structs that became `#[non_exhaustive]` ([`super::super::alpha6_non_exhaustive`]).
//! A review finding through a glob import is also recorded for
//! [`super::super::non_exhaustive_globs`], which resolves it per package.

use super::super::Rule;
use super::super::alpha6_non_exhaustive::{call_crates, call_rule, literal_crates, literal_rule};
use super::super::catalog::NON_EXHAUSTIVE_LITERAL_GLOB;
use super::line;
use syn::Expr;

impl super::Visitor<'_, '_> {
    /// Reports a [`literal`].
    pub(super) fn non_exhaustive_literal(&mut self, expr: &syn::ExprStruct) {
        if let Some((rule, at)) = literal(expr, self.context.text) {
            self.hit(rule, at);
            if rule.code == NON_EXHAUSTIVE_LITERAL_GLOB.code
                && let Some(last) = expr.path.segments.last()
            {
                let name = last.ident.to_string();
                let crates = literal_crates(&name, &field_names(expr));
                self.out.add_glob_literal(at, name, crates);
            }
        }
    }

    /// Reports a call of a non-exhaustive tuple struct, such as
    /// `RagRetrievalError(message)`.
    pub(super) fn non_exhaustive_call(&mut self, expr: &syn::ExprCall) {
        if let Expr::Path(path) = &*expr.func
            && let Some(last) = path.path.segments.last()
            && let Some(rule) = call_rule(&last.ident.to_string(), self.context.text)
        {
            let at = line(last.ident.span());
            self.hit(rule, at);
            if rule.code == NON_EXHAUSTIVE_LITERAL_GLOB.code {
                let name = last.ident.to_string();
                let crates = call_crates(&name);
                self.out.add_glob_literal(at, name, crates);
            }
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
    literal_rule(&last.ident.to_string(), &field_names(expr), text)
        .map(|rule| (rule, line(last.ident.span())))
}

/// The named fields a struct literal sets.
fn field_names(expr: &syn::ExprStruct) -> Vec<String> {
    expr.fields
        .iter()
        .filter_map(|field| match &field.member {
            syn::Member::Named(name) => Some(name.to_string()),
            syn::Member::Unnamed(_) => None,
        })
        .collect()
}
