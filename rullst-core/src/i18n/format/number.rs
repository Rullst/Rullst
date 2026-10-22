//! Numbers, percentages and money from a locale's [`NumberData`].

use super::data_numbers::CURRENCIES;
use super::{Currency, NumberData};
use crate::i18n::Decimal;

/// Groups `integer` by thousands and appends `fraction` after the decimal
/// separator.
fn digits(data: &NumberData, integer: &str, fraction: &str) -> String {
    let length = integer.len();
    let grouped = length >= 3 + data.min_grouping;
    let mut out = String::new();
    for (index, digit) in integer.chars().enumerate() {
        if grouped && index > 0 && (length - index).is_multiple_of(3) {
            out.push_str(data.group);
        }
        out.push(digit);
    }
    if !fraction.is_empty() {
        out.push_str(data.decimal);
        out.push_str(fraction);
    }
    out
}

/// Puts `body` where the pattern's `#` is.
fn apply(pattern: &str, body: &str) -> String {
    pattern.replacen('#', body, 1)
}

/// Picks the positive or negative pattern.
fn signed<'a>(patterns: &[&'a str; 2], negative: bool) -> &'a str {
    let [positive, negative_pattern] = patterns;
    if negative { negative_pattern } else { positive }
}

pub(super) fn number(data: &NumberData, value: Decimal) -> String {
    let (integer, fraction) = value.digits();
    let body = digits(data, &integer, &fraction);
    if value.is_negative() {
        apply(data.negative, &body)
    } else {
        body
    }
}

pub(super) fn percent(data: &NumberData, ratio: Decimal) -> String {
    let (integer, mut fraction) = ratio.digits();
    // Multiplying by 100 moves two fraction digits into the integer part.
    while fraction.len() < 2 {
        fraction.push('0');
    }
    let (moved, rest) = fraction.split_at(2);
    let joined = format!("{integer}{moved}");
    let trimmed = joined.trim_start_matches('0');
    let integer = if trimmed.is_empty() { "0" } else { trimmed };
    let body = digits(data, integer, rest);
    apply(signed(&data.percent, ratio.is_negative()), &body)
}

pub(super) fn currency(data: &NumberData, minor_units: i64, currency: Currency) -> String {
    let amount = Decimal::new(i128::from(minor_units), currency.minor_digits());
    let (integer, fraction) = amount.digits();
    let body = digits(data, &integer, &fraction);
    let negative = amount.is_negative();
    let code = currency.code();
    let pattern = CURRENCIES
        .iter()
        .position(|known| *known == code)
        .and_then(|index| data.currency.get(index));
    match pattern {
        Some(patterns) => apply(signed(patterns, negative), &body),
        None => {
            let [.., other] = &data.currency;
            apply(&signed(other, negative).replace('¤', code), &body)
        }
    }
}
