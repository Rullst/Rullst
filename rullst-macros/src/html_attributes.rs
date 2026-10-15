#![cfg_attr(mutants, mutants::skip)]
//! Element attributes inside `html!`: `name="text"`, `name={value}`, bare
//! `name` and conditional `name?={value}`.

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Expr, Ident, LitStr, Result, Token, token};

pub struct HtmlAttribute {
    pub name: String,
    pub value: HtmlAttrValue,
}

pub enum HtmlAttrValue {
    /// `name="text"`: author-owned source, written as it is.
    Static(LitStr),
    /// `name={value}`: always written, with the value escaped.
    Dynamic(Box<Expr>),
    /// `name`: a valueless (boolean) attribute, written as it is.
    Bare,
    /// `name?={value}`: a `bool` writes the bare name or nothing; an
    /// `Option` writes `name="value"` or nothing.
    Optional(Box<Expr>),
}

/// Attributes whose value a browser follows as a URL. A dynamic value with a
/// `javascript:`, `vbscript:` or (outside media elements) `data:` scheme is
/// replaced by `#` at runtime.
const URL_ATTRIBUTES: [&str; 13] = [
    "action",
    "background",
    "cite",
    "codebase",
    "data",
    "formaction",
    "href",
    "icon",
    "longdesc",
    "manifest",
    "poster",
    "src",
    "usemap",
];

/// Media elements whose URL attributes may keep inline `data:` resources.
const DATA_URL_ELEMENTS: [&str; 6] = ["audio", "image", "img", "source", "track", "video"];

fn is_url_attribute(name: &str) -> bool {
    URL_ATTRIBUTES
        .iter()
        .any(|attribute| name.eq_ignore_ascii_case(attribute))
}

pub fn allows_data_urls(tag: &str) -> bool {
    DATA_URL_ELEMENTS
        .iter()
        .any(|element| tag.eq_ignore_ascii_case(element))
}

/// Local helper emitted into elements with dynamic URL attributes. It returns
/// the escaped value, or `#` for a script-capable scheme. It is generated
/// inline instead of calling a runtime function, so the expansion works with
/// any `rullst::html` version that provides `escape_attr`.
///
/// Like the URL parser, it skips leading C0 controls and spaces and ignores
/// ASCII tab, LF and CR anywhere before reading the scheme. Every character
/// that can appear in a scheme survives `escape_attr` unchanged, so checking
/// the escaped value checks the URL the browser decodes. Entity references
/// written inside an explicit `RawHtml` value are not decoded.
pub fn url_guard_tokens() -> TokenStream {
    quote! {
        #[inline]
        fn rullst_url_attr(value: &str, allow_data: bool) -> &str {
            let mut scheme = [0u8; 10];
            let mut len = 0usize;
            for byte in value.bytes().skip_while(|byte| *byte <= b' ') {
                match byte {
                    b'\t' | b'\n' | b'\r' => {}
                    b':' => {
                        let scheme = &scheme[..len];
                        let unsafe_scheme = scheme == b"javascript"
                            || scheme == b"vbscript"
                            || (!allow_data && scheme == b"data");
                        return if unsafe_scheme { "#" } else { value };
                    }
                    b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'+' | b'-' | b'.'
                        if len < scheme.len() =>
                    {
                        scheme[len] = byte.to_ascii_lowercase();
                        len += 1;
                    }
                    _ => return value,
                }
            }
            value
        }
    }
}

/// Attributes whose value is run as JavaScript: `on*` event handlers and
/// htmx's `hx-on`/`hx-on-*` (also with a `data-` prefix).
fn is_event_handler_attribute(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let htmx = name.strip_prefix("data-").unwrap_or(&name);
    (name.len() > 2 && name.starts_with("on") && !name.contains('-'))
        || htmx == "hx-on"
        || htmx.starts_with("hx-on-")
}

fn braced_expr(input: ParseStream) -> Result<Box<Expr>> {
    let content;
    syn::braced!(content in input);
    Ok(Box::new(content.parse::<Expr>()?))
}

impl Parse for HtmlAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        // Names are identifiers (Rust keywords included) joined by `-`, so
        // no quote, `=`, `>` or space can reach the output through a name.
        let mut name_parts = Vec::new();
        let first = Ident::parse_any(input)?;
        let name_span = first.span();
        name_parts.push(first.to_string());

        while input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            name_parts.push(Ident::parse_any(input)?.to_string());
        }

        let name = name_parts.join("-");
        let value = if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            input.parse::<Token![=]>()?;
            if !input.peek(token::Brace) {
                return Err(input.error(format!(
                    "a conditional attribute takes a `bool` or `Option` expression in braces, \
                     such as `{name}?={{is_set}}`; write `{name}=\"...\"` or a bare `{name}` \
                     for a fixed attribute"
                )));
            }
            HtmlAttrValue::Optional(braced_expr(input)?)
        } else if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            if input.peek(token::Brace) {
                HtmlAttrValue::Dynamic(braced_expr(input)?)
            } else {
                HtmlAttrValue::Static(input.parse::<LitStr>()?)
            }
        } else {
            HtmlAttrValue::Bare
        };
        if matches!(
            value,
            HtmlAttrValue::Dynamic(_) | HtmlAttrValue::Optional(_)
        ) && is_event_handler_attribute(&name)
        {
            return Err(syn::Error::new(
                name_span,
                format!(
                    "dynamic `{name}` values are not allowed in html!: event-handler attributes \
                     run their value as JavaScript, which HTML escaping cannot make safe; use a \
                     static handler string, or attach the listener from a nonce'd script and \
                     pass data through a `data-*` attribute"
                ),
            ));
        }
        Ok(HtmlAttribute { name, value })
    }
}

impl HtmlAttribute {
    /// Bytes this attribute writes at least, for the buffer capacity.
    pub fn static_size(&self) -> usize {
        match &self.value {
            HtmlAttrValue::Static(lit) => self.name.len() + 4 + lit.value().len(), //  name=""
            HtmlAttrValue::Bare => self.name.len() + 1,
            HtmlAttrValue::Dynamic(_) | HtmlAttrValue::Optional(_) => self.name.len() + 4,
        }
    }

    /// Statements appending the attribute to `buffer`, and whether they call
    /// the element's local URL guard.
    pub fn to_tokens(&self, buffer: &Ident, allow_data_urls: bool) -> (TokenStream, bool) {
        let name = &self.name;
        let prefix = format!(" {name}=\"");
        let is_url = is_url_attribute(name);
        // The escaped value of a URL attribute goes through the guard.
        let write_value = |value: TokenStream| {
            if is_url {
                quote! { #buffer.push_str(rullst_url_attr(&#value, #allow_data_urls)); }
            } else {
                quote! { #buffer.push_str(&#value); }
            }
        };
        match &self.value {
            HtmlAttrValue::Static(lit) => {
                let attribute = format!(" {name}=\"{}\"", lit.value());
                (quote! { #buffer.push_str(#attribute); }, false)
            }
            HtmlAttrValue::Bare => {
                let attribute = format!(" {name}");
                (quote! { #buffer.push_str(#attribute); }, false)
            }
            HtmlAttrValue::Dynamic(expr) => {
                let value = write_value(quote! { rullst::html::escape_attr(&(#expr)) });
                // One statement, so temporaries such as `format!(..)` live
                // until the value has been written.
                let tokens = quote! {
                    #buffer.push_str(#prefix);
                    #value
                    #buffer.push_str("\"");
                };
                (tokens, is_url)
            }
            HtmlAttrValue::Optional(expr) => {
                let bare = format!(" {name}");
                let binding = Ident::new("value", Span::mixed_site());
                let value = write_value(quote! { #binding });
                // A value that is neither a `bool` nor an `Option` is reported
                // at the expression rather than at the whole macro.
                let presence =
                    quote_spanned! { expr.span()=> rullst::html::optional_attr(&(#expr)) };
                // The scrutinee's temporaries live until the match ends.
                let tokens = quote! {
                    match #presence {
                        rullst::html::AttributePresence::Value(#binding) => {
                            #buffer.push_str(#prefix);
                            #value
                            #buffer.push_str("\"");
                        }
                        rullst::html::AttributePresence::Bare => #buffer.push_str(#bare),
                        rullst::html::AttributePresence::Absent => {}
                    }
                };
                (tokens, is_url)
            }
        }
    }
}
