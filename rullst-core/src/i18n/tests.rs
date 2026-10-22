use super::*;

/// 65 canonical tags, one over the 64-tag cap: `xa`..`xz` twice, then
/// `xa`..`xm`. A `static` gives `Locales::new` its `'static` list without
/// leaking memory (LeakSanitizer runs this suite).
static TOO_MANY_TAGS: [&str; 65] = [
    "xa", "xb", "xc", "xd", "xe", "xf", "xg", "xh", "xi", "xj", "xk", "xl", "xm", "xn", "xo", "xp",
    "xq", "xr", "xs", "xt", "xu", "xv", "xw", "xx", "xy", "xz", "xa", "xb", "xc", "xd", "xe", "xf",
    "xg", "xh", "xi", "xj", "xk", "xl", "xm", "xn", "xo", "xp", "xq", "xr", "xs", "xt", "xu", "xv",
    "xw", "xx", "xy", "xz", "xa", "xb", "xc", "xd", "xe", "xf", "xg", "xh", "xi", "xj", "xk", "xl",
    "xm",
];

#[test]
fn a_locale_list_finds_tags_ignoring_case_and_keeps_order() {
    let locales = Locales::new(&["en", "pt-BR", "es-419"], "pt-BR").unwrap();
    assert_eq!(locales.default_locale().tag(), "pt-BR");
    assert_eq!(locales.get("PT-br").map(|l| l.tag()), Some("pt-BR"));
    assert_eq!(locales.get("es-419").map(|l| l.tag()), Some("es-419"));
    assert_eq!(locales.get("pt"), None);
    let tags: Vec<&str> = locales.iter().map(|l| l.tag()).collect();
    assert_eq!(tags, ["en", "pt-BR", "es-419"]);
    let locale = locales.get("en").unwrap();
    assert_eq!(locale.to_string(), "en");
    assert_eq!(format!("{locale}-x"), "en-x");
}

#[test]
fn a_locale_list_rejects_empty_invalid_duplicate_and_unknown_default() {
    assert_eq!(Locales::new(&[], "en").unwrap_err(), LocaleError::Empty);
    assert_eq!(
        Locales::new(&["en", "pt-br"], "en").unwrap_err(),
        LocaleError::InvalidTag("pt-br".into())
    );
    assert_eq!(
        Locales::new(&["en", "es", "en"], "en").unwrap_err(),
        LocaleError::DuplicateTag("en".into())
    );
    assert_eq!(
        Locales::new(&["en", "es"], "fr").unwrap_err(),
        LocaleError::UnknownDefault("fr".into())
    );
    let long = "x".repeat(100);
    assert_eq!(
        Locales::new(&["en"], &long).unwrap_err(),
        LocaleError::UnknownDefault("x".repeat(64))
    );
    assert_eq!(
        Locales::new(&TOO_MANY_TAGS, "xa").unwrap_err(),
        LocaleError::TooMany
    );
    assert!(
        Locales::new(&TOO_MANY_TAGS[..64], "xa")
            .is_err_and(|e| e == LocaleError::DuplicateTag("xa".into()))
    );
}

#[test]
fn locale_errors_explain_themselves() {
    assert_eq!(
        LocaleError::Empty.to_string(),
        "a locale list needs at least one tag"
    );
    assert_eq!(
        LocaleError::TooMany.to_string(),
        "a locale list holds at most 64 tags"
    );
    assert_eq!(
        LocaleError::InvalidTag("en_US".into()).to_string(),
        "`en_US` is not a canonical `language[-Script][-REGION]` tag"
    );
    assert_eq!(
        LocaleError::DuplicateTag("en".into()).to_string(),
        "`en` is listed twice"
    );
    assert_eq!(
        LocaleError::UnknownDefault("fr".into()).to_string(),
        "the default locale `fr` is not in the list"
    );
}

#[test]
fn canonical_tags_are_language_script_region() {
    for valid in ["en", "fil", "pt-BR", "zh-Hant", "zh-Hant-TW", "es-419"] {
        assert!(is_canonical_tag(valid), "{valid}");
    }
    for invalid in [
        "",
        "e",
        "engl",
        "EN",
        "En",
        "en-us",
        "pt-br",
        "zh-hant",
        "zh-HANT",
        "en_US",
        "en-",
        "en-U",
        "en-USA",
        "en-12",
        "en-1234",
        "en-US-x",
        "en-Latn-US-1",
        "e1",
        "-en",
    ] {
        assert!(!is_canonical_tag(invalid), "{invalid}");
    }
}

#[test]
fn the_default_locale_survives_a_bad_index() {
    static OUT_OF_RANGE: Locales = Locales::__from_catalog(&["es", "en"], 7);
    static EMPTY: Locales = Locales::__from_catalog(&[], 0);
    assert_eq!(OUT_OF_RANGE.default_locale().tag(), "es");
    assert_eq!(EMPTY.default_locale().tag(), "en");
    static SECOND: Locales = Locales::__from_catalog(&["es", "en"], 1);
    assert_eq!(SECOND.default_locale().tag(), "en");
}

#[test]
fn plural_counts_are_absolute_and_saturate() {
    assert_eq!(7u8.plural_count(), 7);
    assert_eq!(300u16.plural_count(), 300);
    assert_eq!(70_000u32.plural_count(), 70_000);
    assert_eq!(u64::MAX.plural_count(), u64::MAX);
    assert_eq!(9u128.plural_count(), 9);
    assert_eq!(u128::MAX.plural_count(), u64::MAX);
    assert_eq!(5usize.plural_count(), 5);
    assert_eq!((-3i8).plural_count(), 3);
    assert_eq!((-300i16).plural_count(), 300);
    assert_eq!((-70_000i32).plural_count(), 70_000);
    assert_eq!(i64::MIN.plural_count(), 9_223_372_036_854_775_808);
    assert_eq!((-9i128).plural_count(), 9);
    assert_eq!(i128::MIN.plural_count(), u64::MAX);
    assert_eq!((-5isize).plural_count(), 5);
    let count = 2u32;
    let by_ref: &u32 = &count;
    assert_eq!(PluralCount::plural_count(&by_ref), 2);
    assert_eq!(PluralCount::plural_count(&&by_ref), 2);
}

#[test]
fn decimals_select_plural_forms_with_their_fraction_digits() {
    assert_eq!(Decimal::new(-15, 1).plural_count(), 1);
    assert_eq!(Decimal::new(-15, 1).plural_decimal(), Decimal::new(15, 1));
    assert_eq!(Decimal::new(i128::MAX, 0).plural_count(), u64::MAX);
    assert_eq!(Decimal::new(25, 40).plural_count(), 0);
    assert_eq!((-3i32).plural_decimal(), Decimal::new(3, 0));
    assert_eq!(u128::MAX.plural_decimal(), Decimal::from(u64::MAX));
    let by_ref = Decimal::new(25, 1);
    assert_eq!(PluralCount::plural_decimal(&&by_ref), by_ref);
    assert_eq!(PluralCount::plural_count(&&by_ref), 2);

    let en = Locale { tag: "en" };
    assert_eq!(en.plural_category(1), Some(PluralCategory::One));
    assert_eq!(en.plural_category(-1), Some(PluralCategory::One));
    assert_eq!(
        en.plural_category(Decimal::new(10, 1)),
        Some(PluralCategory::Other)
    );
    let ar = Locale { tag: "ar" };
    assert_eq!(ar.plural_category(0), Some(PluralCategory::Zero));
    assert_eq!(ar.plural_category(105u16), Some(PluralCategory::Few));
    assert_eq!(Locale { tag: "sv" }.plural_category(1), None);
    assert_eq!(Locale { tag: "pt-AO" }.plural_category(1), None);
}
