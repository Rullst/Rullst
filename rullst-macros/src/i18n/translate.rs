//! `__translate!`: checks a `t!` call against the encoded catalog and
//! expands it to a runtime lookup.

use super::template::{self, Segment};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::parse::{ParseStream, Parser};
use syn::{Expr, Ident, LitInt, LitStr, Token, braced, bracketed, parenthesized};

/// One key of the encoded catalog: its forms per locale, in locale order.
struct Entry {
    key: String,
    html: bool,
    plural: bool,
    /// `(category, template)` pairs per locale, `other` last; single
    /// messages only have `other`.
    forms: Vec<Vec<(Ident, String)>>,
}

struct Encoded {
    tags: Vec<LitStr>,
    rules: Vec<Ident>,
    default: LitInt,
    entries: Vec<Entry>,
}

struct Call {
    locale: Expr,
    key: LitStr,
    args: Vec<(Ident, Expr)>,
}

pub(crate) fn expand_translate(input: TokenStream) -> TokenStream {
    try_expand(input).unwrap_or_else(|error| error.to_compile_error())
}

/// Parses `<encoded catalog> ; <t! arguments>` and expands the call.
pub(crate) fn try_expand(input: TokenStream) -> syn::Result<TokenStream> {
    let (encoded, call) = Parser::parse2(
        |input: ParseStream| {
            let encoded = parse_encoded(input)?;
            input.parse::<Token![;]>()?;
            let call = parse_call(input)?;
            Ok((encoded, call))
        },
        input,
    )?;
    expand(&encoded, call)
}

fn parse_encoded(input: ParseStream) -> syn::Result<Encoded> {
    let locales;
    bracketed!(locales in input);
    let mut tags = Vec::new();
    let mut rules = Vec::new();
    while !locales.is_empty() {
        tags.push(locales.parse()?);
        rules.push(locales.parse()?);
        if !locales.is_empty() {
            locales.parse::<Token![,]>()?;
        }
    }
    let default = input.parse()?;
    let body;
    braced!(body in input);
    let mut entries = Vec::new();
    while !body.is_empty() {
        let key: LitStr = body.parse()?;
        let kind: Ident = body.parse()?;
        let shape: Ident = body.parse()?;
        let groups;
        bracketed!(groups in body);
        let mut forms = Vec::new();
        while !groups.is_empty() {
            let group;
            parenthesized!(group in groups);
            let mut locale_forms = Vec::new();
            while !group.is_empty() {
                let category: Ident = group.parse()?;
                if category_variant(&category).is_none() {
                    return Err(syn::Error::new(
                        category.span(),
                        format!("unknown plural category `{category}`"),
                    ));
                }
                let text: LitStr = group.parse()?;
                locale_forms.push((category, text.value()));
            }
            forms.push(locale_forms);
        }
        entries.push(Entry {
            key: key.value(),
            html: kind == "html",
            plural: shape == "plural",
            forms,
        });
    }
    Ok(Encoded {
        tags,
        rules,
        default,
        entries,
    })
}

fn parse_call(input: ParseStream) -> syn::Result<Call> {
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

fn expand(encoded: &Encoded, call: Call) -> syn::Result<TokenStream> {
    let key = call.key.value();
    let Some(entry) = encoded.entries.iter().find(|entry| entry.key == key) else {
        let mut message = format!("unknown translation key `{key}`");
        if let Some(close) = suggest(&key, encoded.entries.iter().map(|e| e.key.as_str())) {
            message.push_str(&format!("; did you mean `{close}`?"));
        }
        return Err(syn::Error::new(call.key.span(), message));
    };
    let parsed = parse_forms(entry, call.key.span())?;
    let mut names: BTreeSet<String> = parsed
        .iter()
        .flatten()
        .flat_map(|(_, segments)| template::placeholders(segments))
        .collect();
    if entry.plural {
        names.insert("count".to_owned());
    }
    check_args(entry, &call, &names)?;

    let placeholder_index = |name: &str| names.iter().position(|n| n == name).unwrap_or_default();
    let forms = parsed.iter().map(|locale_forms| {
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
    let tags = &encoded.tags;
    let rules = &encoded.rules;
    let default = &encoded.default;
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

/// The parsed templates of each locale, by category.
type Forms<'a> = Vec<(&'a Ident, Vec<Segment>)>;

fn parse_forms(entry: &Entry, span: Span) -> syn::Result<Vec<Forms<'_>>> {
    let parse = |text: &str| {
        template::parse(text)
            .map_err(|problem| syn::Error::new(span, format!("`{}` {problem}", entry.key)))
    };
    entry
        .forms
        .iter()
        .map(|locale_forms| {
            locale_forms
                .iter()
                .map(|(category, text)| Ok((category, parse(text)?)))
                .collect()
        })
        .collect()
}

/// The `rullst::i18n::PluralCategory` variant of an encoded category.
fn category_variant(category: &Ident) -> Option<&'static str> {
    Some(match category.to_string().as_str() {
        "zero" => "Zero",
        "one" => "One",
        "two" => "Two",
        "few" => "Few",
        "many" => "Many",
        "other" => "Other",
        _ => return None,
    })
}

fn check_args(entry: &Entry, call: &Call, names: &BTreeSet<String>) -> syn::Result<()> {
    let key = &entry.key;
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
