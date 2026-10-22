//! Locale tags, templates and catalog validation.

use super::catalog::{self, Message, Source};
use super::tag::{self, PluralRule};
use super::template::{self, Segment};
use std::path::{Path, PathBuf};

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

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn canonical_tags_fix_case_and_reject_other_shapes() {
    assert_eq!(tag::canonical("pt-br").as_deref(), Some("pt-BR"));
    assert_eq!(tag::canonical("EN").as_deref(), Some("en"));
    assert_eq!(tag::canonical("zh-hant-tw").as_deref(), Some("zh-Hant-TW"));
    assert_eq!(tag::canonical("es-419").as_deref(), Some("es-419"));
    assert_eq!(tag::canonical("fil").as_deref(), Some("fil"));
    for invalid in [
        "",
        "e",
        "engl",
        "en-",
        "en_US",
        "en-1234",
        "en-U1",
        "en-12",
        "e1",
        "en-US-x",
        "en-Latn-US-1",
        "abcdefghijklmnopq",
    ] {
        assert_eq!(tag::canonical(invalid), None, "{invalid}");
    }
}

#[test]
fn tag_check_names_the_canonical_file() {
    assert_eq!(tag::check("pt-BR"), Ok(()));
    assert_eq!(
        tag::check("pt-br"),
        Err("locale `pt-br` is not in canonical form; name the file `pt-BR.toml`".into())
    );
    assert!(
        tag::check("english")
            .is_err_and(|e| e.starts_with("`english` is not a supported locale tag"))
    );
}

#[test]
fn templates_split_placeholders_and_literal_braces() {
    let text = |s: &str| Segment::Text(s.to_owned());
    let name = |s: &str| Segment::Placeholder(s.to_owned());
    assert_eq!(
        template::parse("Hello, {name}!"),
        Ok(vec![text("Hello, "), name("name"), text("!")])
    );
    assert_eq!(template::parse("{{x}} }}"), Ok(vec![text("{x} }")]));
    assert_eq!(
        template::parse("{a}{b_2}"),
        Ok(vec![name("a"), name("b_2")])
    );
    assert_eq!(template::parse(""), Ok(vec![]));
    let parsed = template::parse("{b} {a} {b}").unwrap();
    let names: Vec<String> = template::placeholders(&parsed).into_iter().collect();
    assert_eq!(names, ["a", "b"]);
}

#[test]
fn templates_reject_unbalanced_braces_and_bad_names() {
    assert_eq!(
        template::parse("Hi {name"),
        Err("has an unclosed `{`; write `{{` for a literal brace".into())
    );
    assert_eq!(
        template::parse("a } b"),
        Err("has an unmatched `}`; write `}}` for a literal brace".into())
    );
    for bad in ["{}", "{1x}", "{type}", "{_}", "{na-me}", "{é}", "{ name }"] {
        assert!(
            template::parse(bad).is_err_and(|e| e.starts_with("has an invalid placeholder")),
            "{bad}"
        );
    }
    assert!(template::parse("{_x}").is_ok());
}

#[test]
fn a_valid_catalog_flattens_keys_and_finds_the_default() {
    let sources = vec![
        source(
            "pt-BR",
            "[nav]\nhome = \"Início\"\n[n]\none = \"{count} coisa\"\nother = \"{count} coisas\"\n",
        ),
        source(
            "en",
            "[nav]\nhome = \"Home\"\n[n]\none = \"one item\"\nother = \"{count} items\"\n",
        ),
    ];
    let catalog = catalog::build(sources, "pt-BR").unwrap();
    let tags: Vec<&str> = catalog.locales.iter().map(|l| l.tag.as_str()).collect();
    assert_eq!(tags, ["en", "pt-BR"]);
    assert_eq!(catalog.default, 1);
    let en = &catalog.locales[0];
    assert_eq!(en.file, "locales/en.toml");
    assert_eq!(en.path, PathBuf::from("/abs/locales/en.toml"));
    assert_eq!(en.rule, Some(PluralRule::One));
    assert_eq!(en.messages["nav.home"], Message::Single("Home".into()));
    assert_eq!(
        en.messages["n"],
        Message::Plural(vec![
            ("one", "one item".into()),
            ("other", "{count} items".into())
        ])
    );
    assert_eq!(catalog.locales[1].rule, Some(PluralRule::ZeroOne));
    assert_eq!(catalog.locales[1].messages.len(), 2);
}

#[test]
fn missing_keys_are_reported_in_both_directions() {
    let sources = vec![
        source("en", "a = \"A\"\nb = \"B\"\n"),
        source("pt-BR", "a = \"A\"\nc = \"C\"\n"),
    ];
    assert_eq!(
        errors(sources, "en"),
        [
            "locales/pt-BR.toml: missing key `b` (defined in locales/en.toml)",
            "locales/en.toml: missing key `c` (defined in locales/pt-BR.toml)",
        ]
    );
}

#[test]
fn placeholders_and_shapes_must_agree() {
    let sources = vec![
        source("en", "g = \"Hello, {name}!\"\nn = \"items\"\n"),
        source(
            "pt-BR",
            "g = \"Olá, {nome}!\"\n[n]\none = \"coisa\"\nother = \"coisas\"\n",
        ),
    ];
    assert_eq!(
        errors(sources, "en"),
        [
            "`g` has different placeholders: locales/en.toml uses {name} but locales/pt-BR.toml uses {nome}",
            "`n` is a plural message in locales/pt-BR.toml but a single message in locales/en.toml",
        ]
    );
    let sources = vec![
        source("en", "g = \"Hello\"\n"),
        source("es", "g = \"Hola, {name}\"\n"),
    ];
    assert_eq!(
        errors(sources, "en"),
        [
            "`g` has different placeholders: locales/en.toml uses no placeholders but locales/es.toml uses {name}"
        ]
    );
}

#[test]
fn a_table_without_other_is_a_namespace() {
    let catalog = catalog::build(vec![source("en", "[n]\none = \"a\"\n")], "en").unwrap();
    assert_eq!(
        catalog.locales[0].messages["n.one"],
        Message::Single("a".into())
    );
    let catalog =
        catalog::build(vec![source("en", "[n]\nother = \"a\"\nb = 1\n")], "en").map(|_| ());
    assert_eq!(
        catalog,
        Err(vec![
            "locales/en.toml: `n.b` must be a string or a table".to_owned()
        ])
    );
}

#[test]
fn malformed_files_tags_keys_and_defaults_are_reported() {
    assert_eq!(
        errors(vec![source("pt-br", "a = \"A\"\n")], "pt-br"),
        ["locales/pt-br.toml: locale `pt-br` is not in canonical form; name the file `pt-BR.toml`"]
    );
    assert!(
        errors(vec![source("en", "a = \n")], "en")[0]
            .starts_with("locales/en.toml: invalid TOML: ")
    );
    assert_eq!(
        errors(vec![source("en", "\"a.b\" = \"x\"\n")], "en"),
        ["locales/en.toml: key `a.b` may use only ASCII letters, digits, `_` and `-`"]
    );
    assert_eq!(
        errors(vec![source("en", "a = \"Hi {name\"\n")], "en"),
        ["locales/en.toml: `a` has an unclosed `{`; write `{{` for a literal brace"]
    );
    assert_eq!(
        errors(
            vec![source("en", "a = \"A\"\n"), source("es", "a = \"A\"\n")],
            "fr"
        ),
        ["the default locale `fr` has no catalog file (found: en, es)"]
    );
}

#[test]
fn error_lists_are_bounded() {
    let text: String = (0..40).map(|i| format!("k{i} = 1\n")).collect();
    let errors = errors(vec![source("en", &text)], "en");
    assert_eq!(errors.len(), 31);
    assert_eq!(errors[30], "10 more catalog errors not shown");
}

#[test]
fn read_dir_loads_only_toml_files() {
    let mut sources = catalog::read_dir(manifest_dir(), "tests/i18n/valid/").unwrap();
    sources.sort_by(|a, b| a.tag.cmp(&b.tag));
    let tags: Vec<&str> = sources.iter().map(|s| s.tag.as_str()).collect();
    assert_eq!(tags, ["en", "pt-BR"]);
    assert_eq!(sources[0].file, "tests/i18n/valid/en.toml");
    assert_eq!(
        sources[1].path,
        manifest_dir().join("tests/i18n/valid/pt-BR.toml")
    );
    assert!(sources[1].text.contains("Início"));
}

#[test]
fn read_dir_reports_missing_empty_and_oversized_directories() {
    let missing = catalog::read_dir(manifest_dir(), "tests/i18n/nope")
        .map(|_| ())
        .unwrap_err();
    assert!(missing[0].starts_with("cannot read the catalog directory `tests/i18n/nope`: "));

    let root = std::env::temp_dir().join(format!("rullst-i18n-{}", std::process::id()));
    let empty = root.join("empty");
    let large = root.join("large");
    std::fs::create_dir_all(&empty).unwrap();
    std::fs::create_dir_all(&large).unwrap();
    std::fs::write(empty.join("README.md"), "no catalogs").unwrap();
    let big = format!("a = \"{}\"\n", "x".repeat(1024 * 1024));
    std::fs::write(large.join("en.toml"), big).unwrap();
    std::fs::write(large.join("es.toml"), "a = \"x\"\n").unwrap();
    let empty_errors = catalog::read_dir(&root, "empty").map(|_| ());
    let large_errors = catalog::read_dir(&root, "large").map(|_| ());
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        empty_errors,
        Err(vec!["no `*.toml` catalogs in `empty`".to_owned()])
    );
    assert_eq!(
        large_errors,
        Err(vec!["large/en.toml: larger than 1 MiB".to_owned()])
    );
}

#[test]
fn keys_may_use_dashes_but_not_be_empty() {
    let catalog = catalog::build(vec![source("en", "sign-in = \"Sign in\"\n")], "en").unwrap();
    assert_eq!(
        catalog.locales[0].messages["sign-in"],
        Message::Single("Sign in".into())
    );
    assert_eq!(
        errors(vec![source("en", "\"\" = \"x\"\n")], "en"),
        ["locales/en.toml: key `` may use only ASCII letters, digits, `_` and `-`"]
    );
}

#[test]
fn a_category_table_with_a_non_string_form_is_a_namespace() {
    assert_eq!(
        errors(vec![source("en", "[n]\none = 1\nother = \"x\"\n")], "en"),
        ["locales/en.toml: `n.one` must be a string or a table"]
    );
}

#[test]
fn shape_mismatches_name_the_plural_file_first() {
    let sources = vec![
        source("en", "[n]\none = \"a\"\nother = \"b\"\n"),
        source("es", "n = \"c\"\n"),
    ];
    assert_eq!(
        errors(sources, "en"),
        ["`n` is a plural message in locales/en.toml but a single message in locales/es.toml"]
    );
}

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rullst-i18n-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn read_dir_accepts_exactly_one_mebibyte() {
    let root = scratch_dir("limit");
    let line = |len: usize| format!("a = \"{}\"\n", "x".repeat(len - 7));
    std::fs::create_dir_all(root.join("at")).unwrap();
    std::fs::write(root.join("at/en.toml"), line(1024 * 1024)).unwrap();
    let at_limit = catalog::read_dir(&root, "at").map(|sources| sources.len());
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(at_limit, Ok(1));
}

#[test]
fn read_dir_accepts_at_most_64_catalogs() {
    let root = scratch_dir("count");
    for dir in ["64", "65"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    for i in 0..65 {
        std::fs::write(root.join(format!("65/l{i}.toml")), "").unwrap();
        if i < 64 {
            std::fs::write(root.join(format!("64/l{i}.toml")), "").unwrap();
        }
    }
    let at_limit = catalog::read_dir(&root, "64").map(|sources| sources.len());
    let over = catalog::read_dir(&root, "65").map(|sources| sources.len());
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(at_limit, Ok(64));
    assert_eq!(over, Err(vec!["`65` has more than 64 catalogs".to_owned()]));
}
