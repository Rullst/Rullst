//! 13.0.0-alpha.5 Capital and Mail struct literals: Capital's invoices moved
//! from `f64` amounts to integer minor units behind constructors, and Mail's
//! `Message` gained a `reply_to` field.

use super::super::Rule;
use super::super::catalog::{CAPITAL_INVOICE_MINOR_UNITS, MAIL_MESSAGE_REPLY_TO};
use super::line;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, Token};

/// The rule and line of a struct literal that 13.0.0-alpha.5 breaks: Capital's
/// `Invoice { .. total .. }` or `InvoiceItem { .. amount .. }`, in a file that
/// names Capital, and an exhaustive Mail `Message { .. }` (no `..rest`, no
/// `reply_to`) in a file that names Mail. The field checks keep an
/// application's own `Invoice` or `Message` types out.
pub(super) fn legacy_literal(expr: &syn::ExprStruct, text: &str) -> Option<(&'static Rule, usize)> {
    let name = expr.path.segments.last()?.ident.to_string();
    let names = |field: &str| {
        expr.fields
            .iter()
            .any(|value| matches!(&value.member, syn::Member::Named(name) if name == field))
    };
    let at = line(expr.path.span());
    match name.as_str() {
        "Invoice" if names("total") && names_crate(text, "capital") => {
            Some((&CAPITAL_INVOICE_MINOR_UNITS, at))
        }
        "InvoiceItem" if names("amount") && names_crate(text, "capital") => {
            Some((&CAPITAL_INVOICE_MINOR_UNITS, at))
        }
        "Message"
            if expr.rest.is_none()
                && names("subject")
                && names("body_html")
                && !names("reply_to")
                && names_crate(text, "mail") =>
        {
            Some((&MAIL_MESSAGE_REPLY_TO, at))
        }
        _ => None,
    }
}

/// `rullst_<name>` or `<name>::` (as in `rullst::capital::Invoice`).
fn names_crate(text: &str, name: &str) -> bool {
    text.contains(&format!("rullst_{name}")) || text.contains(&format!("{name}::"))
}

/// The same literals inside `vec![a, b]`, whose body the syntax visitor
/// does not enter (an invoice's `items` are usually built this way).
pub(super) fn in_vec_macro(mac: &syn::Macro, text: &str) -> Vec<(&'static Rule, usize)> {
    let mut finder = Finder {
        text,
        found: Vec::new(),
    };
    finder.vec_macro(mac);
    finder.found
}

struct Finder<'a> {
    text: &'a str,
    found: Vec<(&'static Rule, usize)>,
}

impl Finder<'_> {
    fn vec_macro(&mut self, mac: &syn::Macro) {
        if !mac.path.is_ident("vec") {
            return;
        }
        if let Ok(items) = mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated) {
            for item in &items {
                self.visit_expr(item);
            }
        }
    }
}

impl<'ast> Visit<'ast> for Finder<'_> {
    fn visit_expr_struct(&mut self, expr: &'ast syn::ExprStruct) {
        self.found.extend(legacy_literal(expr, self.text));
        visit::visit_expr_struct(self, expr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.vec_macro(mac);
    }
}
