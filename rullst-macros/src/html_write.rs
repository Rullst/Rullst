//! The statements of an `html!` expansion. Every node writes into one
//! `String`: text known at compile time is joined into one `push_str` per
//! run, and interpolated values are escaped into the same buffer, so a page
//! allocates its buffer once instead of a `String` per element.
//!
//! `continue` and `break` inside an `@for` body leave the expansion's loop in
//! the middle of a node. Each node used to build its own `String`, so the
//! node they cut short wrote nothing; a top-level body node that mentions
//! either keyword still writes into a buffer of its own, which is appended
//! only when the node completes.

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{Expr, Ident};

use super::HtmlNode;

/// Collects the statements that write into one buffer.
pub(super) struct Writer {
    buffer: Ident,
    /// How many node buffers enclose this one (see [`write_loop_body`]).
    depth: usize,
    statements: TokenStream,
    text: String,
}

impl Writer {
    pub(super) fn new(buffer: Ident, depth: usize) -> Self {
        Self {
            buffer,
            depth,
            statements: TokenStream::new(),
            text: String::new(),
        }
    }

    /// A writer for a block body (`@if`, `@for` or `@match`) that writes
    /// into the same buffer.
    pub(super) fn body(&self) -> Self {
        Self::new(self.buffer.clone(), self.depth)
    }

    pub(super) fn buffer(&self) -> &Ident {
        &self.buffer
    }

    /// Appends text known at compile time.
    pub(super) fn text(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// Appends statements, after the text written before them.
    pub(super) fn code(&mut self, tokens: TokenStream) {
        self.flush();
        self.statements.extend(tokens);
    }

    fn flush(&mut self) {
        if !self.text.is_empty() {
            let text = std::mem::take(&mut self.text);
            let buffer = &self.buffer;
            self.statements.extend(quote! { #buffer.push_str(#text); });
        }
    }

    pub(super) fn finish(mut self) -> TokenStream {
        self.flush();
        self.statements
    }
}

/// Local items the expansion needs once, whichever elements use them.
#[derive(Default)]
pub(super) struct Helpers {
    pub(super) url_guard: bool,
    pub(super) option_probe: bool,
}

impl Helpers {
    pub(super) fn to_tokens(&self) -> TokenStream {
        let mut tokens = TokenStream::new();
        if self.url_guard {
            tokens.extend(super::attributes::url_guard_tokens());
        }
        if self.option_probe {
            tokens.extend(super::attributes::option_value_probe_tokens());
        }
        tokens
    }
}

/// Writes `{expr}`: the value escaped as text.
pub(super) fn escaped_text(buffer: &Ident, expr: &Expr) -> TokenStream {
    quote! { #buffer.push_str(&rullst::html::escape(&(#expr))); }
}

/// Writes `value` (a reference) escaped for a double-quoted attribute.
pub(super) fn escaped_attribute(buffer: &Ident, value: &Ident) -> TokenStream {
    quote! { #buffer.push_str(&rullst::html::escape_attr(#value)); }
}

/// Writes the nodes of an `@for` body. A node that mentions `continue` or
/// `break` writes into a buffer of its own, appended when it completes.
pub(super) fn write_loop_body(body: &[HtmlNode], out: &mut Writer, helpers: &mut Helpers) {
    for node in body {
        if !node.mentions_loop_exit() {
            node.write(out, helpers);
            continue;
        }
        let depth = out.depth + 1;
        let own = Ident::new(&format!("s{depth}"), Span::mixed_site());
        let mut inner = Writer::new(own.clone(), depth);
        node.write(&mut inner, helpers);
        let statements = inner.finish();
        let capacity = node.static_size();
        let buffer = out.buffer().clone();
        out.code(quote! {
            {
                let mut #own = String::with_capacity(#capacity);
                #statements
                #buffer.push_str(&#own);
            }
        });
    }
}

/// Whether `expr` contains the keyword `continue` or `break` anywhere, even
/// where it leaves a loop of its own.
pub(super) fn expr_mentions_loop_exit(expr: &Expr) -> bool {
    mentions_loop_exit(expr.to_token_stream())
}

fn mentions_loop_exit(tokens: TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident == "continue" || ident == "break",
        TokenTree::Group(group) => mentions_loop_exit(group.stream()),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

#[cfg(test)]
#[path = "html_write_tests.rs"]
mod tests;
