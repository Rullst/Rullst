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
