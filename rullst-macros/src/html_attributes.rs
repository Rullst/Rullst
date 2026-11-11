#![cfg_attr(mutants, mutants::skip)]
//! Element attributes inside `html!`: `name="text"`, `name={value}`, bare
//! `name` and conditional `name?={value}`. Names are identifiers joined by
//! `-`, `:`, `::` or `.`, optionally after a leading `@` or `:`, as Alpine.js
//! and htmx write them (`@click.prevent`, `:class`, `x-on:click`,
//! `hx-on::after-request`).

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
    /// `name="text"`: author-owned source, written as it is except for `"`,
    /// which would end the attribute and is written `&quot;`.
    Static(LitStr),
    /// `name={value}`: always written, with the value escaped. An `Option`
    /// value raises a deprecation warning that points to `name?={value}`.
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
const URL_ATTRIBUTES: [&str; 14] = [
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
    // SVG's legacy link attribute, now that names may contain `:`.
    "xlink:href",
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

/// The text a fixed `name="text"` value writes between the attribute's double
/// quotes: the author's source, except `"` (a Rust `\"`), which would end the
/// attribute and is written `&quot;`. Everything else is kept, including
/// character references the author wrote, such as `&amp;`.
fn static_attribute_value(lit: &LitStr) -> String {
    lit.value().replace('"', "&quot;")
}

/// Items emitted once into an element with `name={value}` attributes. Each
/// such value is passed to `rullst_attribute_value`, which resolves by
/// autoref to the deprecated method for an `Option` (or `&Option`) and to the
/// plain one for every other type, so `name={option}` raises a deprecation
/// warning at the value while the attribute is written exactly as before. The
/// calls do nothing at run time. Generated inline, like the URL guard, so the
/// expansion needs nothing new from `rullst::html`.
pub fn option_value_probe_tokens() -> TokenStream {
    quote! {
        struct RullstAttributeValue<'a, T: ?Sized>(&'a T);
        trait RullstOptionAttributeValue {
            #[deprecated(
                note = "`name={value}` with an `Option` writes `name=\"\"` for `None`, and an empty \
                        boolean attribute such as `disabled` or `selected` is still on: write \
                        `name?={value}` to leave the attribute out for `None`, or pass \
                        `value.unwrap_or_default()` to keep the empty value"
            )]
            #[inline(always)]
            fn rullst_attribute_value(&self) {}
        }
        impl<T> RullstOptionAttributeValue for RullstAttributeValue<'_, ::core::option::Option<T>> {}
        impl<T> RullstOptionAttributeValue for RullstAttributeValue<'_, &::core::option::Option<T>> {}
        trait RullstPlainAttributeValue {
            #[inline(always)]
            fn rullst_attribute_value(&self) {}
        }
        impl<T: ?Sized> RullstPlainAttributeValue for &RullstAttributeValue<'_, T> {}
    }
}

/// Attributes whose value is run as JavaScript: `on*` event handlers,
/// htmx's `hx-on`, `hx-on-*` and `hx-on:*` (also with a `data-` prefix), and
/// Alpine.js directives: `x-*` and the `@*` and `:*` shorthands.
fn is_event_handler_attribute(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let unprefixed = name.strip_prefix("data-").unwrap_or(&name);
    (name.len() > 2 && name.starts_with("on") && !name.contains('-'))
        || unprefixed == "hx-on"
        || unprefixed.starts_with("hx-on-")
        || unprefixed.starts_with("hx-on:")
        || unprefixed.starts_with("x-")
        || name.starts_with('@')
        || name.starts_with(':')
}

/// A name segment after a separator: an identifier (Rust keywords
/// included) or digits with letters, as in Alpine's `.500ms` modifier.
fn name_segment(input: ParseStream, after: &str) -> Result<String> {
    if input.peek(syn::LitInt) {
        let number = input.parse::<syn::LitInt>()?;
        let text = number.to_string();
        if text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Ok(text);
        }
        return Err(syn::Error::new(
            number.span(),
            "an attribute name segment is letters, digits, `_` and `-`",
        ));
    }
    Ident::parse_any(input)
        .map(|ident| ident.to_string())
        .map_err(|error| {
            syn::Error::new(
                error.span(),
                format!("expected the rest of the attribute name after `{after}`"),
            )
        })
}

/// Whether the `:` that follows continues the name (`x-on:click`) rather
/// than starting the next attribute (`required :value="v"`): it does when
/// written directly after the name, or when the tokens carry no positions.
fn colon_continues(input: ParseStream) -> bool {
    super::text::gap(input.cursor().prev_span(), input.span()) != super::text::Gap::Space
}

/// Parses an attribute name.
fn parse_name(input: ParseStream) -> Result<(String, Span)> {
    let name_span = input.span();
    let mut name = String::new();
    if input.peek(Token![@]) {
        input.parse::<Token![@]>()?;
        name.push('@');
    } else if input.peek(Token![:]) && !input.peek(Token![::]) {
        input.parse::<Token![:]>()?;
        name.push(':');
    }
    name.push_str(&if name.is_empty() {
        Ident::parse_any(input)?.to_string()
    } else {
        name_segment(input, &name)?
    });
    loop {
        let separator = if input.peek(Token![-]) {
            input.parse::<Token![-]>()?;
            "-"
        } else if input.peek(Token![.]) && !input.peek(Token![..]) {
            input.parse::<Token![.]>()?;
            "."
        } else if input.peek(Token![::]) && colon_continues(input) {
            input.parse::<Token![::]>()?;
            "::"
        } else if input.peek(Token![:]) && colon_continues(input) {
            input.parse::<Token![:]>()?;
            ":"
        } else {
            break;
        };
        name.push_str(separator);
        name.push_str(&name_segment(input, separator)?);
    }
    Ok((name, name_span))
}

fn braced_expr(input: ParseStream) -> Result<Box<Expr>> {
    let content;
    syn::braced!(content in input);
    Ok(Box::new(content.parse::<Expr>()?))
}

impl Parse for HtmlAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        // Names are identifiers (Rust keywords included) and digits joined by
        // `-`, `:` or `.` after an optional `@` or `:`, so no quote, `=`, `>`
        // or space can reach the output through a name.
        let (name, name_span) = parse_name(input)?;
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
                HtmlAttrValue::Static(super::text::unsuffixed(input.parse::<LitStr>()?)?)
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
                     and Alpine.js directives run their value as JavaScript, which HTML escaping \
                     cannot make safe; use a static handler string, or attach the listener from a \
                     nonce'd script and pass data through a `data-*` attribute"
                ),
            ));
        }
        Ok(HtmlAttribute { name, value })
    }
}

impl HtmlAttribute {
    /// Whether the attribute is written `name={value}`, whose element needs
    /// [`option_value_probe_tokens`].
    pub fn is_dynamic(&self) -> bool {
        matches!(self.value, HtmlAttrValue::Dynamic(_))
    }

    /// Bytes this attribute writes at least, for the buffer capacity.
    pub fn static_size(&self) -> usize {
        match &self.value {
            HtmlAttrValue::Static(lit) => self.name.len() + 4 + static_attribute_value(lit).len(), //  name=""
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
                let attribute = format!(" {name}=\"{}\"", static_attribute_value(lit));
                (quote! { #buffer.push_str(#attribute); }, false)
            }
            HtmlAttrValue::Bare => {
                let attribute = format!(" {name}");
                (quote! { #buffer.push_str(#attribute); }, false)
            }
            HtmlAttrValue::Dynamic(expr) => {
                let binding = Ident::new("value", Span::mixed_site());
                let value = write_value(quote! { rullst::html::escape_attr(#binding) });
                // Spanned at the value, so the `Option` warning points there.
                let probe = quote_spanned! { expr.span()=>
                    (&RullstAttributeValue(#binding)).rullst_attribute_value();
                };
                // The scrutinee's temporaries, such as `format!(..)`, live
                // until the value has been written.
                let tokens = quote! {
                    match &(#expr) {
                        #binding => {
                            #probe
                            #buffer.push_str(#prefix);
                            #value
                            #buffer.push_str("\"");
                        }
                    }
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
