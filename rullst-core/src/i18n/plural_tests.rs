//! CLDR cardinal rules checked against CLDR's own sample numbers
//! (`plurals.xml`, compact-exponent samples left out), cross-checked with
//! ICU 78 / CLDR 48.

use super::*;
use PluralCategory::{Few, Many, One, Other, Two, Zero};

/// Expands CLDR sample notation: `0, 2~4, 0.0~1.5` (a range keeps the
/// fraction digits of its bounds).
fn samples(notation: &str) -> Vec<Decimal> {
    let mut numbers = Vec::new();
    for item in notation.split(", ") {
        match item.split_once('~') {
            Some((start, end)) => {
                let start: Decimal = start.parse().unwrap();
                let end: Decimal = end.parse().unwrap();
                assert_eq!(start.scale(), end.scale(), "{item}");
                for units in start.units()..=end.units() {
                    numbers.push(Decimal::new(units, start.scale()));
                }
            }
            None => numbers.push(item.parse().unwrap()),
        }
    }
    numbers
}

type Samples = &'static [(PluralCategory, &'static str)];

const CLDR_SAMPLES: &[(&str, Samples)] = &[
    (
        "en",
        &[
            (One, "1"),
            (
                Other,
                "0, 2~16, 100, 1000, 10000, 100000, 1000000, 0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "de",
        &[
            (One, "1"),
            (
                Other,
                "0, 2~16, 100, 1000, 10000, 100000, 1000000, 0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "nl",
        &[
            (One, "1"),
            (
                Other,
                "0, 2~16, 100, 1000, 10000, 100000, 1000000, 0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "it",
        &[
            (One, "1"),
            (Many, "1000000"),
            (
                Other,
                "0, 2~16, 100, 1000, 10000, 100000, 0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "pt-PT",
        &[
            (One, "1"),
            (Many, "1000000"),
            (
                Other,
                "0, 2~16, 100, 1000, 10000, 100000, 0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "es",
        &[
            (One, "1, 1.0, 1.00, 1.000, 1.0000"),
            (Many, "1000000"),
            (
                Other,
                "0, 2~16, 100, 1000, 10000, 100000, 0.0~0.9, 1.1~1.6, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "fr",
        &[
            (One, "0, 1, 0.0~1.5"),
            (Many, "1000000"),
            (
                Other,
                "2~17, 100, 1000, 10000, 100000, 2.0~3.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "pt",
        &[
            (One, "0, 1, 0.0~1.5"),
            (Many, "1000000"),
            (
                Other,
                "2~17, 100, 1000, 10000, 100000, 2.0~3.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "tr",
        &[
            (One, "1, 1.0, 1.00, 1.000, 1.0000"),
            (
                Other,
                "0, 2~16, 100, 1000, 10000, 100000, 1000000, 0.0~0.9, 1.1~1.6, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "ru",
        &[
            (One, "1, 21, 31, 41, 51, 61, 71, 81, 101, 1001"),
            (Few, "2~4, 22~24, 32~34, 42~44, 52~54, 62, 102, 1002"),
            (Many, "0, 5~19, 100, 1000, 10000, 100000, 1000000"),
            (
                Other,
                "0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "uk",
        &[
            (One, "1, 21, 31, 41, 51, 61, 71, 81, 101, 1001"),
            (Few, "2~4, 22~24, 32~34, 42~44, 52~54, 62, 102, 1002"),
            (Many, "0, 5~19, 100, 1000, 10000, 100000, 1000000"),
            (
                Other,
                "0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "pl",
        &[
            (One, "1"),
            (Few, "2~4, 22~24, 32~34, 42~44, 52~54, 62, 102, 1002"),
            (Many, "0, 5~19, 100, 1000, 10000, 100000, 1000000"),
            (
                Other,
                "0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "cs",
        &[
            (One, "1"),
            (Few, "2~4"),
            (
                Many,
                "0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
            (Other, "0, 5~19, 100, 1000, 10000, 100000, 1000000"),
        ],
    ),
    (
        "ro",
        &[
            (One, "1"),
            (
                Few,
                "0, 2~16, 101, 1001, 0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
            (Other, "20~35, 100, 1000, 10000, 100000, 1000000"),
        ],
    ),
    (
        "ar",
        &[
            (Zero, "0, 0.0, 0.00, 0.000, 0.0000"),
            (One, "1, 1.0, 1.00, 1.000, 1.0000"),
            (Two, "2, 2.0, 2.00, 2.000, 2.0000"),
            (
                Few,
                "3~10, 103~110, 1003, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 103.0, 1003.0",
            ),
            (
                Many,
                "11~26, 111, 1011, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 111.0, 1011.0",
            ),
            (
                Other,
                "100~102, 200~202, 300~302, 400~402, 500~502, 600, 1000, 10000, 100000, 1000000, 0.1~0.9, 1.1~1.7, 10.1, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "he",
        &[
            (One, "1, 0.0~0.9, 0.00~0.05"),
            (Two, "2"),
            (
                Other,
                "0, 3~17, 100, 1000, 10000, 100000, 1000000, 1.0~2.5, 3.0~3.4, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
            ),
        ],
    ),
    (
        "ja",
        &[(
            Other,
            "0~15, 100, 1000, 10000, 100000, 1000000, 0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
        )],
    ),
    (
        "zh",
        &[(
            Other,
            "0~15, 100, 1000, 10000, 100000, 1000000, 0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
        )],
    ),
    (
        "ko",
        &[(
            Other,
            "0~15, 100, 1000, 10000, 100000, 1000000, 0.0~1.5, 10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0",
        )],
    ),
];

#[test]
fn every_language_matches_the_cldr_samples() {
    let mut checked = 0;
    for (tag, categories) in CLDR_SAMPLES {
        let rule = __Plural::for_tag(tag).unwrap();
        for (category, notation) in *categories {
            for number in samples(notation) {
                assert_eq!(rule.category(number), *category, "{tag} {number}");
                let negative = Decimal::new(-number.units(), number.scale());
                assert_eq!(rule.category(negative), *category, "{tag} {negative}");
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 1058);
}

#[test]
fn tags_map_to_their_language_rule() {
    let cases = [
        ("en", Some(__Plural::One)),
        ("en-GB", Some(__Plural::One)),
        ("de-AT", Some(__Plural::One)),
        ("nl", Some(__Plural::One)),
        ("it", Some(__Plural::OneMany)),
        ("pt-PT", Some(__Plural::OneMany)),
        ("pt", Some(__Plural::ZeroOne)),
        ("pt-BR", Some(__Plural::ZeroOne)),
        ("pt-Latn-BR", Some(__Plural::ZeroOne)),
        ("pt-AO", None),
        ("fr-CA", Some(__Plural::ZeroOne)),
        ("tr", Some(__Plural::ExactOne)),
        ("es-419", Some(__Plural::ExactOneMany)),
        ("ru", Some(__Plural::EastSlavic)),
        ("uk", Some(__Plural::EastSlavic)),
        ("pl", Some(__Plural::Polish)),
        ("cs", Some(__Plural::Czech)),
        ("ro", Some(__Plural::Romanian)),
        ("ar-EG", Some(__Plural::Arabic)),
        ("he", Some(__Plural::Hebrew)),
        ("ja", Some(__Plural::Other)),
        ("zh-Hant", Some(__Plural::Other)),
        ("ko", Some(__Plural::Other)),
        ("sv", None),
        ("", None),
    ];
    for (tag, rule) in cases {
        assert_eq!(__Plural::for_tag(tag), rule, "{tag}");
    }
}

#[test]
fn large_values_and_many_fraction_digits_keep_their_operands() {
    // Integer parts beyond `u64` still use their last digits.
    let huge = Decimal::new(10i128.pow(30) + 21, 0);
    assert_eq!(__Plural::EastSlavic.category(huge), One);
    assert_eq!(
        __Plural::ZeroOne.category(Decimal::new(10i128.pow(30), 0)),
        Many
    );
    // A scale beyond 38 digits leaves an integer part of 0.
    let tiny = Decimal::new(5, 40);
    assert_eq!(__Plural::Hebrew.category(tiny), One);
    assert_eq!(__Plural::Arabic.category(tiny), Other);
    assert_eq!(__Plural::Arabic.category(Decimal::new(0, 40)), Zero);
    assert_eq!(__Plural::Czech.category(Decimal::new(i128::MIN, 0)), Other);
    assert_eq!(__Plural::Hebrew.category(Decimal::from(2)), Two);
    assert_eq!(__Plural::Romanian.category(Decimal::from(119)), Few);
    assert_eq!(__Plural::Romanian.category(Decimal::from(120)), Other);
}

#[test]
fn categories_have_their_cldr_names() {
    let names: Vec<&str> = [Zero, One, Two, Few, Many, Other]
        .into_iter()
        .map(PluralCategory::as_str)
        .collect();
    assert_eq!(names, ["zero", "one", "two", "few", "many", "other"]);
}
