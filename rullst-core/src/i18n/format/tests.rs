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
        ("es-419", "es"),
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
    let franc = Currency::new("CHF", 2).unwrap();
    assert_eq!(
        locale("en").format_currency(-1250, franc),
        "-CHF\u{a0}12.50"
    );
    assert_eq!(
        locale("pt-BR").format_currency(1250, franc),
        "CHF\u{a0}12,50"
    );
    assert_eq!(locale("de").format_currency(1250, franc), "12,50\u{a0}CHF");
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
