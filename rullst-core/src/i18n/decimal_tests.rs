use super::*;

#[test]
fn decimals_keep_units_scale_and_sign() {
    let price = Decimal::new(123_456, 2);
    assert_eq!(price.units(), 123_456);
    assert_eq!(price.scale(), 2);
    assert!(!price.is_negative());
    assert!(Decimal::new(-1, 3).is_negative());
    assert!(!Decimal::new(0, 3).is_negative());
    assert_eq!(Decimal::new(-15, 1).abs(), Decimal::new(15, 1));
    assert_eq!(Decimal::new(15, 1).abs(), Decimal::new(15, 1));
    assert_eq!(Decimal::new(i128::MIN, 0).abs().units(), i128::MAX);
    // Fraction digits are part of the value.
    assert_ne!(Decimal::new(10, 1), Decimal::new(1, 0));
}

#[test]
fn display_is_plain_ascii_with_every_fraction_digit() {
    let cases = [
        (Decimal::new(123_456, 2), "1234.56"),
        (Decimal::new(-5, 2), "-0.05"),
        (Decimal::new(0, 2), "0.00"),
        (Decimal::new(7, 0), "7"),
        (Decimal::new(-7, 0), "-7"),
        (Decimal::new(100, 2), "1.00"),
        (Decimal::new(5, 1), "0.5"),
        (
            Decimal::new(i128::MIN, 0),
            "-170141183460469231731687303715884105728",
        ),
        (
            Decimal::new(12, 40),
            "0.0000000000000000000000000000000000000012",
        ),
    ];
    for (decimal, text) in cases {
        assert_eq!(decimal.to_string(), text);
    }
}

#[test]
fn integer_parts_and_digits_split_at_the_scale() {
    assert_eq!(Decimal::new(123_456, 2).integer_part(), (1234, false));
    assert_eq!(Decimal::new(-123_400, 2).integer_part(), (1234, true));
    assert_eq!(Decimal::new(7, 0).integer_part(), (7, true));
    assert_eq!(Decimal::new(5, 39).integer_part(), (0, false));
    assert_eq!(Decimal::new(0, 39).integer_part(), (0, true));
    assert_eq!(
        Decimal::new(123_456, 2).digits(),
        ("1234".to_owned(), "56".to_owned())
    );
    assert_eq!(
        Decimal::new(-5, 3).digits(),
        ("0".to_owned(), "005".to_owned())
    );
    assert_eq!(
        Decimal::new(250, 3).digits(),
        ("0".to_owned(), "250".to_owned())
    );
    assert_eq!(
        Decimal::new(42, 0).digits(),
        ("42".to_owned(), String::new())
    );
}

#[test]
fn strings_parse_with_their_fraction_digits() {
    let cases = [
        ("1234.50", Decimal::new(123_450, 2)),
        ("-0.05", Decimal::new(-5, 2)),
        ("0", Decimal::new(0, 0)),
        ("007", Decimal::new(7, 0)),
        ("1.0", Decimal::new(10, 1)),
        (
            "170141183460469231731687303715884105727",
            Decimal::new(i128::MAX, 0),
        ),
    ];
    for (text, decimal) in cases {
        assert_eq!(text.parse::<Decimal>(), Ok(decimal), "{text}");
    }
    for text in [
        "", "-", ".5", "1.", "1.2.3", "+1", "1e3", " 1", "1,5", "--1", "١",
    ] {
        assert_eq!(
            text.parse::<Decimal>(),
            Err(ParseDecimalError::Invalid),
            "{text}"
        );
    }
    assert_eq!(
        "170141183460469231731687303715884105728".parse::<Decimal>(),
        Err(ParseDecimalError::TooLarge)
    );
    assert_eq!(
        format!("0.{}", "0".repeat(256)).parse::<Decimal>(),
        Err(ParseDecimalError::TooLarge)
    );
    assert_eq!(
        format!("0.{}", "0".repeat(255)).parse::<Decimal>(),
        Ok(Decimal::new(0, 255))
    );
    assert_eq!(
        ParseDecimalError::Invalid.to_string(),
        "expected a decimal number such as `-1234.56`"
    );
    assert_eq!(
        ParseDecimalError::TooLarge.to_string(),
        "the decimal number has too many digits"
    );
}

#[test]
fn integers_convert_without_fraction_digits() {
    assert_eq!(Decimal::from(-3i8), Decimal::new(-3, 0));
    assert_eq!(Decimal::from(300i16), Decimal::new(300, 0));
    assert_eq!(Decimal::from(-70_000i32), Decimal::new(-70_000, 0));
    assert_eq!(
        Decimal::from(i64::MIN),
        Decimal::new(-9_223_372_036_854_775_808, 0)
    );
    assert_eq!(Decimal::from(i128::MAX), Decimal::new(i128::MAX, 0));
    assert_eq!(Decimal::from(-5isize), Decimal::new(-5, 0));
    assert_eq!(Decimal::from(200u8), Decimal::new(200, 0));
    assert_eq!(Decimal::from(60_000u16), Decimal::new(60_000, 0));
    assert_eq!(
        Decimal::from(4_000_000_000u32),
        Decimal::new(4_000_000_000, 0)
    );
    assert_eq!(
        Decimal::from(u64::MAX),
        Decimal::new(18_446_744_073_709_551_615, 0)
    );
    assert_eq!(Decimal::from(9usize), Decimal::new(9, 0));
}

/// `(value, [0, 1, 2 digits half-even], [0, 1, 2 digits half-up])`, from
/// ICU's `halfEven` and `halfExpand` modes (ICU writes `-0` where a
/// `Decimal` zero has no sign).
const ROUNDING: &[(&str, [&str; 3], [&str; 3])] = &[
    ("2.345", ["2", "2.3", "2.34"], ["2", "2.3", "2.35"]),
    ("2.355", ["2", "2.4", "2.36"], ["2", "2.4", "2.36"]),
    ("-2.345", ["-2", "-2.3", "-2.34"], ["-2", "-2.3", "-2.35"]),
    ("-2.5", ["-2", "-2.5", "-2.50"], ["-3", "-2.5", "-2.50"]),
    ("0.5", ["0", "0.5", "0.50"], ["1", "0.5", "0.50"]),
    ("1.5", ["2", "1.5", "1.50"], ["2", "1.5", "1.50"]),
    ("2.5", ["2", "2.5", "2.50"], ["3", "2.5", "2.50"]),
    ("-0.5", ["0", "-0.5", "-0.50"], ["-1", "-0.5", "-0.50"]),
    (
        "1234.5678",
        ["1235", "1234.6", "1234.57"],
        ["1235", "1234.6", "1234.57"],
    ),
    ("9.995", ["10", "10.0", "10.00"], ["10", "10.0", "10.00"]),
    (
        "-9.995",
        ["-10", "-10.0", "-10.00"],
        ["-10", "-10.0", "-10.00"],
    ),
    ("0.05", ["0", "0.0", "0.05"], ["0", "0.1", "0.05"]),
    (
        "123456789012345678901234567890.125",
        [
            "123456789012345678901234567890",
            "123456789012345678901234567890.1",
            "123456789012345678901234567890.12",
        ],
        [
            "123456789012345678901234567890",
            "123456789012345678901234567890.1",
            "123456789012345678901234567890.13",
        ],
    ),
];

#[test]
fn rounding_matches_icu_half_even_and_half_expand() {
    for (value, even, up) in ROUNDING {
        let decimal: Decimal = value.parse().unwrap();
        for scale in 0..3u8 {
            let index = usize::from(scale);
            assert_eq!(
                decimal.round_half_even(scale).to_string(),
                even[index],
                "{value} even {scale}"
            );
            assert_eq!(
                decimal.round_half_up(scale).to_string(),
                up[index],
                "{value} up {scale}"
            );
            assert_eq!(decimal.round_half_up(scale).scale(), scale);
        }
    }
}

#[test]
fn rounding_to_more_digits_pads_or_keeps_the_value() {
    assert_eq!(
        Decimal::new(15, 1).round_half_even(3),
        Decimal::new(1500, 3)
    );
    assert_eq!(Decimal::new(15, 1).round_half_up(1), Decimal::new(15, 1));
    assert_eq!(Decimal::new(-7, 0).round_half_up(2), Decimal::new(-700, 2));
    // Padding that would overflow `i128` keeps the number as it is.
    let large = Decimal::new(i128::MAX / 5, 0);
    assert_eq!(large.round_half_even(1), large);
    assert_eq!(Decimal::new(1, 0).round_half_up(39), Decimal::new(1, 0));
    // Dropping 39 or more digits leaves zero.
    assert_eq!(
        Decimal::new(i128::MAX, 40).round_half_up(0),
        Decimal::new(0, 0)
    );
    assert_eq!(
        Decimal::new(i128::MIN, 40).round_half_even(1),
        Decimal::new(0, 1)
    );
    // Exactly 38 dropped digits still compare against the half correctly.
    let half = Decimal::new(5 * 10i128.pow(37), 38);
    assert_eq!(half.round_half_up(0), Decimal::new(1, 0));
    assert_eq!(half.round_half_even(0), Decimal::new(0, 0));
    assert_eq!(
        Decimal::new(i128::MIN, 38).round_half_up(0),
        Decimal::new(-2, 0)
    );
}
