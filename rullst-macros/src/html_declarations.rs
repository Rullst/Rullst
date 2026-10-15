//! HTML comments and the `<!DOCTYPE html>` declaration inside `html!`.

use proc_macro2::{Span, TokenTree};
use syn::ext::IdentExt;
use syn::parse::ParseStream;
use syn::{Ident, LitStr, Result, Token};

use super::HtmlNode;

pub(super) fn starts_comment(input: ParseStream) -> bool {
    input.peek(Token![<]) && input.peek2(Token![!])
}

/// `<!DOCTYPE html>` (any letter case); a comment starts with `<!--` instead.
pub(super) fn starts_doctype(input: ParseStream) -> bool {
    input.peek(Token![<]) && input.peek2(Token![!]) && input.peek3(Ident)
}

/// Parses `<!DOCTYPE html>` into a text node that emits the declaration as
/// written. Only the HTML5 form is accepted.
pub(super) fn parse_doctype(input: ParseStream) -> Result<HtmlNode> {
    let open = input.parse::<Token![<]>()?;
    input.parse::<Token![!]>()?;
    let keyword = input.parse::<Ident>()?;
    let name = Ident::parse_any(input)?;
    if !keyword.to_string().eq_ignore_ascii_case("doctype")
        || !name.to_string().eq_ignore_ascii_case("html")
        || !input.peek(Token![>])
    {
        return Err(syn::Error::new(
            keyword.span(),
            "html! accepts only the HTML5 declaration `<!DOCTYPE html>`",
        ));
    }
    input.parse::<Token![>]>()?;
    Ok(HtmlNode::Text(LitStr::new(
        &format!("<!{keyword} {name}>"),
        open.span,
    )))
}

pub(super) fn misplaced_doctype(input: ParseStream) -> syn::Error {
    syn::Error::new(
        input.span(),
        "`<!DOCTYPE html>` must come before any other content in html!",
    )
}

pub(super) fn parse_comment(input: ParseStream) -> Result<()> {
    input.parse::<Token![<]>()?;
    input.parse::<Token![!]>()?;
    input.parse::<Token![-]>()?;
    input.parse::<Token![-]>()?;

    while !input.is_empty() {
        if input.peek(Token![-]) && input.peek2(Token![-]) && input.peek3(Token![>]) {
            input.parse::<Token![-]>()?;
            input.parse::<Token![-]>()?;
            input.parse::<Token![>]>()?;
            return Ok(());
        }
        input.parse::<TokenTree>()?;
    }

    Err(syn::Error::new(
        Span::call_site(),
        "unterminated HTML comment; expected -->",
    ))
}
