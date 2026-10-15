use super::*;
use axum::http::HeaderValue;

static LOCALES: Locales = Locales::__from_catalog(&["en", "pt-BR", "es"], 0);

fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in pairs {
        headers.append(*name, value.parse().unwrap());
    }
    headers
}

fn accept(value: &str) -> Option<&'static str> {
    LOCALES.accept_language(value).map(|locale| locale.tag())
}

#[test]
fn the_cookie_wins_then_accept_language_then_the_default() {
    let both = headers(&[
        ("cookie", "theme=dark; rullst_locale=es"),
        ("accept-language", "pt-BR"),
    ]);
    assert_eq!(LOCALES.negotiate(&both).tag(), "es");
    let unsupported_cookie =
        headers(&[("cookie", "rullst_locale=fr"), ("accept-language", "pt-BR")]);
    assert_eq!(LOCALES.negotiate(&unsupported_cookie).tag(), "pt-BR");
    assert_eq!(
        LOCALES
            .negotiate(&headers(&[("accept-language", "de")]))
            .tag(),
        "en"
    );
    assert_eq!(LOCALES.negotiate(&HeaderMap::new()).tag(), "en");
    let mut opaque = HeaderMap::new();
    opaque.insert(
        "accept-language",
        HeaderValue::from_bytes(b"pt-BR\xff").unwrap(),
    );
    assert_eq!(LOCALES.negotiate(&opaque).tag(), "en");
}

#[test]
fn the_cookie_must_name_a_supported_locale() {
    let cookie = |value: &str| LOCALES.cookie_locale(&headers(&[("cookie", value)]));
    assert_eq!(
        cookie("rullst_locale=PT-br").map(|l| l.tag()),
        Some("pt-BR")
    );
    assert_eq!(
        cookie("a=1;rullst_locale=es ;b=2").map(|l| l.tag()),
        Some("es")
    );
    assert_eq!(
        cookie("rullst_locale=fr; rullst_locale=es").map(|l| l.tag()),
        Some("es")
    );
    assert_eq!(cookie("rullst_locale_old=es"), None);
    assert_eq!(cookie("my_rullst_locale=es"), None);
    assert_eq!(cookie("rullst_locale="), None);
    assert_eq!(
        cookie(&format!("rullst_locale=es{}", " ".repeat(40))).map(|l| l.tag()),
        Some("es")
    );
    assert_eq!(cookie(&format!("rullst_locale={}", "e".repeat(5000))), None);
    let split = headers(&[("cookie", "a=1"), ("cookie", "rullst_locale=pt-BR")]);
    assert_eq!(
        LOCALES.cookie_locale(&split).map(|l| l.tag()),
        Some("pt-BR")
    );
    let mut opaque = HeaderMap::new();
    opaque.insert(
        "cookie",
        HeaderValue::from_bytes(b"x=\xc3\xa9; rullst_locale=\xff; rullst_locale=en").unwrap(),
    );
    assert_eq!(LOCALES.cookie_locale(&opaque).map(|l| l.tag()), Some("en"));
}

#[test]
fn accept_language_orders_by_weight_then_header_order() {
    assert_eq!(accept("es;q=0.5, pt-BR;q=0.9, en;q=0.1"), Some("pt-BR"));
    assert_eq!(accept("es, pt-BR"), Some("es"));
    assert_eq!(accept("pt-BR;q=0.8, es;q=0.8"), Some("pt-BR"));
    assert_eq!(accept("de, fr;q=0.9, es;q=0.2"), Some("es"));
    assert_eq!(accept("de, fr"), None);
    assert_eq!(accept(""), None);
    assert_eq!(accept(" , ,es"), Some("es"));
    assert_eq!(accept("  es  ;  Q = 0.3 , en;q=0.2"), Some("es"));
}

#[test]
fn accept_language_matches_by_language_and_wildcard() {
    assert_eq!(accept("pt-PT"), Some("pt-BR"));
    assert_eq!(accept("PT"), Some("pt-BR"));
    assert_eq!(accept("es-MX, en;q=0.9"), Some("es"));
    assert_eq!(accept("de, *;q=0.1"), Some("en"));
    assert_eq!(accept("*, es;q=0.5"), Some("en"));
}

#[test]
fn accept_language_skips_rejected_and_malformed_ranges() {
    assert_eq!(accept("es;q=0, pt-BR;q=0.1"), Some("pt-BR"));
    assert_eq!(accept("es;q=0.000"), None);
    for malformed in ["q=abc", "q=1.5", "q=0.1234", "q=-1", "q", "q=", "q=2"] {
        assert_eq!(
            accept(&format!("es;{malformed}, pt-BR;q=0.1")),
            Some("pt-BR"),
            "{malformed}"
        );
    }
    assert_eq!(accept("es;level=1, en;q=0.5"), Some("es"));
    let range = |len: usize| format!("es-{}", "x".repeat(len - 3));
    assert_eq!(accept(&range(35)), Some("es"));
    assert_eq!(accept(&range(36)), None);
    assert_eq!(
        accept(&format!("{}, pt-BR;q=0.5", range(36))),
        Some("pt-BR")
    );
}

#[test]
fn accept_language_input_is_bounded() {
    let padded = |len: usize| format!("{}es", " ".repeat(len - 2));
    assert_eq!(accept(&padded(4096)), Some("es"));
    assert_eq!(accept(&padded(4097)), None);

    let ranges = |count: usize| {
        let mut value = "de,".repeat(count);
        value.push_str("es");
        accept(&value)
    };
    assert_eq!(ranges(31), Some("es"));
    assert_eq!(ranges(32), None);
}

#[test]
fn weights_follow_rfc_9110() {
    let cases = [
        ("0", Some(0)),
        ("0.", Some(0)),
        ("0.5", Some(500)),
        ("0.25", Some(250)),
        ("0.125", Some(125)),
        ("0.999", Some(999)),
        ("1", Some(1000)),
        ("1.", Some(1000)),
        ("1.0", Some(1000)),
        ("1.000", Some(1000)),
        ("1.001", None),
        ("1.5", None),
        ("2", None),
        (".5", None),
        ("0.1234", None),
        ("0.a", None),
        ("00.5", None),
        ("", None),
    ];
    for (value, expected) in cases {
        assert_eq!(parse_weight(value), expected, "{value}");
    }
}

#[test]
fn closest_prefers_exact_then_the_default_language_then_the_first() {
    let tags = ["en-GB", "en-US", "pt-BR"];
    assert_eq!(closest(&tags, 1, "en-gb"), Some(0));
    assert_eq!(closest(&tags, 1, "en-AU"), Some(1));
    assert_eq!(closest(&tags, 2, "en-AU"), Some(0));
    assert_eq!(closest(&tags, 0, "pt"), Some(2));
    assert_eq!(closest(&tags, 0, "fr"), None);
    assert_eq!(closest(&tags, 9, "en"), Some(0));
    assert_eq!(language("zh-Hant-TW"), "zh");
    assert_eq!(language("en"), "en");
}
