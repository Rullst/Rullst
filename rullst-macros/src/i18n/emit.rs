//! `catalog!`: loads the catalogs and defines `LOCALES` and `t!`.

use super::catalog::{self, Catalog};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token, bracketed};

/// `catalog!(default = "en", dir = "locales", order = ["en", "pt-BR"])`.
pub(crate) struct CatalogArgs {
    default: LitStr,
    dir: Option<LitStr>,
    /// The explicit locale order and the span of `order`.
    order: Option<(Vec<LitStr>, Span)>,
}

const ARGUMENTS: &str = "`catalog!` takes `default = \"tag\"`, an optional `dir = \"path\"` and an optional `order = [\"tag\", …]`";

impl Parse for CatalogArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut default = None;
        let mut dir = None;
        let mut order = None;
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let repeated = match name.to_string().as_str() {
                "default" => default.replace(input.parse::<LitStr>()?).is_some(),
                "dir" => dir.replace(input.parse::<LitStr>()?).is_some(),
                "order" => {
                    let tags;
                    bracketed!(tags in input);
                    let tags = tags.parse_terminated(<LitStr as Parse>::parse, Token![,])?;
                    order
                        .replace((tags.into_iter().collect(), name.span()))
                        .is_some()
                }
                _ => {
                    return Err(syn::Error::new(
                        name.span(),
                        format!("unknown argument: {ARGUMENTS}"),
                    ));
                }
            };
            if repeated {
                return Err(syn::Error::new(
                    name.span(),
                    format!("`{name}` is given twice"),
                ));
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        let default = default.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "missing `default = \"tag\"`, the locale used when negotiation finds no match",
            )
        })?;
        Ok(Self {
            default,
            dir,
            order,
        })
    }
}

/// Expands `catalog!` against the crate's `CARGO_MANIFEST_DIR`.
pub(crate) fn expand_catalog(input: TokenStream) -> TokenStream {
    let args = match syn::parse2::<CatalogArgs>(input) {
        Ok(args) => args,
        Err(error) => return with_fallback(error.to_compile_error(), "en"),
    };
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default();
    expand_in(std::path::Path::new(&manifest_dir), &args)
}

/// Expands `catalog!` with catalogs resolved against `manifest_dir`.
pub(crate) fn expand_in(manifest_dir: &std::path::Path, args: &CatalogArgs) -> TokenStream {
    let default = args.default.value();
    let dir = args
        .dir
        .as_ref()
        .map_or_else(|| "locales".to_owned(), LitStr::value);
    let span = args.dir.as_ref().map_or_else(Span::call_site, LitStr::span);
    if std::path::Path::new(&dir).is_absolute() {
        let error = syn::Error::new(span, "`dir` must be relative to the crate's Cargo.toml");
        return with_fallback(error.to_compile_error(), &default);
    }
    let order = args
        .order
        .as_ref()
        .map(|(tags, _)| tags.iter().map(LitStr::value).collect::<Vec<_>>());
    let catalog = catalog::read_dir(manifest_dir, &dir)
        .and_then(|sources| catalog::build(sources, &default))
        .map_err(|messages| (messages, span))
        .and_then(|catalog| match &order {
            Some(order) => catalog::ordered(catalog, order)
                .map_err(|messages| (messages, args.order.as_ref().map_or(span, |o| o.1))),
            None => Ok(catalog),
        });
    match catalog {
        Ok(catalog) => generate(&catalog, &dir),
        Err((messages, span)) => {
            let errors = messages
                .iter()
                .map(|message| syn::Error::new(span, message).to_compile_error());
            with_fallback(quote! { #(#errors)* }, &default)
        }
    }
}

/// `LOCALES`, the file tracking and a `t!` that forwards where the catalog
/// is, not what it holds: `__translate!` reads and prepares it once per
/// compilation, so a call costs the same whatever the catalog's size.
fn generate(catalog: &Catalog, dir: &str) -> TokenStream {
    let tags: Vec<&String> = catalog.locales.iter().map(|locale| &locale.tag).collect();
    let default = catalog.default;
    let default_tag = catalog
        .locales
        .get(default)
        .map_or("", |locale| locale.tag.as_str());
    let paths = catalog
        .locales
        .iter()
        .map(|locale| locale.path.to_string_lossy().into_owned());
    let items = quote! {
        /// The locales of this crate's translation catalog, generated by
        /// `rullst::i18n::catalog!`.
        #[allow(dead_code)]
        pub static LOCALES: rullst::i18n::Locales =
            rullst::i18n::Locales::__from_catalog(&[#(#tags),*], #default);
        #(const _: &[u8] = include_bytes!(#paths);)*
    };
    let translate =
        quote! { rullst::i18n::__translate! { #dir [#(#tags),*] #default_tag ; $($input)* } };
    define_t(items, translate)
}

/// Defines `t!` next to `items`; the `$($input:tt)*` forwarding keeps the
/// caller's spans for precise errors.
fn define_t(items: TokenStream, body: TokenStream) -> TokenStream {
    quote! {
        #items
        /// Translates a catalog key: `t!(locale, "key", name = value)`.
        /// Generated by `rullst::i18n::catalog!`.
        #[allow(unused_macros)]
        macro_rules! t {
            ($($input:tt)*) => { #body };
        }
        #[allow(unused_imports)]
        pub(crate) use t;
    }
}

/// After a catalog error, still defines `LOCALES` and a `t!` that
/// type-checks anywhere, so the error is not buried under follow-on ones.
pub(crate) fn with_fallback(errors: TokenStream, default: &str) -> TokenStream {
    let default = if super::tag::canonical(default).is_some() {
        default
    } else {
        "en"
    };
    let items = quote! {
        #errors
        #[allow(dead_code)]
        pub static LOCALES: rullst::i18n::Locales =
            rullst::i18n::Locales::__from_catalog(&[#default], 0);
    };
    define_t(items, quote! { loop {} })
}
