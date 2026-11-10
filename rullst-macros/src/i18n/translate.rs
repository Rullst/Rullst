//! `__translate!`: checks a `t!` call against the catalog that `catalog!`
//! validated and expands it to a runtime lookup.
//!
//! `t!` forwards `"dir" ["tag", …] "default" ; <call>`: the catalog's
//! directory, locale order and default, not its contents, so the cost of a
//! call does not grow with the catalog (see [`super::prepared`]).

use super::prepared::{self, Entry, Prepared};
use super::template::Segment;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::parse::{ParseStream, Parser};
use syn::{Expr, Ident, LitStr, Token, bracketed};

/// Where `catalog!` found the catalog.
pub(crate) struct Header {
    dir: LitStr,
    order: Vec<String>,
    default: LitStr,
}

pub(crate) struct Call {
    locale: Expr,
    key: LitStr,
    args: Vec<(Ident, Expr)>,
}

pub(crate) fn expand_translate(input: TokenStream) -> TokenStream {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default();
    try_expand(std::path::Path::new(&manifest_dir), input)
        .unwrap_or_else(|error| error.to_compile_error())
}

/// Parses `<header> ; <t! arguments>`, loads the catalog relative to
/// `manifest_dir` and expands the call.
pub(crate) fn try_expand(
    manifest_dir: &std::path::Path,
    input: TokenStream,
) -> syn::Result<TokenStream> {
    let (header, call) = Parser::parse2(
        |input: ParseStream| {
            let header = parse_header(input)?;
            input.parse::<Token![;]>()?;
            Ok((header, parse_call(input)?))
        },
        input,
    )?;
    let prepared = prepared::load(
        manifest_dir,
        &header.dir.value(),
        &header.order,
        &header.default.value(),
    )
    .map_err(|_| {
        syn::Error::new(
            call.key.span(),
            format!(
                "the translation catalog in `{}` has errors; `catalog!` reports them",
                header.dir.value()
            ),
        )
    })?;
    expand(&prepared, call)
}

fn parse_header(input: ParseStream) -> syn::Result<Header> {
    let dir = input.parse()?;
    let tags;
    bracketed!(tags in input);
    let mut order = Vec::new();
    while !tags.is_empty() {
        order.push(tags.parse::<LitStr>()?.value());
        if !tags.is_empty() {
            tags.parse::<Token![,]>()?;
        }
    }
    let default = input.parse()?;
    Ok(Header {
        dir,
        order,
        default,
    })
}

pub(crate) fn parse_call(input: ParseStream) -> syn::Result<Call> {
    if input.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "expected `t!(locale, \"key\", name = value, …)`",
        ));
    }
    let locale: Expr = input.parse()?;
    input.parse::<Token![,]>()?;
    let key = match input.parse::<LitStr>() {
        Ok(key) => key,
        Err(error) => {
            return Err(syn::Error::new(
                error.span(),
                "the translation key must be a string literal, so that it is checked at compile time",
            ));
        }
    };
    let mut args = Vec::new();
    while !input.is_empty() {
        input.parse::<Token![,]>()?;
        if input.is_empty() {
            break;
        }
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        args.push((name, input.parse()?));
    }
    Ok(Call { locale, key, args })
}

/// Expands one call against a prepared catalog.
pub(crate) fn expand(prepared: &Prepared, call: Call) -> syn::Result<TokenStream> {
    let key = call.key.value();
    let Some(entry) = prepared.entries.get(&key) else {
        let mut message = format!("unknown translation key `{key}`");
        if let Some(close) = suggest(&key, prepared.entries.keys().map(String::as_str)) {
            message.push_str(&format!("; did you mean `{close}`?"));
        }
        return Err(syn::Error::new(call.key.span(), message));
    };
    let names = &entry.names;
    check_args(&key, entry, &call, names)?;

    let placeholder_index = |name: &str| names.iter().position(|n| n == name).unwrap_or_default();
    let forms = entry.forms.iter().map(|locale_forms| {
        let mut other = Vec::new();
        let mut plural = Vec::new();
        for (category, form) in locale_forms {
            let form = segments(form, &placeholder_index);
            match category_variant(category) {
                Some("Other") | None => other = form.collect(),
                Some(variant) => {
                    let variant = format_ident!("{variant}");
                    plural.push(quote! {
                        (rullst::i18n::PluralCategory::#variant, &[#(#form),*])
                    });
                }
            }
        }
        quote! { rullst::i18n::__Forms { plural: &[#(#plural),*], other: &[#(#other),*] } }
    });
    let arg_ident = |name: &str| {
        let index = call
            .args
            .iter()
            .position(|(n, _)| n == name)
            .unwrap_or_default();
        format_ident!("__rullst_i18n_arg_{}", index, span = Span::mixed_site())
    };
    let bindings = call.args.iter().enumerate().map(|(index, (_, value))| {
        let ident = format_ident!("__rullst_i18n_arg_{}", index, span = Span::mixed_site());
        quote! { let #ident = &(#value); }
    });
    let displays = names.iter().map(|name| {
        let ident = arg_ident(name);
        quote! { #ident as &dyn ::core::fmt::Display }
    });
    let count = if entry.plural {
        let ident = arg_ident("count");
        quote! { ::core::option::Option::Some(rullst::i18n::PluralCount::plural_decimal(#ident)) }
    } else {
        quote! { ::core::option::Option::None }
    };
    let tags = &prepared.tags;
    let rules = prepared.rules.iter().map(|rule| format_ident!("{rule}"));
    let default = prepared.default;
    let render = if entry.html {
        quote! { rullst::i18n::__html }
    } else {
        quote! { rullst::i18n::__text }
    };
    let locale_expr = &call.locale;
    let locale = Ident::new("__rullst_i18n_locale", Span::mixed_site());
    Ok(quote! {
        {
            const __RULLST_I18N_CATALOG: rullst::i18n::__Catalog = rullst::i18n::__Catalog {
                tags: &[#(#tags),*],
                plurals: &[#(rullst::i18n::__Plural::#rules),*],
                default: #default,
            };
            const __RULLST_I18N_FORMS: &[rullst::i18n::__Forms] = &[#(#forms),*];
            let #locale: rullst::i18n::Locale = rullst::i18n::__locale(&(#locale_expr));
            #(#bindings)*
            #render(
                &__RULLST_I18N_CATALOG,
                #locale,
                #count,
                __RULLST_I18N_FORMS,
                &[#(#displays),*],
            )
        }
    })
}

/// The `rullst::i18n::PluralCategory` variant of a CLDR category.
fn category_variant(category: &str) -> Option<&'static str> {
    Some(match category {
        "zero" => "Zero",
        "one" => "One",
        "two" => "Two",
        "few" => "Few",
        "many" => "Many",
        "other" => "Other",
        _ => return None,
    })
}

fn check_args(key: &str, entry: &Entry, call: &Call, names: &BTreeSet<String>) -> syn::Result<()> {
    let mut errors: Vec<syn::Error> = Vec::new();
    let expected = if names.is_empty() {
        "it takes no arguments".to_owned()
    } else {
        let list: Vec<&str> = names.iter().map(String::as_str).collect();
        format!("it takes {}", list.join(", "))
    };
    let mut seen = BTreeSet::new();
    for (name, _) in &call.args {
        let text = name.to_string();
        if !names.contains(&text) {
            errors.push(syn::Error::new(
                name.span(),
                format!("`{text}` is not a placeholder of `{key}`; {expected}"),
            ));
        } else if !seen.insert(text.clone()) {
            errors.push(syn::Error::new(
                name.span(),
                format!("`{text}` is given twice"),
            ));
        }
    }
    for name in names.iter().filter(|name| !seen.contains(*name)) {
        let hint = if entry.plural && name == "count" {
            " (the number that selects the plural form)"
        } else {
            ""
        };
        errors.push(syn::Error::new(
            call.key.span(),
            format!("missing argument `{name}`{hint} for `{key}`; {expected}"),
        ));
    }
    let mut errors = errors.into_iter();
    match errors.next() {
        None => Ok(()),
        Some(mut first) => {
            first.extend(errors);
            Err(first)
        }
    }
}

fn segments(
    segments: &[Segment],
    index: &impl Fn(&str) -> usize,
) -> impl Iterator<Item = TokenStream> {
    segments.iter().map(|segment| match segment {
        Segment::Text(text) => quote! { rullst::i18n::__Segment::Text(#text) },
        Segment::Placeholder(name) => {
            let index = index(name);
            quote! { rullst::i18n::__Segment::Arg(#index) }
        }
    })
}

/// The closest known key within a small edit distance, for typos.
pub(crate) fn suggest<'a>(key: &str, keys: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let limit = (key.chars().count() / 3).max(2);
    keys.map(|candidate| (distance(key, candidate), candidate))
        .filter(|(distance, _)| *distance <= limit)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

/// Levenshtein distance between two short strings.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != *cb);
            current.push(substitution.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    previous.last().copied().unwrap_or_default()
}
