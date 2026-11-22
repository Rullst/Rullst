//! The jiff values of a file, which `V13-LOCALE-JIFF-DATES` and
//! `V13-CAPITAL-INVOICE-JIFF-DATE` leave alone: the names, functions and
//! struct fields of [`ChronoBindings`] that hold one, and the expressions
//! that start with or reach one.

use super::{ChronoBindings, collect, elements, is_chrono_type, is_punct};
use proc_macro2::{Delimiter, TokenTree};
use std::collections::BTreeSet;

/// Identifiers that only name jiff's types and paths among the date
/// libraries here: chrono has no `Timestamp` or `Zoned` type.
const JIFF_NAMES: [&str; 4] = ["jiff", "civil", "Zoned", "Timestamp"];

/// The names of Rullst's ORM date types, built on jiff, and of jiff's civil
/// types; chrono's `DateTime` always takes a time zone argument.
const JIFF_TYPES: [&str; 4] = ["Timestamp", "DateTime", "Date", "Time"];

/// The paths of Rullst's ORM, whose `Timestamp`, `DateTime`, `Date` and
/// `Time` are jiff values.
const ORM_PATHS: [&str; 3] = ["orm", "rullst_orm", "db"];

/// Methods that only jiff's values have, so a chain through one holds a jiff
/// value (`at.in_tz("America/Sao_Paulo")?`).
const JIFF_METHODS: [&str; 3] = ["in_tz", "to_zoned", "to_zoned_datetime"];

/// Whether the type `tokens` is a jiff value's: a type that does not name
/// chrono and names jiff, `civil`, `Zoned`, `Timestamp`, or one of
/// [`JIFF_TYPES`] without a type argument (`rullst::orm::Date`).
pub(super) fn is_jiff_type(tokens: &[TokenTree]) -> bool {
    fn names(tokens: &[TokenTree]) -> bool {
        tokens.iter().enumerate().any(|(index, tree)| match tree {
            TokenTree::Ident(ident) => {
                JIFF_NAMES.iter().any(|name| ident == name)
                    || (JIFF_TYPES.iter().any(|name| ident == name)
                        && !is_punct(tokens.get(index + 1), '<'))
            }
            TokenTree::Group(group) => names(&collect(group.stream())),
            _ => false,
        })
    }
    !is_chrono_type(tokens) && names(tokens)
}

/// Whether the expression `tokens` only reads a field: `name.field`, more
/// fields after it, a leading `&` or `*`, and no call.
fn field_read(tokens: &[TokenTree]) -> bool {
    let start = tokens
        .iter()
        .position(|tree| !matches!(tree, TokenTree::Punct(p) if matches!(p.as_char(), '&' | '*')))
        .unwrap_or(tokens.len());
    let tokens = &tokens[start..];
    tokens.len() >= 3
        && tokens.iter().enumerate().all(|(index, tree)| match tree {
            TokenTree::Ident(_) => index % 2 == 0,
            TokenTree::Punct(dot) => index % 2 == 1 && dot.as_char() == '.',
            _ => false,
        })
        && tokens.len() % 2 == 1
}

/// The names a file binds to jiff values, its functions that return one and
/// the fields of its structs that hold one.
#[derive(Debug, Default)]
pub(super) struct JiffBindings {
    pub(super) names: BTreeSet<String>,
    pub(super) functions: BTreeSet<String>,
    pub(super) fields: BTreeSet<String>,
}

impl ChronoBindings {
    /// Whether the argument at `index` of `args` (a call's arguments) needs
    /// no change for an API that takes jiff values: it holds a jiff value
    /// and no chrono value, or it reads a field the file does not declare
    /// with a chrono type (`payment.paid_at`). Such a field belongs to a
    /// struct declared elsewhere; a model's chrono field is reported there
    /// (`V13-ORM-CHRONO-TYPES`), and the read compiles unchanged once the
    /// field is a Rullst ORM type. A chrono field of another file's struct
    /// that is not a model is reported by no rule; the compiler reports the
    /// call.
    pub(in super::super::super) fn jiff_argument(&self, args: &[TokenTree], index: usize) -> bool {
        elements(args).nth(index).is_some_and(|argument| {
            self.holds_jiff(argument) || (field_read(argument) && !self.is_chrono_value(argument))
        })
    }

    /// Whether the expression `tokens` holds a jiff value (see the module
    /// documentation) and no chrono value.
    pub(in super::super::super) fn holds_jiff(&self, tokens: &[TokenTree]) -> bool {
        !self.is_chrono_value(tokens) && self.is_jiff_value(tokens)
    }

    /// Whether the expression `tokens` starts with a jiff value, or reaches
    /// one through a field of the file's structs or a jiff-only method.
    fn is_jiff_value(&self, tokens: &[TokenTree]) -> bool {
        let start = tokens
            .iter()
            .position(|tree| {
                !matches!(tree, TokenTree::Punct(p) if matches!(p.as_char(), '&' | '*'))
                    && !matches!(tree, TokenTree::Ident(i) if i == "mut")
            })
            .unwrap_or(tokens.len());
        let tokens = &tokens[start..];
        let (mut head, after) = match tokens.first() {
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                (self.is_jiff_value(&collect(group.stream())), 1)
            }
            Some(TokenTree::Ident(_)) => self.jiff_head(tokens),
            _ => return false,
        };
        let chain = &tokens[after.min(tokens.len())..];
        for index in 0..chain.len() {
            if !is_punct(chain.get(index), '.') {
                continue;
            }
            let Some(TokenTree::Ident(member)) = chain.get(index + 1) else {
                continue;
            };
            let called = matches!(chain.get(index + 2), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis);
            if JIFF_METHODS.iter().any(|name| member == name) {
                return true;
            }
            if !called && member != "await" {
                head = self.jiff.fields.contains(&member.to_string());
            }
        }
        head
    }

    /// Whether the path at the start of `tokens` starts a jiff value, and the
    /// index after it: a path through jiff or Rullst's ORM types, a name
    /// bound to one, or a call of a function of the file that returns one.
    fn jiff_head(&self, tokens: &[TokenTree]) -> (bool, usize) {
        let mut segments = Vec::new();
        let mut end = 0;
        while let Some(TokenTree::Ident(ident)) = tokens.get(end) {
            segments.push(ident.to_string());
            if is_punct(tokens.get(end + 1), ':')
                && matches!(tokens.get(end + 3), Some(TokenTree::Ident(_)))
            {
                end += 3;
            } else {
                end += 1;
                break;
            }
        }
        let named = |names: &[&str]| {
            segments
                .iter()
                .any(|segment| names.contains(&segment.as_str()))
        };
        let called = matches!(tokens.get(end), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis);
        let first = segments.first().map(String::as_str).unwrap_or_default();
        let jiff = named(&JIFF_NAMES)
            || (named(&ORM_PATHS) && named(&JIFF_TYPES))
            || (segments.len() == 1 && self.jiff.names.contains(first))
            || (called
                && segments
                    .last()
                    .is_some_and(|last| self.jiff.functions.contains(last)));
        (jiff, end)
    }
}
