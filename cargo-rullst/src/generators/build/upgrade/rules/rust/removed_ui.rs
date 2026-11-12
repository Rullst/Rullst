//! The browser UI stacks 13.0.0-alpha.5 removed: Wasm islands
//! (`V13-ISLANDS-REMOVED`), Live components (`V13-LIVE-REMOVED`), typed
//! server functions (`V13-SERVER-FUNCTIONS-REMOVED`) and the frontend
//! adapters (`V13-FRONTEND-ADAPTERS-REMOVED`). The token stream locates their
//! attributes (`#[island]`, `#[live_component]`, `#[live_event]`,
//! `#[server_function]`, however qualified) and every path through the
//! framework (`rullst::live::…`, `rullst_core::island`, `rullst::rpc`,
//! `rullst::client_contract`, `rullst_core::frontend`, `use rullst::{live, …}`),
//! in macro bodies too. Literals, so doc comments, never match.

use super::super::catalog::{
    FRONTEND_ADAPTERS_REMOVED, ISLANDS_REMOVED, LIVE_REMOVED, SERVER_FUNCTIONS_REMOVED,
};
use super::super::{Collector, Rule};
use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

/// The rule of a removed name, as an attribute's last segment or as the
/// segment after `rullst::`/`rullst_core::`.
fn removed(name: &str) -> Option<&'static Rule> {
    match name {
        "island" => Some(&ISLANDS_REMOVED),
        "live" | "live_component" | "live_event" => Some(&LIVE_REMOVED),
        "server_function" | "rpc" | "client" | "client_contract" => Some(&SERVER_FUNCTIONS_REMOVED),
        "frontend" => Some(&FRONTEND_ADAPTERS_REMOVED),
        _ => None,
    }
}

/// The rule of a removed attribute, by the last segment of its path. Module
/// names are not attributes: `#[rpc(server)]` belongs to other crates.
fn removed_attribute(name: &str) -> Option<&'static Rule> {
    match name {
        "rpc" | "client" | "client_contract" | "frontend" => None,
        _ => removed(name),
    }
}

/// The crates whose paths reach the removed modules and macros.
fn is_framework(tree: &TokenTree) -> bool {
    matches!(tree, TokenTree::Ident(ident) if ident == "rullst" || ident == "rullst_core")
}

fn is_punct(tree: Option<&TokenTree>, character: char) -> bool {
    matches!(tree, Some(TokenTree::Punct(punct)) if punct.as_char() == character)
}

/// Whether `tokens[index..]` starts with `::`.
fn path_separator_at(tokens: &[TokenTree], index: usize) -> bool {
    matches!(tokens.get(index), Some(TokenTree::Punct(punct))
        if punct.as_char() == ':' && punct.spacing() == Spacing::Joint)
        && is_punct(tokens.get(index + 1), ':')
}

/// Scans a file's tokens.
pub(super) fn scan(stream: TokenStream, out: &mut Collector) {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut index = 0;
    while index < tokens.len() {
        let tree = &tokens[index];
        if is_punct(Some(tree), '#') {
            // `#[...]` or `#![...]`: only its path counts, never its arguments.
            let bracket = index + 1 + usize::from(is_punct(tokens.get(index + 1), '!'));
            if let Some(TokenTree::Group(group)) = tokens.get(bracket)
                && group.delimiter() == Delimiter::Bracket
            {
                attribute(group.stream(), out);
                index = bracket + 1;
                continue;
            }
        }
        match tree {
            TokenTree::Group(group) => scan(group.stream(), out),
            _ if is_framework(tree) && path_separator_at(&tokens, index + 1) => {
                match tokens.get(index + 3) {
                    Some(TokenTree::Ident(segment)) => {
                        if let Some(rule) = removed(&segment.to_string()) {
                            out.add(rule, segment.span().start().line);
                        }
                    }
                    Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => {
                        use_group(group.stream(), out);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        index += 1;
    }
}

/// The attribute `#[a::b::name(...)]`, by the last segment of its path.
fn attribute(stream: TokenStream, out: &mut Collector) {
    let mut last = None;
    for tree in stream {
        match tree {
            TokenTree::Ident(ident) => last = Some(ident),
            TokenTree::Punct(punct) if punct.as_char() == ':' => {}
            _ => break,
        }
    }
    if let Some(ident) = last
        && let Some(rule) = removed_attribute(&ident.to_string())
    {
        out.add(rule, ident.span().start().line);
    }
}

/// The first segment of each item of `use rullst::{a, b::c, …}`.
fn use_group(stream: TokenStream, out: &mut Collector) {
    let mut first = true;
    for tree in stream {
        match &tree {
            TokenTree::Punct(punct) if punct.as_char() == ',' => first = true,
            TokenTree::Ident(ident) if first => {
                first = false;
                if let Some(rule) = removed(&ident.to_string()) {
                    out.add(rule, ident.span().start().line);
                }
            }
            _ => first = false,
        }
    }
}
