//! Which expressions of a file hold chrono values, for
//! `V13-ORM-CHRONO-TYPES`: the names the file binds to one (`let` bindings
//! and parameters of a chrono type, and its functions that return one), and
//! the SQLx reads that decode one. Scopes are not tracked, so a name counts
//! in the whole file.
//!
//! An expression holds a chrono value when it starts with one (a chrono
//! path such as `Utc::now()`, a bound name, a call of a function of the file
//! that returns one) and no later method turns it into something else
//! (`.to_string()`, `.format(..)`, `.timestamp()`) or reads a field. A struct
//! or a call that merely takes a chrono value does not hold one.
//!
//! The same bindings record the jiff values of the file, so that
//! `V13-LOCALE-JIFF-DATES` and `V13-CAPITAL-INVOICE-JIFF-DATE` stay silent
//! for code that already passes one ([`ChronoBindings::holds_jiff`]): a
//! `jiff::`, `civil::`, `Zoned` or `Timestamp` path, Rullst's ORM types
//! (`rullst::orm::Timestamp`), a name, field or function of such a type, and
//! a chain through a method only jiff has (`.in_tz(..)`); and a read of a
//! field the file does not declare as chrono ([`ChronoBindings::jiff_argument`]).

use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use std::collections::BTreeSet;

#[path = "jiff_values.rs"]
mod jiff_values;
use jiff_values::{JiffBindings, is_jiff_type};

/// Identifiers that only chrono defines among the date libraries an
/// application uses with Rullst (jiff's UTC is `TimeZone::UTC`). `Local`
/// counts only in `Local::now`, since applications name variants `Local`.
const CHRONO_NAMES: [&str; 7] = [
    "chrono",
    "Utc",
    "FixedOffset",
    "TimeDelta",
    "NaiveDateTime",
    "NaiveDate",
    "NaiveTime",
];

/// chrono's `DateTime` constructors; `DateTime::` alone may be jiff's
/// civil type or Rullst's.
const CHRONO_CONSTRUCTORS: [&str; 5] = [
    "from_timestamp",
    "from_timestamp_millis",
    "parse_from_rfc3339",
    "parse_from_rfc2822",
    "from_naive_utc_and_offset",
];

/// Methods whose result is no longer a chrono value.
const PLAIN_RESULTS: [&str; 30] = [
    "to_string",
    "to_rfc3339",
    "to_rfc3339_opts",
    "to_rfc2822",
    "format",
    "timestamp",
    "timestamp_millis",
    "timestamp_micros",
    "timestamp_nanos_opt",
    "num_days",
    "num_hours",
    "num_minutes",
    "num_seconds",
    "num_milliseconds",
    "num_weeks",
    "year",
    "month",
    "month0",
    "day",
    "day0",
    "hour",
    "minute",
    "second",
    "ordinal",
    "weekday",
    "iso_week",
    "num_days_from_ce",
    "num_days_from_monday",
    "num_days_from_sunday",
    "is_some",
];

/// SQLx reads whose turbofish names the decoded type.
const TURBOFISH_DECODERS: [&str; 4] = ["query_scalar", "query_as", "get", "try_get"];

/// SQLx queries whose decoded type a `let` annotation gives.
const ANNOTATED_DECODERS: [&str; 2] = ["query_scalar", "query_as"];

fn is_punct(tree: Option<&TokenTree>, expected: char) -> bool {
    matches!(tree, Some(TokenTree::Punct(punct)) if punct.as_char() == expected)
}

fn is_ident(tree: Option<&TokenTree>, expected: &str) -> bool {
    matches!(tree, Some(TokenTree::Ident(ident)) if ident == expected)
}

/// `tokens` split at their top-level commas, empty parts dropped.
fn elements(tokens: &[TokenTree]) -> impl Iterator<Item = &[TokenTree]> {
    tokens
        .split(|tree| matches!(tree, TokenTree::Punct(comma) if comma.as_char() == ','))
        .filter(|element| !element.is_empty())
}

fn collect(stream: TokenStream) -> Vec<TokenTree> {
    stream.into_iter().collect()
}

/// Whether the type `tokens` names chrono: one of chrono's own names, or
/// `DateTime<…>` (Rullst's `DateTime` takes no argument).
fn is_chrono_type(tokens: &[TokenTree]) -> bool {
    tokens.iter().enumerate().any(|(index, tree)| match tree {
        TokenTree::Ident(ident) => {
            CHRONO_NAMES.iter().any(|name| ident == name)
                || (ident == "DateTime" && is_punct(tokens.get(index + 1), '<'))
        }
        TokenTree::Group(group) => is_chrono_type(&collect(group.stream())),
        _ => false,
    })
}

/// The index of the `=` of a `let` statement, if any: an alone `=` that does
/// not end `==`, `<=`, `>=`, `!=` or `+=`.
fn assignment(tokens: &[TokenTree]) -> Option<usize> {
    (0..tokens.len()).find(|index| {
        let joined_before = index.checked_sub(1).is_some_and(
            |before| matches!(&tokens[before], TokenTree::Punct(p) if p.spacing() == Spacing::Joint),
        );
        matches!(&tokens[*index], TokenTree::Punct(p) if p.as_char() == '=' && p.spacing() == Spacing::Alone)
            && !joined_before
    })
}

/// The names of the comma-separated `name: Type` entries of `tokens`
/// (parameters or struct fields, attributes and `pub` allowed) whose type
/// names chrono.
fn chrono_typed(tokens: &[TokenTree]) -> Vec<String> {
    typed(tokens, is_chrono_type)
}

/// The names of the `name: Type` entries of `tokens` whose type `kind`
/// accepts.
fn typed(tokens: &[TokenTree], kind: fn(&[TokenTree]) -> bool) -> Vec<String> {
    elements(tokens)
        .filter_map(|entry| {
            let colon = entry.iter().position(|tree| {
                matches!(tree, TokenTree::Punct(colon) if colon.as_char() == ':' && colon.spacing() == Spacing::Alone)
            })?;
            match colon.checked_sub(1).and_then(|at| entry.get(at)) {
                Some(TokenTree::Ident(name)) if kind(&entry[colon + 1..]) => Some(name.to_string()),
                _ => None,
            }
        })
        .collect()
}

/// What a file binds to chrono values, and its SQLx reads that decode one;
/// and what it binds to jiff values.
#[derive(Debug, Default)]
pub(in super::super) struct ChronoBindings {
    names: BTreeSet<String>,
    functions: BTreeSet<String>,
    /// Named fields of the file's structs that have a chrono type.
    fields: BTreeSet<String>,
    decodes: Vec<usize>,
    /// The names, functions and struct fields of jiff values.
    jiff: JiffBindings,
}

impl ChronoBindings {
    pub(in super::super) fn of(stream: TokenStream) -> Self {
        let mut bindings = Self::default();
        bindings.declarations(stream.clone());
        bindings.read(stream);
        bindings
    }

    /// The lines of SQLx reads that decode a chrono type: a turbofish
    /// (`query_scalar::<_, NaiveDate>`, `try_get::<DateTime<Utc>, _>`) or a
    /// `let` of a chrono type assigned from `query_scalar`/`query_as`.
    pub(in super::super) fn decodes(&self) -> &[usize] {
        &self.decodes
    }

    /// The file's functions whose return type names chrono, and the named
    /// struct fields that have a chrono type.
    fn declarations(&mut self, stream: TokenStream) {
        let tokens = collect(stream);
        for (index, tree) in tokens.iter().enumerate() {
            match tree {
                TokenTree::Ident(ident) if ident == "fn" => {
                    let Some(TokenTree::Ident(name)) = tokens.get(index + 1) else {
                        continue;
                    };
                    let rest = &tokens[index + 2..];
                    let body = rest
                        .iter()
                        .position(|tree| {
                            matches!(tree, TokenTree::Group(g) if g.delimiter() == Delimiter::Brace)
                                || matches!(tree, TokenTree::Punct(p) if p.as_char() == ';')
                                || matches!(tree, TokenTree::Ident(i) if i == "where")
                        })
                        .unwrap_or(rest.len());
                    let arrow = (0..body)
                        .find(|at| is_punct(rest.get(*at), '-') && is_punct(rest.get(at + 1), '>'));
                    if let Some(arrow) = arrow {
                        let returned = &rest[arrow + 2..body];
                        if is_chrono_type(returned) {
                            self.functions.insert(name.to_string());
                        } else if is_jiff_type(returned) {
                            self.jiff.functions.insert(name.to_string());
                        }
                    }
                }
                TokenTree::Ident(ident) if ident == "struct" => {
                    // `struct Name<..> { .. }`; a tuple or unit struct ends
                    // at its `;` first.
                    let body = tokens[index + 1..]
                        .iter()
                        .take_while(
                            |tree| !matches!(tree, TokenTree::Punct(p) if p.as_char() == ';'),
                        )
                        .find_map(|tree| match tree {
                            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                                Some(group.stream())
                            }
                            _ => None,
                        });
                    if let Some(body) = body {
                        let body = collect(body);
                        self.fields.extend(chrono_typed(&body));
                        self.jiff.fields.extend(typed(&body, is_jiff_type));
                    }
                }
                TokenTree::Group(group) => self.declarations(group.stream()),
                _ => {}
            }
        }
    }

    fn read(&mut self, stream: TokenStream) {
        let tokens = collect(stream);
        for (index, tree) in tokens.iter().enumerate() {
            match tree {
                TokenTree::Ident(ident) if ident == "let" => self.binding(&tokens, index + 1),
                TokenTree::Ident(ident) if ident == "fn" => self.parameters(&tokens, index + 1),
                TokenTree::Ident(ident)
                    if TURBOFISH_DECODERS.iter().any(|name| ident == name)
                        && is_punct(tokens.get(index + 1), ':')
                        && is_punct(tokens.get(index + 3), '<') =>
                {
                    let end = (index + 4..tokens.len())
                        .find(|at| {
                            matches!(&tokens[*at], TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis)
                        })
                        .unwrap_or(tokens.len());
                    if is_chrono_type(&tokens[index + 4..end]) {
                        self.decodes.push(ident.span().start().line);
                    }
                }
                TokenTree::Group(group) => self.read(group.stream()),
                _ => {}
            }
        }
    }

    /// `let [mut] name[: Type] = value;` from `start`, after `let`. A
    /// pattern (`let Some(x) = …`, `if let …`) binds no name here.
    fn binding(&mut self, tokens: &[TokenTree], start: usize) {
        let index = start + usize::from(is_ident(tokens.get(start), "mut"));
        let (Some(TokenTree::Ident(name)), Some(TokenTree::Punct(after))) =
            (tokens.get(index), tokens.get(index + 1))
        else {
            return;
        };
        if !matches!(after.as_char(), ':' | '=') {
            return;
        }
        let end = (index..tokens.len())
            .find(|at| is_punct(tokens.get(*at), ';'))
            .unwrap_or(tokens.len());
        let statement = &tokens[index + 1..end];
        let Some(equals) = assignment(statement) else {
            return;
        };
        let value = &statement[equals + 1..];
        let (chrono, jiff) = if after.as_char() == ':' {
            let annotation = &statement[1..equals];
            let annotated = is_chrono_type(annotation);
            let decoded = value.iter().any(|tree| {
                matches!(tree, TokenTree::Ident(i) if ANNOTATED_DECODERS.iter().any(|name| i == name))
            });
            if annotated && decoded {
                self.decodes.push(name.span().start().line);
            }
            (annotated, is_jiff_type(annotation))
        } else {
            (self.is_chrono_value(value), self.holds_jiff(value))
        };
        if chrono {
            self.names.insert(name.to_string());
        } else if jiff {
            self.jiff.names.insert(name.to_string());
        }
    }

    /// The parameters of `fn name<..>(params)`, from the token after `fn`.
    fn parameters(&mut self, tokens: &[TokenTree], start: usize) {
        let Some(TokenTree::Group(params)) = tokens[start..].iter().find(|tree| {
            matches!(tree, TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis)
        }) else {
            return;
        };
        let params = collect(params.stream());
        self.names.extend(chrono_typed(&params));
        self.jiff.names.extend(typed(&params, is_jiff_type));
    }

    /// Whether one of the comma-separated expressions of `stream` (a call's
    /// arguments) holds a chrono value.
    pub(in super::super) fn carries_chrono(&self, stream: &TokenStream) -> bool {
        elements(&collect(stream.clone())).any(|element| self.is_chrono_value(element))
    }

    /// Whether the expression `tokens` holds a chrono value (see the module
    /// documentation).
    fn is_chrono_value(&self, tokens: &[TokenTree]) -> bool {
        let start = tokens
            .iter()
            .position(|tree| {
                !matches!(tree, TokenTree::Punct(p) if matches!(p.as_char(), '&' | '*'))
                    && !matches!(tree, TokenTree::Ident(i) if i == "mut")
            })
            .unwrap_or(tokens.len());
        let tokens = &tokens[start..];
        let (head, after) = match tokens.first() {
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket => {
                return elements(&collect(group.stream())).any(|e| self.is_chrono_value(e));
            }
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                let inner = collect(group.stream());
                let tuple = elements(&inner).count() > 1;
                if tuple {
                    return elements(&inner).any(|e| self.is_chrono_value(e));
                }
                (self.is_chrono_value(&inner), 1)
            }
            Some(TokenTree::Ident(_)) => self.chrono_head(tokens),
            _ => return false,
        };
        let chain = &tokens[after.min(tokens.len())..];
        let mut head = head;
        let mut parsed_chrono = false;
        for index in 0..chain.len() {
            if !is_punct(chain.get(index), '.') {
                continue;
            }
            let Some(TokenTree::Ident(method)) = chain.get(index + 1) else {
                continue;
            };
            if method == "await" {
                continue;
            }
            let turbofish =
                is_punct(chain.get(index + 2), ':') && is_punct(chain.get(index + 4), '<');
            let called = turbofish
                || matches!(chain.get(index + 2), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis);
            if !called {
                // A field read: chrono's types have none, so only a field of
                // a struct of the file that has a chrono type holds one.
                head = self.fields.contains(&method.to_string());
                if !head {
                    return false;
                }
                continue;
            }
            if PLAIN_RESULTS.iter().any(|name| method == name) {
                return false;
            }
            if method == "parse" && turbofish {
                let end = (index + 5..chain.len())
                    .find(|at| matches!(&chain[*at], TokenTree::Group(_)))
                    .unwrap_or(chain.len());
                parsed_chrono = is_chrono_type(&chain[index + 5..end]);
            }
        }
        head || parsed_chrono
    }

    /// Whether the path at the start of `tokens` starts a chrono value, and
    /// the index after it: a chrono path (`Utc::now`, `NaiveDate::…`,
    /// `chrono::…`, `DateTime::<Tz>::…` or a constructor of chrono's), a name
    /// bound to one, or a call of a function of the file that returns one.
    /// `vec![…]` holds one when an element does; other macros do not.
    fn chrono_head(&self, tokens: &[TokenTree]) -> (bool, usize) {
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
        if is_punct(tokens.get(end), '!') {
            let vec = segments == ["vec"];
            return match tokens.get(end + 1) {
                Some(TokenTree::Group(body)) if vec => (
                    elements(&collect(body.stream())).any(|e| self.is_chrono_value(e)),
                    end + 2,
                ),
                _ => (false, end),
            };
        }
        let first = segments.first().map(String::as_str).unwrap_or_default();
        let generic = is_punct(tokens.get(end), '<')
            || (is_punct(tokens.get(end), ':') && is_punct(tokens.get(end + 2), '<'));
        let called = matches!(tokens.get(end), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis);
        let chrono = segments.iter().any(|segment| segment == "chrono")
            || CHRONO_NAMES.contains(&first)
            || (first == "Local" && segments.get(1).is_some_and(|next| next == "now"))
            || (first == "DateTime"
                && (generic
                    || segments
                        .get(1)
                        .is_some_and(|next| CHRONO_CONSTRUCTORS.contains(&next.as_str()))))
            || (segments.len() == 1 && self.names.contains(first))
            || (called
                && segments
                    .last()
                    .is_some_and(|last| self.functions.contains(last)));
        (chrono, end)
    }
}
