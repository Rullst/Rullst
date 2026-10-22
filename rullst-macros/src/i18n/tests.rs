//! `catalog!` and `__translate!` expansions.

use super::catalog::{self, Source};
use super::emit::{self, CatalogArgs};
use super::translate::{self, suggest};
use proc_macro2::TokenStream;
use quote::quote;
use std::path::{Path, PathBuf};

fn source(tag: &str, text: &str) -> Source {
    Source {
        tag: tag.to_owned(),
        file: format!("locales/{tag}.toml"),
        path: PathBuf::from(format!("/abs/{tag}.toml")),
        text: text.to_owned(),
    }
}

fn encoded(sources: Vec<Source>) -> TokenStream {
    emit::encode(&catalog::build(sources, "en").unwrap())
}

const EN: &str = r#"
nav.home = "Home"
pair = "{b} and {a}"
legal.terms_html = "Read the <a href='/t'>terms</a>, {name}"
[inbox]
one = "One message"
other = "{count} messages"
[visits]
one = "{name} visited once"
other = "{name} visited {count} times"
"#;
const PT: &str = r#"
nav.home = "Início"
pair = "{a} e {b}"
legal.terms_html = "Leia os <a href='/t'>termos</a>, {name}"
[inbox]
one = "{count} mensagem"
other = "{count} mensagens"
[visits]
one = "{name} visitou {count} vez"
other = "{name} visitou {count} vezes"
"#;

fn expand(call: TokenStream) -> syn::Result<String> {
    let encoded = encoded(vec![source("en", EN), source("pt-BR", PT)]);
    translate::try_expand(quote! { #encoded ; #call }).map(|tokens| tokens.to_string())
}

fn errors(call: TokenStream) -> Vec<String> {
    match expand(call) {
        Ok(tokens) => panic!("expected an error, got {tokens}"),
        Err(error) => error.into_iter().map(|e| e.to_string()).collect(),
    }
}

fn contains(haystack: &str, needle: TokenStream) {
    let needle = needle.to_string();
    assert!(haystack.contains(&needle), "{needle}\nnot in\n{haystack}");
}

#[test]
fn encode_lists_locales_rules_and_forms_per_key() {
    let tokens = encoded(vec![
        source(
            "en",
            "a = \"A\"\nb_html = \"<b>B</b>\"\n[n]\none = \"1\"\nother = \"N\"\n",
        ),
        source(
            "ja",
            "a = \"エー\"\nb_html = \"<b>ビー</b>\"\n[n]\nother = \"件\"\n",
        ),
    ]);
    let expected = quote! {
        ["en" One, "ja" Other] 0usize {
            "a" text single [(other "A") (other "エー")]
            "b_html" html single [(other "<b>B</b>") (other "<b>ビー</b>")]
            "n" text plural [(one "1" other "N") (other "件")]
        }
    };
    assert_eq!(tokens.to_string(), expected.to_string());
}

#[test]
fn a_plain_message_selects_by_locale_without_arguments() {
    let out = expand(quote! { locale, "nav.home" }).unwrap();
    contains(&out, quote! { tags: &["en", "pt-BR"], });
    contains(
        &out,
        quote! { plurals: &[rullst::i18n::__Plural::One, rullst::i18n::__Plural::ZeroOne], },
    );
    contains(&out, quote! { default: 0usize, });
    contains(
        &out,
        quote! {
            &[rullst::i18n::__Forms { plural: &[], other: &[rullst::i18n::__Segment::Text("Home")] },
              rullst::i18n::__Forms { plural: &[], other: &[rullst::i18n::__Segment::Text("Início")] }]
        },
    );
    contains(&out, quote! { rullst::i18n::__locale(&(locale)); });
    contains(
        &out,
        quote! { rullst::i18n::__text(&__RULLST_I18N_CATALOG, __rullst_i18n_locale, ::core::option::Option::None, __RULLST_I18N_FORMS, &[],) },
    );
}

#[test]
fn arguments_bind_in_call_order_and_display_in_placeholder_order() {
    let out = expand(quote! { ctx.locale, "pair", b = first(), a = second(), }).unwrap();
    contains(
        &out,
        quote! { let __rullst_i18n_arg_0 = &(first()); let __rullst_i18n_arg_1 = &(second()); },
    );
    contains(
        &out,
        quote! { &[__rullst_i18n_arg_1 as &dyn ::core::fmt::Display, __rullst_i18n_arg_0 as &dyn ::core::fmt::Display] },
    );
    contains(
        &out,
        quote! { other: &[rullst::i18n::__Segment::Arg(1usize), rullst::i18n::__Segment::Text(" and "), rullst::i18n::__Segment::Arg(0usize)] },
    );
    contains(
        &out,
        quote! { other: &[rullst::i18n::__Segment::Arg(0usize), rullst::i18n::__Segment::Text(" e "), rullst::i18n::__Segment::Arg(1usize)] },
    );
}

#[test]
fn a_plural_message_passes_its_count() {
    let out = expand(quote! { locale, "inbox", count = unread.len() }).unwrap();
    contains(
        &out,
        quote! { ::core::option::Option::Some(rullst::i18n::PluralCount::plural_decimal(__rullst_i18n_arg_0)) },
    );
    contains(
        &out,
        quote! { rullst::i18n::__Forms { plural: &[(rullst::i18n::PluralCategory::One, &[rullst::i18n::__Segment::Text("One message")])], other: &[rullst::i18n::__Segment::Arg(0usize), rullst::i18n::__Segment::Text(" messages")] } },
    );
}

#[test]
fn every_category_of_a_plural_message_is_expanded() {
    let encoded = encoded(vec![
        source(
            "en",
            "[n]\none = \"{count} file\"\nother = \"{count} files\"\n",
        ),
        source(
            "ru",
            "[n]\none = \"{count} файл\"\nfew = \"{count} файла\"\nmany = \"{count} файлов\"\nother = \"{count} файла\"\n",
        ),
    ]);
    let out = translate::try_expand(quote! { #encoded ; locale, "n", count = 3 })
        .unwrap()
        .to_string();
    contains(
        &out,
        quote! { plurals: &[rullst::i18n::__Plural::One, rullst::i18n::__Plural::EastSlavic], },
    );
    contains(
        &out,
        quote! {
            rullst::i18n::__Forms {
                plural: &[
                    (rullst::i18n::PluralCategory::One, &[rullst::i18n::__Segment::Arg(0usize), rullst::i18n::__Segment::Text(" файл")]),
                    (rullst::i18n::PluralCategory::Few, &[rullst::i18n::__Segment::Arg(0usize), rullst::i18n::__Segment::Text(" файла")]),
                    (rullst::i18n::PluralCategory::Many, &[rullst::i18n::__Segment::Arg(0usize), rullst::i18n::__Segment::Text(" файлов")])
                ],
                other: &[rullst::i18n::__Segment::Arg(0usize), rullst::i18n::__Segment::Text(" файла")]
            }
        },
    );
}

#[test]
fn encoded_categories_must_be_cldr_names() {
    let bad = quote! { ["en" One] 0usize { "n" text plural [(several "x" other "y")] } ; locale, "n", count = 1 };
    let error = translate::try_expand(bad).unwrap_err().to_string();
    assert_eq!(error, "unknown plural category `several`");
    let zero = quote! { ["ar" Arabic] 0usize { "n" text plural [(zero "z" two "t" other "o")] } ; locale, "n", count = 1 };
    let out = translate::try_expand(zero).unwrap().to_string();
    contains(
        &out,
        quote! { (rullst::i18n::PluralCategory::Zero, &[rullst::i18n::__Segment::Text("z")]) },
    );
    contains(
        &out,
        quote! { (rullst::i18n::PluralCategory::Two, &[rullst::i18n::__Segment::Text("t")]) },
    );
}

#[test]
fn an_html_key_renders_trusted_markup() {
    let out = expand(quote! { locale, "legal.terms_html", name = user }).unwrap();
    contains(&out, quote! { rullst::i18n::__html });
    let plain = expand(quote! { locale, "pair", a = 1, b = 2 }).unwrap();
    assert!(!plain.contains("__html"));
}

#[test]
fn unknown_keys_suggest_the_closest_one() {
    assert_eq!(
        errors(quote! { locale, "nav.hone" }),
        ["unknown translation key `nav.hone`; did you mean `nav.home`?"]
    );
    assert_eq!(
        errors(quote! { locale, "settings.title" }),
        ["unknown translation key `settings.title`"]
    );
}

#[test]
fn arguments_must_match_the_placeholders() {
    assert_eq!(
        errors(quote! { locale, "pair", a = 1 }),
        ["missing argument `b` for `pair`; it takes a, b"]
    );
    assert_eq!(
        errors(quote! { locale, "nav.home", name = 1 }),
        ["`name` is not a placeholder of `nav.home`; it takes no arguments"]
    );
    assert_eq!(
        errors(quote! { locale, "pair", a = 1, b = 2, a = 3 }),
        ["`a` is given twice"]
    );
    assert_eq!(
        errors(quote! { locale, "inbox" }),
        [
            "missing argument `count` (the number that selects the plural form) for `inbox`; it takes count"
        ]
    );
    assert_eq!(
        errors(quote! { locale, "visits", count = 2 }),
        ["missing argument `name` for `visits`; it takes count, name"]
    );
    assert_eq!(
        errors(quote! { locale, "pair", c = 1, a = 1 }),
        [
            "`c` is not a placeholder of `pair`; it takes a, b",
            "missing argument `b` for `pair`; it takes a, b",
        ]
    );
}

#[test]
fn malformed_calls_are_rejected() {
    assert_eq!(
        errors(quote! { locale, key }),
        ["the translation key must be a string literal, so that it is checked at compile time"]
    );
    assert_eq!(
        errors(quote! {}),
        ["expected `t!(locale, \"key\", name = value, …)`"]
    );
    assert_eq!(errors(quote! { locale }), ["expected `,`"]);
}

#[test]
fn suggestions_are_limited_to_small_edit_distances() {
    let keys = ["nav.home", "nav.about", "dashboard.title"];
    assert_eq!(suggest("nav.hme", keys.into_iter()), Some("nav.home"));
    assert_eq!(suggest("nav.abo", keys.into_iter()), Some("nav.about"));
    assert_eq!(
        suggest("dashboard.ti", keys.into_iter()),
        Some("dashboard.title")
    );
    assert_eq!(suggest("ab", ["xy"].into_iter()), Some("xy"));
    assert_eq!(suggest("abc", ["xyz"].into_iter()), None);
    assert_eq!(suggest("profile", keys.into_iter()), None);
}

fn args(tokens: TokenStream) -> syn::Result<CatalogArgs> {
    syn::parse2(tokens)
}

fn arg_error(tokens: TokenStream) -> String {
    match args(tokens) {
        Ok(_) => panic!("expected an argument error"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn catalog_arguments_are_validated() {
    assert!(args(quote! { default = "en" }).is_ok());
    assert!(args(quote! { dir = "x", default = "en", }).is_ok());
    assert_eq!(
        arg_error(quote! { dir = "x" }),
        "missing `default = \"tag\"`, the locale used when negotiation finds no match"
    );
    assert_eq!(
        arg_error(quote! { default = "en", path = "x" }),
        "unknown argument: `catalog!` takes `default = \"tag\"` and an optional `dir = \"path\"`"
    );
    assert_eq!(
        arg_error(quote! { default = "en", default = "es" }),
        "`default` is given twice"
    );
}

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn a_valid_catalog_defines_locales_tracking_and_t() {
    let args = args(quote! { default = "pt-BR", dir = "tests/i18n/valid" }).unwrap();
    let out = emit::expand_in(manifest_dir(), &args).to_string();
    contains(
        &out,
        quote! { pub static LOCALES: rullst::i18n::Locales = rullst::i18n::Locales::__from_catalog(&["en", "pt-BR"], 1usize); },
    );
    // `read_dir` joins each file name to `<manifest>/<dir>` with the platform
    // separator: `...\tests/i18n/valid\en.toml` on Windows, which Windows
    // resolves (rullst-core's `tests/i18n_catalog.rs` compiles there).
    let en = manifest_dir().join("tests/i18n/valid").join("en.toml");
    let en = en.to_string_lossy();
    contains(&out, quote! { const _: &[u8] = include_bytes!(#en); });
    let sources = catalog::read_dir(manifest_dir(), "tests/i18n/valid").unwrap();
    let encoded = emit::encode(&catalog::build(sources, "pt-BR").unwrap());
    assert!(
        encoded
            .to_string()
            .starts_with("[\"en\" One , \"pt-BR\" ZeroOne] 1usize {")
    );
    contains(
        &out,
        quote! { macro_rules! t { ($($input:tt)*) => { rullst::i18n::__translate! { #encoded ; $($input)* } }; } },
    );
    contains(&out, quote! { #[allow(unused_imports)] pub(crate) use t; });
    assert!(!out.contains("compile_error"));
}

#[test]
fn catalog_errors_keep_a_fallback_locales_and_t() {
    let args = args(quote! { default = "en", dir = "tests/i18n/missing_key" }).unwrap();
    let out = emit::expand_in(manifest_dir(), &args).to_string();
    contains(
        &out,
        quote! { ::core::compile_error! { "tests/i18n/missing_key/pt-BR.toml: missing key `nav.about` (defined in tests/i18n/missing_key/en.toml)" } },
    );
    contains(
        &out,
        quote! { rullst::i18n::Locales::__from_catalog(&["en"], 0); },
    );
    contains(
        &out,
        quote! { macro_rules! t { ($($input:tt)*) => { loop {} }; } },
    );

    let absolute = args_abs();
    let out = emit::expand_in(manifest_dir(), &absolute).to_string();
    contains(
        &out,
        quote! { ::core::compile_error! { "`dir` must be relative to the crate's Cargo.toml" } },
    );
    contains(&out, quote! { __from_catalog(&["es"], 0); });

    let out = emit::with_fallback(TokenStream::new(), "english").to_string();
    contains(&out, quote! { __from_catalog(&["en"], 0); });
    let out = emit::expand_catalog(quote! { dir = "locales" }).to_string();
    assert!(out.contains("missing `default"));
    contains(&out, quote! { __from_catalog(&["en"], 0); });
}

fn args_abs() -> CatalogArgs {
    let dir = manifest_dir().join("tests/i18n/valid");
    let dir = dir.to_string_lossy();
    args(quote! { default = "es", dir = #dir }).unwrap()
}
