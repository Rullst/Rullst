//! Locale formatting of numbers, percentages, money, dates and times.
//!
//! The data are CLDR 48 tables for 20 locales (`ar`, `cs`, `de`, `en`,
//! `en-GB`, `es`, `fr`, `he`, `it`, `ja`, `ko`, `nl`, `pl`, `pt`, `pt-PT`,
//! `ro`, `ru`, `tr`, `uk`, `zh`), generated from ICU 78 and from CLDR's date
//! patterns, and compiled in: nothing is loaded at runtime. A tag uses its
//! `language-REGION` data when listed (`en-GB`, `pt-PT`), else its
//! language's (`pt-BR` → `pt`, `es-MX` → `es`, `zh-Hant` → `zh`), else
//! `en`'s. Every language listed writes Latin digits and the Gregorian
//! calendar.

mod data_dates;
mod data_numbers;
mod date;
mod number;

use super::{Decimal, Locale};
use chrono::{Datelike, Timelike};

/// How much of a date to write, from CLDR's date formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DateStyle {
    /// Digits, sometimes a two-digit year: `10/22/26`, `22/10/2026`.
    Short,
    /// An abbreviated month or digits: `Oct 22, 2026`, `22 de out. de 2026`.
    Medium,
    /// The month's name: `October 22, 2026`, `22 de outubro de 2026`.
    Long,
    /// The weekday and month names: `Thursday, October 22, 2026`.
    Full,
    /// Day, month and full year in digits (CLDR's `yMd`, what browsers
    /// show by default): `10/22/2026`, `22/10/2026`, `22.10.2026`.
    Numeric,
}

/// How much of a time of day to write, from CLDR's time formats. Time zone
/// names are not written, so CLDR's long and full time styles are absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TimeStyle {
    /// Hours and minutes: `3:45 PM`, `15:45`.
    Short,
    /// Hours, minutes and seconds: `3:45:30 PM`, `15:45:30`.
    Medium,
}

/// An ISO 4217 currency and its number of minor-unit digits, for
/// [`Locale::format_currency`].
///
/// `BRL`, `USD`, `EUR`, `GBP` and `JPY` are written with each locale's
/// symbol (`R$`, `US$`, `$`, `€`, `£`, `¥`); any other code is written as
/// the code (`CHF 12.50`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Currency {
    code: [u8; 3],
    minor_digits: u8,
}

/// Why [`Currency::new`] rejected a currency.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CurrencyError {
    /// The code is not three ASCII letters.
    #[error("a currency code is three ASCII letters, such as `BRL`")]
    InvalidCode,
    /// More than 18 minor-unit digits.
    #[error("a currency has at most 18 minor-unit digits")]
    TooManyDigits,
}

impl Currency {
    /// Brazilian real, 2 minor digits (centavos).
    pub const BRL: Self = Self::known(*b"BRL", 2);
    /// US dollar, 2 minor digits (cents).
    pub const USD: Self = Self::known(*b"USD", 2);
    /// Euro, 2 minor digits (cents).
    pub const EUR: Self = Self::known(*b"EUR", 2);
    /// Pound sterling, 2 minor digits (pence).
    pub const GBP: Self = Self::known(*b"GBP", 2);
    /// Japanese yen, no minor digits.
    pub const JPY: Self = Self::known(*b"JPY", 0);

    const fn known(code: [u8; 3], minor_digits: u8) -> Self {
        Self { code, minor_digits }
    }

    /// A currency by its ISO 4217 code (case-insensitive) and the number
    /// of digits of its minor unit: `Currency::new("CHF", 2)`.
    pub fn new(code: impl Into<String>, minor_digits: u8) -> Result<Self, CurrencyError> {
        let code = code.into();
        let bytes: [u8; 3] = code
            .as_bytes()
            .try_into()
            .map_err(|_| CurrencyError::InvalidCode)?;
        if !bytes.iter().all(u8::is_ascii_alphabetic) {
            return Err(CurrencyError::InvalidCode);
        }
        if minor_digits > 18 {
            return Err(CurrencyError::TooManyDigits);
        }
        Ok(Self {
            code: bytes.map(|byte| byte.to_ascii_uppercase()),
            minor_digits,
        })
    }

    /// The upper-case ISO code: `"BRL"`.
    pub fn code(&self) -> &str {
        std::str::from_utf8(&self.code).unwrap_or_default()
    }

    /// Digits of the minor unit: 2 for cents, 0 for yen.
    pub fn minor_digits(&self) -> u8 {
        self.minor_digits
    }
}

/// One locale's number symbols. Patterns hold `#` where the digits go.
pub(super) struct NumberData {
    pub(super) tag: &'static str,
    pub(super) decimal: &'static str,
    pub(super) group: &'static str,
    /// Integer digits needed before grouping starts, beyond three: 1
    /// groups `1,234`; 2 starts at `12 345`.
    pub(super) min_grouping: usize,
    pub(super) negative: &'static str,
    /// Positive and negative percent patterns.
    pub(super) percent: [&'static str; 2],
    /// Positive and negative patterns for `data_numbers::CURRENCIES`, then
    /// for any other code (`¤`).
    pub(super) currency: [[&'static str; 2]; 6],
}

/// One locale's date and time patterns and names.
pub(super) struct DateData {
    pub(super) tag: &'static str,
    /// Short, medium, long, full and numeric (`yMd`) date patterns.
    pub(super) date: [&'static str; 5],
    /// Short and medium time patterns.
    pub(super) time: [&'static str; 2],
    /// Date (`{1}`) and time (`{0}`) joined, by date style.
    pub(super) glue: [&'static str; 4],
    /// Abbreviated and wide month names, January first.
    pub(super) months: [&'static str; 2],
    /// Wide weekday names, Monday first.
    pub(super) weekdays: &'static str,
    /// The `a` field before and after noon.
    pub(super) day_periods: [&'static str; 2],
}

/// The number table index for a canonical tag: `language-REGION`, else
/// the language, else `en` (index 0).
fn data_index(tag: &str) -> usize {
    let mut subtags = tag.split('-');
    let language = subtags.next().unwrap_or_default();
    let region = subtags.find(|subtag| subtag.len() != 4);
    let find = |wanted: &str| {
        data_numbers::NUMBERS
            .iter()
            .position(|data| data.tag == wanted)
    };
    region
        .and_then(|region| find(&format!("{language}-{region}")))
        .or_else(|| find(language))
        .unwrap_or(0)
}

impl Locale {
    fn number_data(&self) -> &'static NumberData {
        let numbers = &data_numbers::NUMBERS;
        numbers.get(data_index(self.tag)).unwrap_or(&numbers[0])
    }

    fn date_data(&self) -> &'static DateData {
        let tag = self.format_data_tag();
        let dates = &data_dates::DATES;
        dates
            .iter()
            .find(|data| data.tag == tag)
            .unwrap_or(&dates[0])
    }

    /// The tag whose CLDR data formats this locale: `"pt"` for `pt-BR`,
    /// `"en"` for a language without data such as `sv`.
    pub fn format_data_tag(&self) -> &'static str {
        self.number_data().tag
    }

    /// A number with the locale's decimal and grouping separators and
    /// minus sign, keeping every fraction digit of the [`Decimal`]:
    /// `1,234.5` (en), `1.234,5` (pt-BR), `1 234,5` (fr, with U+202F).
    ///
    /// Integers convert directly: `locale.format_number(1234)`. Round first
    /// with [`Decimal::round_half_even`] to show fewer digits.
    pub fn format_number(&self, value: impl Into<Decimal>) -> String {
        number::number(self.number_data(), value.into())
    }

    /// A ratio as a percentage, `0.25` → `25%` (en), `25 %` (fr), `%25`
    /// (tr). The fraction digits beyond the hundredths stay visible:
    /// `Decimal::new(256, 3)` → `25.6%`.
    pub fn format_percent(&self, ratio: impl Into<Decimal>) -> String {
        number::percent(self.number_data(), ratio.into())
    }

    /// An amount of money given in integer minor units, so no floating
    /// point rounding is involved: `format_currency(123456, Currency::BRL)`
    /// is `R$ 1.234,56` in `pt-BR` and `R$1,234.56` in `en`, and
    /// `format_currency(1234, Currency::JPY)` is `¥1,234`.
    pub fn format_currency(&self, minor_units: i64, currency: Currency) -> String {
        number::currency(self.number_data(), minor_units, currency)
    }

    /// A calendar date in the locale's pattern for `style`:
    /// `22/10/2026` (pt-BR, `Short`), `October 22, 2026` (en, `Long`).
    ///
    /// Takes any `chrono` date (`NaiveDate`, `NaiveDateTime`, or a
    /// `DateTime` in its own time zone; convert a UTC timestamp to the
    /// visitor's zone first). Years before 1 are written as ISO numbers
    /// (`0`, `-1`).
    pub fn format_date(&self, date: &impl Datelike, style: DateStyle) -> String {
        date::date(self.date_data(), date, style)
    }

    /// A time of day in the locale's pattern for `style`: `3:45 PM` (en,
    /// with U+202F before `PM`), `15:45` (pt-BR).
    pub fn format_time(&self, time: &impl Timelike, style: TimeStyle) -> String {
        date::time(self.date_data(), time, style)
    }

    /// A date and time joined as the locale does for the date style:
    /// `October 22, 2026 at 3:45 PM` (en, `Long` and `Short`),
    /// `22/10/2026, 15:45` (pt-BR, `Short` and `Short`). `Numeric` dates
    /// join like `Short` ones.
    pub fn format_datetime(
        &self,
        value: &(impl Datelike + Timelike),
        date_style: DateStyle,
        time_style: TimeStyle,
    ) -> String {
        date::datetime(self.date_data(), value, date_style, time_style)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "cldr_tests.rs"]
mod cldr_tests;
