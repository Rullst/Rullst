//! CLDR plural rules and the forms a plural message must define.

use super::catalog::{self, Message, Source};
use super::tag::PluralRule;
use std::path::PathBuf;

fn source(tag: &str, text: &str) -> Source {
    Source {
        tag: tag.to_owned(),
        file: format!("locales/{tag}.toml"),
        path: PathBuf::from(format!("/abs/locales/{tag}.toml")),
        text: text.to_owned(),
    }
}

fn errors(sources: Vec<Source>, default: &str) -> Vec<String> {
    match catalog::build(sources, default) {
        Ok(_) => panic!("the catalog should not build"),
        Err(errors) => errors,
    }
}

#[test]
fn plural_rules_follow_cldr_by_language() {
    let cases = [
        ("en", Some(PluralRule::One)),
        ("en-US", Some(PluralRule::One)),
        ("de", Some(PluralRule::One)),
        ("nl", Some(PluralRule::One)),
        ("it", Some(PluralRule::OneMany)),
        ("pt-PT", Some(PluralRule::OneMany)),
        ("fr", Some(PluralRule::ZeroOne)),
        ("fr-CA", Some(PluralRule::ZeroOne)),
        ("pt", Some(PluralRule::ZeroOne)),
        ("pt-BR", Some(PluralRule::ZeroOne)),
        ("pt-Latn-BR", Some(PluralRule::ZeroOne)),
        ("pt-AO", None),
        ("tr", Some(PluralRule::ExactOne)),
        ("es-419", Some(PluralRule::ExactOneMany)),
        ("ru", Some(PluralRule::EastSlavic)),
        ("uk", Some(PluralRule::EastSlavic)),
        ("pl", Some(PluralRule::Polish)),
        ("cs", Some(PluralRule::Czech)),
        ("ro", Some(PluralRule::Romanian)),
        ("ar", Some(PluralRule::Arabic)),
        ("he", Some(PluralRule::Hebrew)),
        ("ja", Some(PluralRule::Other)),
        ("zh-Hant", Some(PluralRule::Other)),
        ("ko", Some(PluralRule::Other)),
        ("sv", None),
    ];
    for (tag, rule) in cases {
        assert_eq!(PluralRule::for_tag(tag), rule, "{tag}");
    }
}

#[test]
fn each_rule_names_its_forms_and_runtime_variant() {
    let rules = [
        (PluralRule::Other, "Other", &["other"][..], false),
        (PluralRule::One, "One", &["one", "other"], false),
        (PluralRule::OneMany, "OneMany", &["one", "other"], true),
        (PluralRule::ZeroOne, "ZeroOne", &["one", "other"], true),
        (PluralRule::ExactOne, "ExactOne", &["one", "other"], false),
        (
            PluralRule::ExactOneMany,
            "ExactOneMany",
            &["one", "other"],
            true,
        ),
        (
            PluralRule::EastSlavic,
            "EastSlavic",
            &["one", "few", "many", "other"],
            false,
        ),
        (
            PluralRule::Polish,
            "Polish",
            &["one", "few", "many", "other"],
            false,
        ),
        (
            PluralRule::Czech,
            "Czech",
            &["one", "few", "many", "other"],
            false,
        ),
        (
            PluralRule::Romanian,
            "Romanian",
            &["one", "few", "other"],
            false,
        ),
        (
            PluralRule::Arabic,
            "Arabic",
            &["zero", "one", "two", "few", "many", "other"],
            false,
        ),
        (
            PluralRule::Hebrew,
            "Hebrew",
            &["one", "two", "other"],
            false,
        ),
    ];
    for (rule, variant, required, many) in rules {
        assert_eq!(rule.variant(), variant);
        assert_eq!(rule.required(), required, "{variant}");
        assert_eq!(rule.optional_many(), many, "{variant}");
        assert_eq!(
            rule.allows("many"),
            many || required.contains(&"many"),
            "{variant}"
        );
        assert!(rule.allows("other"), "{variant}");
        assert_eq!(rule.allows("zero"), variant == "Arabic", "{variant}");
    }
}

#[test]
fn plural_forms_must_match_the_language_rule() {
    let plural = "[n]\nother = \"{count} x\"\n";
    let with_one = "[n]\none = \"one x\"\nother = \"{count} x\"\n";
    assert_eq!(
        errors(vec![source("en", plural)], "en"),
        [
            "locales/en.toml: plural message `n` needs a `one` form for `en`, which uses `one` and `other`"
        ]
    );
    assert_eq!(
        errors(vec![source("ja", with_one)], "ja"),
        ["locales/ja.toml: `ja` has no `one` plural form; remove `one` from `n`"]
    );
    assert!(catalog::build(vec![source("ja", plural)], "ja").is_ok());
    let unsupported = errors(vec![source("sv", with_one)], "sv");
    assert_eq!(unsupported.len(), 1);
    assert!(unsupported[0].starts_with(
        "locales/sv.toml: `n` is a plural message, but Rullst has no plural rule for `sv` \
         (supported: ar, cs, de, en, es, fr, he, it, ja, ko, nl, pl, pt, pt-BR, pt-PT, ro, ru, tr, uk, zh)"
    ));
    assert!(catalog::build(vec![source("sv", "n = \"x\"\n")], "sv").is_ok());
    assert_eq!(
        errors(
            vec![source(
                "en",
                "[n]\none = \"a\"\nfew = \"b\"\nother = \"c\"\n"
            )],
            "en"
        ),
        ["locales/en.toml: `en` has no `few` plural form; remove `few` from `n`"]
    );
}

#[test]
fn languages_with_more_forms_require_each_of_them() {
    let ru = "[n]\none = \"{count} файл\"\nfew = \"{count} файла\"\nmany = \"{count} файлов\"\nother = \"{count} файла\"\n";
    let catalog = catalog::build(vec![source("ru", ru), source("uk", ru)], "ru").unwrap();
    assert_eq!(catalog.locales[0].rule, Some(PluralRule::EastSlavic));
    let pl = "[n]\none = \"plik\"\nfew = \"pliki\"\nmany = \"plików\"\nother = \"pliku\"\n";
    assert!(catalog::build(vec![source("pl", pl)], "pl").is_ok());
    let ar = "[n]\nzero = \"0\"\none = \"1\"\ntwo = \"2\"\nfew = \"3\"\nmany = \"11\"\nother = \"100\"\n";
    assert!(catalog::build(vec![source("ar", ar)], "ar").is_ok());

    let incomplete = "[n]\none = \"{count} файл\"\nother = \"{count} файла\"\n";
    assert_eq!(
        errors(vec![source("ru", incomplete)], "ru"),
        [
            "locales/ru.toml: plural message `n` needs `few` and `many` forms for `ru`, \
             which uses `one`, `few`, `many` and `other`"
        ]
    );
    let pl_without_many = "[n]\none = \"plik\"\nfew = \"pliki\"\nother = \"pliku\"\n";
    assert_eq!(
        errors(vec![source("pl", pl_without_many)], "pl"),
        [
            "locales/pl.toml: plural message `n` needs a `many` form for `pl`, \
             which uses `one`, `few`, `many` and `other`"
        ]
    );
    assert_eq!(
        errors(
            vec![source("ar", "[n]\none = \"1\"\nother = \"x\"\n")],
            "ar"
        ),
        [
            "locales/ar.toml: plural message `n` needs `zero`, `two`, `few` and `many` forms for `ar`, \
             which uses `zero`, `one`, `two`, `few`, `many` and `other`"
        ]
    );
    let he = "[n]\none = \"1\"\nother = \"x\"\nmany = \"m\"\n";
    assert_eq!(
        errors(vec![source("he", he)], "he"),
        [
            "locales/he.toml: plural message `n` needs a `two` form for `he`, which uses `one`, `two` and `other`",
            "locales/he.toml: `he` has no `many` plural form; remove `many` from `n`"
        ]
    );
}

#[test]
fn many_is_optional_where_it_only_means_millions() {
    let with_many =
        "[n]\none = \"un\"\nmany = \"{count} de fichiers\"\nother = \"{count} fichiers\"\n";
    let without = "[n]\none = \"un\"\nother = \"{count} fichiers\"\n";
    for tag in ["fr", "es", "it", "pt-BR", "pt-PT"] {
        assert!(
            catalog::build(vec![source(tag, with_many)], tag).is_ok(),
            "{tag}"
        );
        assert!(
            catalog::build(vec![source(tag, without)], tag).is_ok(),
            "{tag}"
        );
    }
    assert_eq!(
        errors(vec![source("en", with_many)], "en"),
        ["locales/en.toml: `en` has no `many` plural form; remove `many` from `n`"]
    );
    let catalog = catalog::build(vec![source("fr", with_many)], "fr").unwrap();
    assert_eq!(
        catalog.locales[0].messages["n"],
        Message::Plural(vec![
            ("one", "un".into()),
            ("many", "{count} de fichiers".into()),
            ("other", "{count} fichiers".into())
        ])
    );
}
