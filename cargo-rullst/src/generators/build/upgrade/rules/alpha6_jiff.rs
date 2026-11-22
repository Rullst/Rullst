//! The 13.0.0-alpha.6 move from chrono, which RUSTSEC-2026-0543 marks
//! unmaintained, to jiff 1. Rullst's ORM stores dates and times as
//! `rullst::orm::Timestamp`, `DateTime`, `Date` and `Time` and no longer
//! re-exports chrono; Core's `Locale` formats jiff civil values; Mail's
//! scheduled sends and account notices and Capital's invoice date hold a
//! `jiff::Timestamp`. [`super::rust`] calls [`chrono_type`] for model fields
//! and [`super::tokens`] the other rules with what it reads.
//!
//! Each rule is must-change and looks for chrono where it meets a Rullst
//! API, never for an application's own chrono code:
//!
//! - `V13-ORM-CHRONO-TYPES`: a chrono type in a field that a
//!   `#[derive(Orm)]` or `#[derive(Nexus)]` struct reads, under any path,
//!   inside `Option` or another generic, or through a type alias or a
//!   `use … as` rename of the same file; a path through Rullst's re-export
//!   (`rullst::orm::chrono`, `rullst::db::chrono`, `rullst_orm::chrono`),
//!   through SQLx's (`sqlx::types::chrono`) when no manifest enables SQLx's
//!   own `chrono` feature, or a bare `chrono::` path when no manifest
//!   declares chrono either, so that it can only reach Rullst's re-export;
//!   `RullstValue`'s chrono variants; and a chrono value passed to
//!   `RullstValue::from` or to a query method that takes one.
//! - `V13-LOCALE-JIFF-DATES`: `format_date`, `format_time` and
//!   `format_datetime` called with a `DateStyle` or `TimeStyle`, unless the
//!   value is a jiff one ([`ChronoBindings::holds_jiff`]).
//! - `V13-MAIL-JIFF-TIMESTAMPS`: `send_at` (called or read) and
//!   `with_scheduled_at` in a file that names Rullst's Mail, and the
//!   `expires_at` of `PasswordResetAt` and `EmailLoginAt`.
//! - `V13-CAPITAL-INVOICE-JIFF-DATE`: `Invoice::new` with its four
//!   arguments in a file that names Rullst's Capital, unless the date is a
//!   jiff value.

use super::catalog::{
    CAPITAL_INVOICE_JIFF_DATE, LOCALE_JIFF_DATES, MAIL_JIFF_TIMESTAMPS, ORM_CHRONO_TYPES,
};
use super::{Rule, WorkspaceFacts};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use std::collections::BTreeSet;

#[path = "alpha6_jiff/values.rs"]
mod values;
pub(super) use values::ChronoBindings;

/// chrono's zone-less types, which no other date library here names.
const NAIVE_TYPES: [&str; 3] = ["NaiveDateTime", "NaiveDate", "NaiveTime"];

/// `RullstValue`'s variants that held chrono values up to 13.0.0-alpha.5.
const CHRONO_VARIANTS: [&str; 4] = ["DateTime", "NaiveDateTime", "Date", "Time"];

/// Query-builder methods whose values are `impl Into<RullstValue>`, besides
/// the `where_…` and `or_where…` families.
const VALUE_METHODS: [&str; 5] = [
    "bind",
    "on_eq",
    "select_raw_bindings",
    "with_raw_bindings",
    "with_recursive_raw_bindings",
];

/// Whether `ty` names a chrono type: `DateTime<Tz>` (Rullst's `DateTime`
/// takes no argument), a `Naive…` type or one of the file's `aliases`, as
/// itself, a reference, or a generic argument (`Option<…>`, `Vec<…>`).
pub(super) fn chrono_type(ty: &syn::Type, aliases: &BTreeSet<String>) -> bool {
    match ty {
        syn::Type::Path(path) => {
            let named = path.path.segments.last().is_some_and(|last| {
                let name = last.ident.to_string();
                NAIVE_TYPES.contains(&name.as_str())
                    || aliases.contains(&name)
                    || (name == "DateTime"
                        && matches!(last.arguments, syn::PathArguments::AngleBracketed(_)))
            });
            named
                || path.path.segments.iter().any(|segment| {
                    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                        return false;
                    };
                    arguments.args.iter().any(|argument| {
                        matches!(argument, syn::GenericArgument::Type(inner) if chrono_type(inner, aliases))
                    })
                })
        }
        syn::Type::Reference(reference) => chrono_type(&reference.elem, aliases),
        syn::Type::Paren(inner) => chrono_type(&inner.elem, aliases),
        syn::Type::Group(inner) => chrono_type(&inner.elem, aliases),
        syn::Type::Slice(slice) => chrono_type(&slice.elem, aliases),
        syn::Type::Array(array) => chrono_type(&array.elem, aliases),
        syn::Type::Tuple(tuple) => tuple.elems.iter().any(|ty| chrono_type(ty, aliases)),
        _ => false,
    }
}

/// The names `file` gives chrono types: `type Stamp = DateTime<Utc>;` and
/// `use chrono::NaiveDate as Day;`, in its modules too, in order, so an alias
/// of an alias counts.
pub(super) fn aliases(file: &syn::File) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    collect_aliases(&file.items, &mut found);
    found
}

fn collect_aliases(items: &[syn::Item], found: &mut BTreeSet<String>) {
    for item in items {
        match item {
            syn::Item::Type(alias) if chrono_type(&alias.ty, found) => {
                found.insert(alias.ident.to_string());
            }
            syn::Item::Use(item) => renamed(&item.tree, false, found),
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    collect_aliases(items, found);
                }
            }
            _ => {}
        }
    }
}

/// `use … chrono … ::{NaiveDate as Day}`: renames of chrono's types under
/// a path that names chrono.
fn renamed(tree: &syn::UseTree, under_chrono: bool, found: &mut BTreeSet<String>) {
    match tree {
        syn::UseTree::Path(path) => {
            renamed(&path.tree, under_chrono || path.ident == "chrono", found)
        }
        syn::UseTree::Rename(rename)
            if under_chrono
                && (rename.ident == "DateTime"
                    || NAIVE_TYPES.iter().any(|name| rename.ident == name)) =>
        {
            found.insert(rename.rename.to_string());
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                renamed(item, under_chrono, found);
            }
        }
        _ => {}
    }
}

/// The rule for a path: Rullst's or SQLx's chrono re-export, a bare
/// `chrono::` that only Rullst's re-export can resolve, `RullstValue`'s
/// chrono variants, or `PasswordResetAt`/`EmailLoginAt` with `expires_at`
/// in the braces after it (`next`).
pub(super) fn path_rule(
    segments: &[String],
    next: Option<&TokenTree>,
    facts: &WorkspaceFacts,
) -> Option<&'static Rule> {
    let at = |index: usize, name: &str| segments.get(index).is_some_and(|segment| segment == name);
    let reexport = (0..segments.len()).any(|index| {
        let rullst = at(index, "rullst")
            && (at(index + 1, "orm") || at(index + 1, "db"))
            && at(index + 2, "chrono");
        let orm = at(index, "rullst_orm") && at(index + 1, "chrono");
        let sqlx = at(index, "sqlx") && at(index + 1, "types") && at(index + 2, "chrono");
        rullst || orm || (sqlx && !facts.sqlx_chrono)
    });
    let bare = at(0, "chrono") && !facts.chrono_dependency && !facts.sqlx_chrono;
    let variant = (0..segments.len()).any(|index| {
        at(index, "RullstValue")
            && segments
                .get(index + 1)
                .is_some_and(|variant| CHRONO_VARIANTS.contains(&variant.as_str()))
    });
    if reexport || bare || variant {
        return Some(&ORM_CHRONO_TYPES);
    }
    let notice = segments
        .last()
        .is_some_and(|last| last == "PasswordResetAt" || last == "EmailLoginAt");
    let expiry = matches!(next, Some(TokenTree::Group(fields))
        if fields.delimiter() == Delimiter::Brace && mentions(&fields.stream(), &["expires_at"]));
    (notice && expiry).then_some(&MAIL_JIFF_TIMESTAMPS)
}

/// The rule for a method call `.name(args)` on `chain`, given the file's
/// text, its [`ChronoBindings`] and the workspace facts.
pub(super) fn method_rule(
    name: &str,
    chain: &[String],
    args: &TokenStream,
    text: &str,
    bindings: &ChronoBindings,
    facts: &WorkspaceFacts,
) -> Option<&'static Rule> {
    match name {
        "format_date" | "format_time" | "format_datetime"
            if mentions(args, &["DateStyle", "TimeStyle"])
                && !bindings.jiff_argument(&args.clone().into_iter().collect::<Vec<_>>(), 0) =>
        {
            Some(&LOCALE_JIFF_DATES)
        }
        "send_at" | "with_scheduled_at" if names_mail(text) => Some(&MAIL_JIFF_TIMESTAMPS),
        // SQLx's own `bind` keeps working where the application enables
        // SQLx's `chrono` feature; `Orm::raw(..).bind(..)` does not.
        "bind" if facts.sqlx_chrono && !chain.iter().any(|link| link.contains("raw")) => None,
        _ if (name.starts_with("where_")
            || name.starts_with("or_where")
            || VALUE_METHODS.contains(&name))
            && bindings.carries_chrono(args) =>
        {
            Some(&ORM_CHRONO_TYPES)
        }
        _ => None,
    }
}

/// The rule for a call `path(args)`: `RullstValue::from` of a chrono value,
/// or Capital's `Invoice::new(id, email, date, currency)`.
pub(super) fn call_rule(
    segments: &[&str],
    args: &[TokenTree],
    text: &str,
    bindings: &ChronoBindings,
) -> Option<&'static Rule> {
    match segments {
        [.., "RullstValue", "from"] if bindings.carries_chrono(&args.iter().cloned().collect()) => {
            Some(&ORM_CHRONO_TYPES)
        }
        [.., "Invoice", "new"]
            if names_capital(text)
                && argument_count(args) == 4
                && !bindings.jiff_argument(args, 2) =>
        {
            Some(&CAPITAL_INVOICE_JIFF_DATE)
        }
        _ => None,
    }
}

/// The rule for a field read `.name`: Mail's `send_at`.
pub(super) fn field_rule(name: &str, text: &str) -> Option<&'static Rule> {
    (name == "send_at" && names_mail(text)).then_some(&MAIL_JIFF_TIMESTAMPS)
}

fn names_mail(text: &str) -> bool {
    text.contains("rullst_mail") || text.contains("mail::")
}

fn names_capital(text: &str) -> bool {
    text.contains("rullst_capital") || text.contains("capital::")
}

/// The number of comma-separated arguments, a trailing comma aside.
fn argument_count(args: &[TokenTree]) -> usize {
    args.split(|tree| matches!(tree, TokenTree::Punct(comma) if comma.as_char() == ','))
        .filter(|argument| !argument.is_empty())
        .count()
}

/// Whether `stream`, groups included, contains one of the identifiers.
fn mentions(stream: &TokenStream, names: &[&str]) -> bool {
    stream.clone().into_iter().any(|tree| match tree {
        TokenTree::Ident(ident) => names.iter().any(|name| ident == name),
        TokenTree::Group(group) => mentions(&group.stream(), names),
        _ => false,
    })
}
