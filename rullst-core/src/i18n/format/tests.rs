use super::*;
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};

fn locale(tag: &'static str) -> Locale {
    Locale { tag }
}

#[test]
fn documented_examples_match_cldr() {
    let pt = locale("pt-BR");
    assert_eq!(
        pt.format_currency(123_456, Currency::BRL),
        "R$\u{a0}1.234,56"
    );
    let date = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();
    assert_eq!(pt.format_date(&date, DateStyle::Short), "22/10/2026");
    assert_eq!(pt.format_date(&date, DateStyle::Numeric), "22/10/2026");
    let en = locale("en-US");
    assert_eq!(en.format_currency(123_456, Currency::USD), "$1,234.56");
    assert_eq!(en.format_date(&date, DateStyle::Numeric), "10/22/2026");
    assert_eq!(en.format_date(&date, DateStyle::Short), "10/22/26");
    assert_eq!(
        locale("de-DE").format_currency(123_456, Currency::EUR),
        "1.234,56\u{a0}\u{20ac}"
    );
    assert_eq!(
        locale("fr-FR").format_number(1_234_567),
        "1\u{202f}234\u{202f}567"
    );
    assert_eq!(en.format_currency(1234, Currency::JPY), "\u{a5}1,234");
}

#[test]
fn tags_use_region_then_language_data_then_english() {
    let cases = [
        ("en", "en"),
        ("en-US", "en"),
        ("en-GB", "en-GB"),
        ("en-Latn-GB", "en-GB"),
        ("en-AU", "en"),
        ("pt", "pt"),
        ("pt-BR", "pt"),
        ("pt-PT", "pt-PT"),
        ("pt-Latn-PT", "pt-PT"),
        ("es", "es"),
        ("es-ES", "es"),
        ("es-419", "es-419"),
        ("es-MX", "es-MX"),
        ("es-Latn-MX", "es-MX"),
        ("es-AR", "es-AR"),
        ("es-CL", "es-CL"),
        ("es-CO", "es-CO"),
        ("es-PE", "es-PE"),
        ("es-UY", "es-UY"),
        // A region without data uses its language's, not CLDR's `es-419`.
        ("es-VE", "es"),
        ("zh-Hant-TW", "zh"),
        ("sv", "en"),
        ("sv-SE", "en"),
    ];
    for (tag, data) in cases {
        assert_eq!(locale(tag).format_data_tag(), data, "{tag}");
    }
    // Both tables list the same locales in the same order.
    let numbers: Vec<&str> = data_numbers::NUMBERS.iter().map(|data| data.tag).collect();
    let dates: Vec<&str> = data_dates::DATES.iter().map(|data| data.tag).collect();
    assert_eq!(numbers, dates);
    // A language without data formats like English.
    let date = NaiveDate::from_ymd_opt(2026, 10, 22).unwrap();
    assert_eq!(
        locale("sv").format_date(&date, DateStyle::Long),
        "October 22, 2026"
    );
}

#[test]
fn currencies_validate_codes_and_digits() {
    let franc = Currency::new("chf", 2).unwrap();
    assert_eq!(franc.code(), "CHF");
    assert_eq!(franc.minor_digits(), 2);
    assert_eq!(Currency::new(String::from("BRL"), 2), Ok(Currency::BRL));
    assert_eq!(Currency::JPY.code(), "JPY");
    assert_eq!(Currency::JPY.minor_digits(), 0);
    for code in ["", "BR", "BRLX", "B1L", "R$1", "ÉUR"] {
        assert_eq!(
            Currency::new(code, 2),
            Err(CurrencyError::InvalidCode),
            "{code}"
        );
    }
    let constants = [
        (Currency::ARS, "ARS", 2),
        (Currency::MXN, "MXN", 2),
        (Currency::CLP, "CLP", 0),
        (Currency::COP, "COP", 2),
        (Currency::PEN, "PEN", 2),
        (Currency::UYU, "UYU", 2),
        (Currency::CAD, "CAD", 2),
        (Currency::AUD, "AUD", 2),
        (Currency::CHF, "CHF", 2),
        (Currency::CNY, "CNY", 2),
    ];
    for (constant, code, digits) in constants {
        assert_eq!(constant.code(), code);
        assert_eq!(constant.minor_digits(), digits, "{code}");
        assert_eq!(Currency::new(code.to_lowercase(), digits), Ok(constant));
    }
    assert_eq!(Currency::new("XBT", 18).map(|c| c.minor_digits()), Ok(18));
    assert_eq!(Currency::new("XBT", 19), Err(CurrencyError::TooManyDigits));
    assert_eq!(
        CurrencyError::InvalidCode.to_string(),
        "a currency code is three ASCII letters, such as `BRL`"
    );
    assert_eq!(
        CurrencyError::TooManyDigits.to_string(),
        "a currency has at most 18 minor-unit digits"
    );
}

#[test]
fn other_currencies_are_written_with_their_code() {
    let krona = Currency::new("SEK", 2).unwrap();
    assert_eq!(
        locale("en").format_currency(-1250, krona),
        "-SEK\u{a0}12.50"
    );
    assert_eq!(
        locale("pt-BR").format_currency(1250, krona),
        "SEK\u{a0}12,50"
    );
    assert_eq!(locale("de").format_currency(1250, krona), "12,50\u{a0}SEK");
    assert_eq!(
        locale("es-419").format_currency(1250, krona),
        "SEK\u{a0}12.50"
    );
    assert_eq!(locale("en").currency_name(krona), None);
    // Known codes keep their symbol with another digit count.
    let mills = Currency::new("USD", 3).unwrap();
    assert_eq!(locale("en").format_currency(12_345, mills), "$12.345");
    assert_eq!(
        locale("en").format_currency(i64::MIN, Currency::USD),
        "-$92,233,720,368,547,758.08"
    );
}

#[test]
fn numbers_keep_their_digits_and_zero_is_never_negative() {
    let en = locale("en");
    assert_eq!(en.format_number(Decimal::new(0, 2)), "0.00");
    assert_eq!(en.format_number(Decimal::new(-5, 3)), "-0.005");
    assert_eq!(
        en.format_number(Decimal::new(i128::MIN, 0)),
        "-170,141,183,460,469,231,731,687,303,715,884,105,728"
    );
    assert_eq!(
        en.format_number(Decimal::new(12_345, 1).round_half_even(0)),
        "1,234"
    );
    assert_eq!(
        en.format_number(Decimal::new(12_355, 1).round_half_up(0)),
        "1,236"
    );
    assert_eq!(en.format_number(255u8), "255");
    assert_eq!(en.format_percent(Decimal::new(-5, 3)), "-0.5%");
    assert_eq!(locale("tr").format_percent(Decimal::new(-5, 1)), "-%50");
    assert_eq!(locale("es").format_number(-1234), "-1234");
    assert_eq!(locale("es").format_number(-12_345), "-12.345");
}

#[test]
fn time_fields_cover_noon_midnight_and_zero_padding() {
    let en = locale("en");
    let at = |hour, minute, second| NaiveTime::from_hms_opt(hour, minute, second).unwrap();
    assert_eq!(
        en.format_time(&at(0, 5, 9), TimeStyle::Medium),
        "12:05:09\u{202f}AM"
    );
    assert_eq!(
        en.format_time(&at(12, 0, 0), TimeStyle::Short),
        "12:00\u{202f}PM"
    );
    assert_eq!(
        en.format_time(&at(11, 59, 0), TimeStyle::Short),
        "11:59\u{202f}AM"
    );
    assert_eq!(
        locale("ko").format_time(&at(13, 1, 0), TimeStyle::Short),
        "PM 1:01"
    );
    assert_eq!(
        locale("de").format_time(&at(9, 3, 7), TimeStyle::Medium),
        "09:03:07"
    );
    assert_eq!(
        locale("es").format_time(&at(9, 3, 7), TimeStyle::Short),
        "9:03"
    );
}

#[test]
fn dates_accept_any_chrono_value_and_unusual_years() {
    let en = locale("en");
    let utc = Utc.with_ymd_and_hms(2026, 10, 22, 15, 45, 30).unwrap();
    assert_eq!(
        en.format_date(&utc, DateStyle::Full),
        "Thursday, October 22, 2026"
    );
    assert_eq!(
        en.format_datetime(&utc, DateStyle::Medium, TimeStyle::Short),
        "Oct 22, 2026, 3:45\u{202f}PM"
    );
    assert_eq!(
        en.format_datetime(&utc.naive_utc(), DateStyle::Numeric, TimeStyle::Short),
        "10/22/2026, 3:45\u{202f}PM"
    );
    let early = NaiveDate::from_ymd_opt(5, 1, 2).unwrap();
    assert_eq!(en.format_date(&early, DateStyle::Short), "1/2/05");
    assert_eq!(en.format_date(&early, DateStyle::Long), "January 2, 5");
    let before = NaiveDate::from_ymd_opt(-1, 3, 4).unwrap();
    assert_eq!(en.format_date(&before, DateStyle::Medium), "Mar 4, -1");
    assert_eq!(en.format_date(&before, DateStyle::Short), "3/4/99");
    assert_eq!(
        locale("ja").format_date(&early, DateStyle::Short),
        "5/01/02"
    );
}

/// Expected strings are CLDR 48.0 (cldr-json `48.0.0`): each locale's
/// currency pattern, symbol and minus sign with CLDR's currency spacing.
#[test]
fn american_currencies_use_each_locale_symbol() {
    let cases = [
        ("es-AR", 123_456, Currency::ARS, "$\u{a0}1.234,56"),
        ("es-AR", 123_456, Currency::USD, "US$\u{a0}1.234,56"),
        ("es-MX", 123_456, Currency::MXN, "$1,234.56"),
        ("es-MX", 123_456, Currency::USD, "USD\u{a0}1,234.56"),
        ("es-CL", 1_235, Currency::CLP, "$1.235"),
        ("es-CL", -1_235, Currency::CLP, "$-1.235"),
        ("es-CO", 123_456, Currency::COP, "$\u{a0}1.234,56"),
        ("es-PE", -123_456, Currency::PEN, "-S/\u{a0}1,234.56"),
        ("es-UY", 123_456, Currency::UYU, "$\u{a0}1.234,56"),
        ("es-419", 123_456, Currency::USD, "USD\u{a0}1,234.56"),
        ("es", 123_456, Currency::ARS, "1234,56\u{a0}ARS"),
        ("pt-BR", 123_456, Currency::MXN, "MX$\u{a0}1.234,56"),
        ("en", 123_456, Currency::CAD, "CA$1,234.56"),
        ("en", 123_456, Currency::AUD, "A$1,234.56"),
        ("en", 123_456, Currency::CNY, "CN\u{a5}1,234.56"),
        ("zh", 123_456, Currency::CNY, "\u{a5}1,234.56"),
        ("de", 123_456, Currency::CHF, "1.234,56\u{a0}CHF"),
        ("fr", 123_456, Currency::CAD, "1\u{202f}234,56\u{a0}$CA"),
    ];
    for (tag, minor_units, currency, expected) in cases {
        assert_eq!(
            locale(tag).format_currency(minor_units, currency),
            expected,
            "{tag} {}",
            currency.code()
        );
    }
    assert_eq!(
        locale("es-MX").format_number(Decimal::new(12_345, 1)),
        "1,234.5"
    );
    assert_eq!(
        locale("es-AR").format_number(Decimal::new(12_345, 1)),
        "1.234,5"
    );
}

#[test]
fn currency_names_are_localized() {
    let cases = [
        ("en", Currency::ARS, "Argentine Peso"),
        ("en-GB", Currency::CNY, "Chinese Yuan"),
        ("es-AR", Currency::ARS, "peso argentino"),
        ("es-MX", Currency::CNY, "yuan renminbi"),
        ("pt-BR", Currency::ARS, "Peso argentino"),
        ("pt-BR", Currency::BRL, "Real brasileiro"),
        ("pt-PT", Currency::CAD, "d\u{f3}lar canadiano"),
        ("fr", Currency::CHF, "franc suisse"),
        ("de", Currency::AUD, "Australischer Dollar"),
        // A language without data names currencies in English.
        ("sv", Currency::PEN, "Peruvian Sol"),
    ];
    for (tag, currency, expected) in cases {
        assert_eq!(locale(tag).currency_name(currency), Some(expected), "{tag}");
    }
    // The name depends on the code only, not on the minor digits.
    let whole_pesos = Currency::new("COP", 0).unwrap();
    assert_eq!(
        locale("es-CO").currency_name(whole_pesos),
        Some("peso colombiano")
    );
    assert_eq!(
        locale("es-CO").format_currency(1_235, whole_pesos),
        "$\u{a0}1.235"
    );
}
