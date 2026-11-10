//! Explicit locale order, catalog loading by `__translate!` and its cache.

use super::emit::{self, CatalogArgs};
use super::prepared::Cache;
use super::translate;
use proc_macro2::TokenStream;
use quote::quote;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn catalog(tokens: TokenStream) -> String {
    let args: CatalogArgs = syn::parse2(tokens).unwrap();
    emit::expand_in(manifest_dir(), &args).to_string()
}

fn contains(haystack: &str, needle: TokenStream) {
    let needle = needle.to_string();
    assert!(haystack.contains(&needle), "{needle}\nnot in\n{haystack}");
}

/// A catalog directory removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rullst-macros-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join("locales")).unwrap();
        Self(path)
    }

    fn write(&self, tag: &str, text: &str) {
        std::fs::write(self.0.join("locales").join(format!("{tag}.toml")), text).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn an_explicit_order_lists_the_locales_for_a_menu() {
    let out =
        catalog(quote! { default = "pt-BR", dir = "tests/i18n/valid", order = ["pt-BR", "en"] });
    contains(
        &out,
        quote! { rullst::i18n::Locales::__from_catalog(&["pt-BR", "en"], 0usize); },
    );
    contains(
        &out,
        quote! { rullst::i18n::__translate! { "tests/i18n/valid" ["pt-BR", "en"] "pt-BR" ; $($input)* } },
    );
    assert!(!out.contains("compile_error"));
    // Without `order`, locales stay sorted by tag.
    let sorted = catalog(quote! { default = "pt-BR", dir = "tests/i18n/valid" });
    contains(
        &sorted,
        quote! { __from_catalog(&["en", "pt-BR"], 1usize); },
    );
    // A trailing comma is accepted.
    let trailing =
        catalog(quote! { default = "en", dir = "tests/i18n/valid", order = ["en", "pt-BR",] });
    contains(
        &trailing,
        quote! { __from_catalog(&["en", "pt-BR"], 0usize); },
    );
}

#[test]
fn order_must_list_every_catalog_locale_once() {
    for (order, message) in [
        (
            quote! { ["en"] },
            "`order` leaves out `pt-BR` (tests/i18n/valid/pt-BR.toml); list every catalog locale",
        ),
        (quote! { ["en", "pt-BR", "en"] }, "`order` lists `en` twice"),
        (
            quote! { ["en", "pt-BR", "es"] },
            "`order` lists `es`, which has no catalog file",
        ),
        (
            quote! { ["en", "pt-br"] },
            "`order` lists `pt-br`, which has no catalog file",
        ),
    ] {
        let out = catalog(quote! { default = "en", dir = "tests/i18n/valid", order = #order });
        contains(&out, quote! { ::core::compile_error! { #message } });
        contains(
            &out,
            quote! { macro_rules! t { ($($input:tt)*) => { loop {} }; } },
        );
    }
    let twice: syn::Result<CatalogArgs> =
        syn::parse2(quote! { default = "en", order = ["en"], order = ["en"] });
    assert_eq!(
        twice.err().map(|error| error.to_string()).as_deref(),
        Some("`order` is given twice")
    );
    let not_a_list: syn::Result<CatalogArgs> = syn::parse2(quote! { default = "en", order = "en" });
    assert!(not_a_list.is_err());
}

#[test]
fn translate_reads_the_catalog_in_the_forwarded_order() {
    let out = translate::try_expand(
        manifest_dir(),
        quote! { "tests/i18n/valid" ["pt-BR", "en"] "pt-BR" ; locale, "nav.home" },
    )
    .unwrap()
    .to_string();
    contains(&out, quote! { tags: &["pt-BR", "en"], });
    contains(
        &out,
        quote! { plurals: &[rullst::i18n::__Plural::ZeroOne, rullst::i18n::__Plural::One], },
    );
    contains(&out, quote! { default: 0usize, });
    contains(
        &out,
        quote! {
            &[rullst::i18n::__Forms { plural: &[], other: &[rullst::i18n::__Segment::Text("Início")] },
              rullst::i18n::__Forms { plural: &[], other: &[rullst::i18n::__Segment::Text("Home")] }]
        },
    );
    let error = translate::try_expand(
        manifest_dir(),
        quote! { "tests/i18n/missing_key" ["en", "pt-BR"] "en" ; locale, "nav.home" },
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "the translation catalog in `tests/i18n/missing_key` has errors; `catalog!` reports them"
    );
    let malformed = translate::try_expand(manifest_dir(), quote! { ["en"] "en" ; locale, "k" });
    assert!(malformed.is_err());
}

#[test]
fn prepared_catalogs_are_reused_until_a_file_changes() {
    let scratch = Scratch::new("cache");
    scratch.write("en", "greeting = \"Hello\"\n");
    scratch.write("es", "greeting = \"Hola\"\n");
    let order = ["en".to_owned(), "es".to_owned()];
    let cache = Cache::new();
    let load = |order: &[String], default: &str| cache.load(&scratch.0, "locales", order, default);
    let first = load(&order, "en").unwrap();
    let again = load(&order, "en").unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    // Another order or default is another catalog.
    let reversed = [order[1].clone(), order[0].clone()];
    let other = load(&reversed, "en").unwrap();
    assert!(!Arc::ptr_eq(&first, &other));
    assert_eq!((other.tags.clone(), other.default), (reversed.to_vec(), 1));
    assert!(!Arc::ptr_eq(&first, &load(&order, "es").unwrap()));

    scratch.write("es", "greeting = \"¡Hola!\"\n");
    let changed = load(&order, "en").unwrap();
    assert!(!Arc::ptr_eq(&first, &changed));
    assert!(matches!(
        &changed.entries["greeting"].forms[1][0].1[..],
        [super::template::Segment::Text(text)] if text == "¡Hola!"
    ));
    assert!(Arc::ptr_eq(&changed, &load(&order, "en").unwrap()));
    // The changed catalog replaced the stale one: one entry per catalog.
    assert_eq!(cache.entries(&scratch.0), 3);

    scratch.write("es", "farewell = \"Adiós\"\n");
    assert!(load(&order, "en").is_err());
    scratch.write("es", "greeting = \"Hola\"\n");
    scratch.write("fr", "greeting = \"Bonjour\"\n");
    // A file added since `catalog!` ran is outside the forwarded order.
    assert!(load(&order, "en").is_err());
}

#[test]
fn the_cache_keeps_the_eight_most_recent_catalogs() {
    let scratch = Scratch::new("eviction");
    for index in 0..9 {
        let dir = scratch.0.join(format!("catalog{index}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("en.toml"), format!("key = \"{index}\"\n")).unwrap();
    }
    let cache = Cache::new();
    let order = ["en".to_owned()];
    let load = |index: usize| {
        cache
            .load(&scratch.0, &format!("catalog{index}"), &order, "en")
            .unwrap()
    };
    let oldest = load(0);
    let kept: Vec<_> = (1..8).map(load).collect();
    assert_eq!(cache.entries(&scratch.0), 8);
    assert!(Arc::ptr_eq(&oldest, &load(0)));
    // A ninth catalog evicts the least recently inserted one, catalog 0.
    load(8);
    assert_eq!(cache.entries(&scratch.0), 8);
    assert!(Arc::ptr_eq(&kept[0], &load(1)));
    assert!(!Arc::ptr_eq(&oldest, &load(0)));
}

#[test]
fn a_t_call_carries_where_the_catalog_is_not_what_it_holds() {
    let scratch = Scratch::new("size");
    for tag in ["en", "es", "pt-BR"] {
        let keys: String = (0..250)
            .map(|index| format!("key{index} = \"{tag} message number {index}\"\n"))
            .collect();
        scratch.write(tag, &keys);
    }
    let args: CatalogArgs = syn::parse2(quote! { default = "en" }).unwrap();
    let out = emit::expand_in(&scratch.0, &args).to_string();
    // `LOCALES`, three `include_bytes!` and a `t!` that names the directory.
    assert!(!out.contains("message number"), "{out}");
    assert!(out.len() < 2048, "{} bytes", out.len());
    let call = translate::try_expand(
        &scratch.0,
        quote! { "locales" ["en", "es", "pt-BR"] "en" ; locale, "key17" },
    )
    .unwrap()
    .to_string();
    assert!(call.contains("es message number 17"));
    assert!(!call.contains("message number 18"));
}
