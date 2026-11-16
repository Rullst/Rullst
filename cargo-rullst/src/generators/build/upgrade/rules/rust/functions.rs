//! Rules decided by a whole function body: password hashing in request code
//! without a bound (`V13-CREDENTIAL-RATE-LIMIT`) and an LMS
//! `record_progress` whose first SQL write is not the idempotency claim
//! (`V13-LMS-RECORD-PROGRESS`). Test functions and `#[cfg(test)]` modules are
//! skipped; the body's tokens are read with attributes left out, as in
//! [`super::super::tokens`].
//!
//! A call of the application's own `hash_password` (a path through `crate`,
//! `self`, `super` or a name the file binds from them or declares) is
//! reported only when no function of that name in the package bounds its
//! hashing: the package scan resolves it once every file is read.

use super::super::Collector;
use super::super::catalog::{CREDENTIAL_RATE_LIMIT, LMS_RECORD_PROGRESS};
use super::{FileContext, line, mentions};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;
use std::collections::BTreeSet;
use std::path::{Component, Path};
use syn::Attribute;
use syn::visit::{self, Visit};

/// Password hashing functions the credential rule bounds.
const HASHERS: &[&str] = &["hash_password", "hash_password_async"];
/// The helper the auth generator writes, which runs password work under one
/// of a few semaphore permits with a timeout.
const BOUNDED_HELPER: &str = "run_password_work";
/// Semaphore methods that take a permit, as a hand-written bound does.
const PERMITS: &[&str] = &[
    "acquire",
    "acquire_owned",
    "acquire_many",
    "acquire_many_owned",
    "try_acquire",
    "try_acquire_owned",
    "try_acquire_many",
    "try_acquire_many_owned",
];
/// Directories whose code serves no request: migrations and seeds that the
/// CLI runs, other binaries, tests, benchmarks and examples.
const OFFLINE_DIRECTORIES: &[&str] = &[
    "migrations",
    "seeds",
    "seeders",
    "bin",
    "tests",
    "benches",
    "examples",
];
/// The table whose insert claims a progress save's idempotency key.
const PROGRESS_EVENTS: &str = "LESSON_PROGRESS_EVENTS";

pub(super) fn scan(file: &syn::File, context: &FileContext<'_>, out: &mut Collector) {
    // The middleware's name in code (a definition or a route layer), never in
    // a comment, string or doc comment.
    let rate_limited = mentions(file.to_token_stream(), "credential_rate_limit");
    let mut names = Names::default();
    names.visit_file(file);
    let mut functions = Functions {
        hashing: serves_requests(context.path) && !rate_limited,
        names,
        out,
    };
    functions.visit_file(file);
}

/// The names a file binds from the application: `use crate::…`,
/// `use self::…`, `use super::…` (renames and `self` included), its `mod`
/// items and its own functions. A path that starts with another name reaches
/// an external crate, Rullst or a glob import, so it is not the
/// application's.
#[derive(Default)]
struct Names {
    application: BTreeSet<String>,
}

impl Names {
    /// Records `tree`, whose path so far ends at `last` and starts below
    /// `crate`, `self` or `super` when `local`.
    fn record(&mut self, tree: &syn::UseTree, local: bool, last: Option<&syn::Ident>) {
        // The first segment decides; later ones keep what it decided.
        let starts_local = |ident: &syn::Ident| match last {
            None => ident == "crate" || ident == "self" || ident == "super",
            Some(_) => local,
        };
        match tree {
            syn::UseTree::Path(path) => {
                self.record(&path.tree, starts_local(&path.ident), Some(&path.ident));
            }
            // `use crate::auth::{self};` binds `auth`.
            syn::UseTree::Name(name) if name.ident == "self" => {
                if let Some(last) = last.filter(|_| local) {
                    self.application.insert(last.to_string());
                }
            }
            syn::UseTree::Name(name) => {
                if last.is_some() && local {
                    self.application.insert(name.ident.to_string());
                }
            }
            syn::UseTree::Rename(rename) => {
                if last.is_some() && local {
                    self.application.insert(rename.rename.to_string());
                }
            }
            syn::UseTree::Glob(_) => {}
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.record(item, local, last);
                }
            }
        }
    }

    /// Whether a call whose path starts with `first` (the hasher's own name
    /// when unqualified) reaches the application's function.
    fn application(&self, first: &str) -> bool {
        matches!(first, "crate" | "self" | "super") || self.application.contains(first)
    }
}

impl<'ast> Visit<'ast> for Names {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.record(&item.tree, false, None);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.application.insert(item.ident.to_string());
        visit::visit_item_mod(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.application.insert(item.sig.ident.to_string());
        visit::visit_item_fn(self, item);
    }
}

/// Whether the file at `path` (relative to its package) can hold request
/// handlers: not `build.rs` and not below an [`OFFLINE_DIRECTORIES`] entry.
fn serves_requests(path: &Path) -> bool {
    let directories = path.parent().map(Path::components).into_iter().flatten();
    let offline = directories
        .filter_map(|component| match component {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .any(|name| OFFLINE_DIRECTORIES.contains(&name));
    !offline && path != Path::new("build.rs")
}

/// `#[test]`, `#[tokio::test]` and other `…::test` attributes, or `#[cfg(test)]`.
fn test_code(attributes: &[Attribute]) -> bool {
    super::super::test_code::is_test_attribute(attributes)
}

struct Functions<'a> {
    /// Whether this file's hashing calls need a bound.
    hashing: bool,
    names: Names,
    out: &'a mut Collector,
}

impl Functions<'_> {
    fn function(
        &mut self,
        attributes: &[Attribute],
        signature: &syn::Signature,
        body: &syn::Block,
    ) {
        if test_code(attributes) {
            return;
        }
        let tokens = body.to_token_stream();
        if let Some(name) = HASHERS.iter().find(|hasher| signature.ident == **hasher)
            && bounds_hashing(tokens.clone(), &self.names)
        {
            self.out.add_bounded(name);
        }
        if self.hashing && !acquires_permit(tokens.clone()) {
            for (at, callee) in unbounded_hashes(tokens.clone(), &self.names) {
                match callee {
                    Some(name) => self.out.add_call(at, name),
                    None => self.out.add(&CREDENTIAL_RATE_LIMIT, at),
                }
            }
        }
        if signature.ident == "record_progress" && writes_before_claiming(tokens) {
            self.out
                .add(&LMS_RECORD_PROGRESS, line(signature.ident.span()));
        }
    }
}

impl<'ast> Visit<'ast> for Functions<'_> {
    fn visit_attribute(&mut self, _attribute: &'ast Attribute) {}

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if !test_code(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if !test_code(&item.attrs) {
            visit::visit_item_impl(self, item);
        }
    }

    // A body is read as a whole, nested items included, so the visit stops
    // at the function.
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.function(&item.attrs, &item.sig, &item.block);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.function(&item.attrs, &item.sig, &item.block);
    }
}

/// The tokens of `stream` without `#[...]` and `#![...]` attributes.
fn without_attributes(stream: TokenStream) -> Vec<TokenTree> {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut kept = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        if let TokenTree::Punct(punct) = &tokens[index]
            && punct.as_char() == '#'
        {
            let mut next = index + 1;
            if matches!(tokens.get(next), Some(TokenTree::Punct(bang)) if bang.as_char() == '!') {
                next += 1;
            }
            if matches!(tokens.get(next), Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Bracket)
            {
                index = next + 1;
                continue;
            }
        }
        kept.push(tokens[index].clone());
        index += 1;
    }
    kept
}

fn arguments(tree: Option<&TokenTree>) -> bool {
    matches!(tree, Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis)
}

fn punct(tree: Option<&TokenTree>, character: char) -> bool {
    matches!(tree, Some(TokenTree::Punct(punct)) if punct.as_char() == character)
}

/// The first segment of the path that ends at `tokens[index]`, or `None`
/// for a method call (`.hash_password(…)`) or a qualified type
/// (`<T as Hasher>::hash_password(…)`), which never name the application's
/// free function.
fn path_start(tokens: &[TokenTree], index: usize) -> Option<String> {
    let before = |offset: usize| index.checked_sub(offset).and_then(|at| tokens.get(at));
    if punct(before(1), '.') {
        return None;
    }
    let mut start = index;
    while start >= 2 && punct(tokens.get(start - 1), ':') && punct(tokens.get(start - 2), ':') {
        match start.checked_sub(3).and_then(|at| tokens.get(at)) {
            Some(TokenTree::Ident(_)) => start -= 3,
            Some(TokenTree::Punct(close)) if close.as_char() == '>' => return None,
            // A leading `::`, as in `::rullst::auth::hash_password`.
            _ => break,
        }
    }
    match &tokens[start] {
        TokenTree::Ident(first) => Some(first.to_string()),
        _ => None,
    }
}

/// Every hashing call that does not run inside the arguments of
/// [`BOUNDED_HELPER`]: its line, and the hasher's name when the call
/// reaches the application's own function rather than Rullst's.
fn unbounded_hashes(stream: TokenStream, names: &Names) -> Vec<(usize, Option<&'static str>)> {
    let tokens = without_attributes(stream);
    let mut found = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        match &tokens[index] {
            TokenTree::Ident(ident) if arguments(tokens.get(index + 1)) => {
                if ident == BOUNDED_HELPER {
                    index += 2;
                    continue;
                }
                let defined = index
                    .checked_sub(1)
                    .is_some_and(|before| matches!(&tokens[before], TokenTree::Ident(keyword) if keyword == "fn"));
                if let Some(hasher) = HASHERS.iter().find(|hasher| ident == **hasher)
                    && !defined
                {
                    let application =
                        path_start(&tokens, index).is_some_and(|first| names.application(&first));
                    found.push((ident.span().start().line, application.then_some(*hasher)));
                }
            }
            TokenTree::Group(group) => found.extend(unbounded_hashes(group.stream(), names)),
            _ => {}
        }
        index += 1;
    }
    found
}

/// A call of [`BOUNDED_HELPER`] anywhere in `stream`.
fn calls_helper(stream: TokenStream) -> bool {
    let tokens = without_attributes(stream);
    tokens.iter().enumerate().any(|(index, tree)| match tree {
        TokenTree::Group(group) => calls_helper(group.stream()),
        TokenTree::Ident(ident) => ident == BOUNDED_HELPER && arguments(tokens.get(index + 1)),
        _ => false,
    })
}

/// Whether a function body bounds the hashing it does: it takes a semaphore
/// permit, or it calls [`BOUNDED_HELPER`] and hashes nothing outside it.
fn bounds_hashing(stream: TokenStream, names: &Names) -> bool {
    acquires_permit(stream.clone())
        || (calls_helper(stream.clone()) && unbounded_hashes(stream, names).is_empty())
}

/// A `.acquire()`-style call anywhere in `stream`.
fn acquires_permit(stream: TokenStream) -> bool {
    let tokens = without_attributes(stream);
    tokens.iter().enumerate().any(|(index, tree)| match tree {
        TokenTree::Group(group) => acquires_permit(group.stream()),
        TokenTree::Ident(ident) => PERMITS.iter().any(|method| ident == method)
            && index.checked_sub(1).is_some_and(
                |before| matches!(&tokens[before], TokenTree::Punct(dot) if dot.as_char() == '.'),
            )
            && arguments(tokens.get(index + 1)),
        _ => false,
    })
}

/// Every string literal of `stream`, in source order.
fn strings(stream: TokenStream, found: &mut Vec<String>) {
    for tree in without_attributes(stream) {
        match tree {
            TokenTree::Group(group) => strings(group.stream(), found),
            TokenTree::Literal(literal) => {
                if let syn::Lit::Str(text) = syn::Lit::new(literal) {
                    found.push(text.value());
                }
            }
            _ => {}
        }
    }
}

/// `sql` in upper case with quotes removed and whitespace collapsed.
fn normalized(sql: &str) -> String {
    sql.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(['"', '`'], "")
        .to_ascii_uppercase()
}

/// Whether a body that keeps progress events writes something else before
/// the event insert that claims the key, as the 12.x LMS service did (it
/// read the stored percentage, upserted it and only then inserted the
/// event, so a double-submitted save failed instead of replaying).
fn writes_before_claiming(stream: TokenStream) -> bool {
    let mut found = Vec::new();
    strings(stream, &mut found);
    let statements = found.iter().map(|sql| normalized(sql)).collect::<Vec<_>>();
    if !statements.iter().any(|sql| sql.contains(PROGRESS_EVENTS)) {
        return false;
    }
    let first_write = statements.iter().find(|sql| {
        ["INSERT ", "UPDATE ", "DELETE ", "REPLACE "]
            .iter()
            .any(|verb| sql.starts_with(verb))
    });
    let claim = format!("INSERT INTO {PROGRESS_EVENTS}");
    first_write.is_some_and(|sql| {
        !sql.strip_prefix(&claim)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '(']))
    })
}
