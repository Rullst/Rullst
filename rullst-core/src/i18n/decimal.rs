//! `Decimal`: a number with visible fraction digits, for plural selection
//! and locale formatting without binary floating point.

use std::fmt;
use std::str::FromStr;

/// A decimal number written with a fixed number of fraction digits:
/// `Decimal::new(123456, 2)` is `1234.56` and `Decimal::new(10, 1)` is
/// `1.0`.
///
/// The fraction digits are part of the value, as CLDR plural rules and
/// number formatting expect: `1.0` selects English `other` ("1.0 hours"),
/// while `1` selects `one`, and `1.50` is formatted with two digits. Two
/// decimals are equal only when their units and scale are equal.
///
/// ```
/// # extern crate rullst_core as rullst;
/// use rullst::i18n::Decimal;
///
/// let price: Decimal = "1234.50".parse()?;
/// assert_eq!(price, Decimal::new(123450, 2));
/// assert_eq!(price.to_string(), "1234.50");
/// assert_eq!(Decimal::from(-7).to_string(), "-7");
/// # Ok::<(), rullst::i18n::ParseDecimalError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Decimal {
    units: i128,
    scale: u8,
}

/// Why a string is not a [`Decimal`].
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseDecimalError {
    /// Not an optional `-`, digits and an optional `.` followed by digits.
    #[error("expected a decimal number such as `-1234.56`")]
    Invalid,
    /// More digits than an `i128` holds, or more than 255 fraction digits.
    #[error("the decimal number has too many digits")]
    TooLarge,
}

impl Decimal {
    /// `units / 10^scale`: `Decimal::new(-5, 2)` is `-0.05`.
    pub const fn new(units: i128, scale: u8) -> Self {
        Self { units, scale }
    }

    /// The value without its decimal point: `12345` for `123.45`.
    pub const fn units(self) -> i128 {
        self.units
    }

    /// The number of fraction digits: `2` for `123.45`.
    pub const fn scale(self) -> u8 {
        self.scale
    }

    /// Whether the value is below zero (`-0.00` is not).
    pub const fn is_negative(self) -> bool {
        self.units < 0
    }

    /// The absolute value, keeping the scale (saturating at `i128::MAX`
    /// units).
    pub const fn abs(self) -> Self {
        Self {
            units: self.units.saturating_abs(),
            scale: self.scale,
        }
    }

    /// The integer part of the absolute value and whether every fraction
    /// digit is zero: the CLDR operands `i` and `t = 0`.
    pub(crate) fn integer_part(self) -> (u128, bool) {
        let magnitude = self.units.unsigned_abs();
        match 10u128.checked_pow(u32::from(self.scale)) {
            Some(divisor) => (magnitude / divisor, magnitude.is_multiple_of(divisor)),
            // 10^39 exceeds every `i128`, so the integer part is zero.
            None => (0, magnitude == 0),
        }
    }

    /// The digits of the absolute value as `(integer, fraction)`, the
    /// fraction exactly `scale` digits long: `("1234", "50")`.
    pub(crate) fn digits(self) -> (String, String) {
        let digits = self.units.unsigned_abs().to_string();
        let scale = usize::from(self.scale);
        if digits.len() > scale {
            let (integer, fraction) = digits.split_at(digits.len() - scale);
            (integer.to_owned(), fraction.to_owned())
        } else {
            let padding = "0".repeat(scale - digits.len());
            ("0".to_owned(), format!("{padding}{digits}"))
        }
    }
}

impl fmt::Display for Decimal {
    /// Plain ASCII digits with `.` and a leading `-`, without grouping.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (integer, fraction) = self.digits();
        let sign = if self.is_negative() { "-" } else { "" };
        if fraction.is_empty() {
            write!(formatter, "{sign}{integer}")
        } else {
            write!(formatter, "{sign}{integer}.{fraction}")
        }
    }
}

impl FromStr for Decimal {
    type Err = ParseDecimalError;

    /// Parses `-?[0-9]+(\.[0-9]+)?`, keeping every fraction digit.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (negative, unsigned) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let (integer, fraction) = match unsigned.split_once('.') {
            Some((integer, fraction)) if !fraction.is_empty() => (integer, fraction),
            Some(_) => return Err(ParseDecimalError::Invalid),
            None => (unsigned, ""),
        };
        let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        if integer.is_empty() || !all_digits(integer) || !all_digits(fraction) {
            return Err(ParseDecimalError::Invalid);
        }
        let scale = u8::try_from(fraction.len()).map_err(|_| ParseDecimalError::TooLarge)?;
        let mut units: i128 = 0;
        for byte in integer.bytes().chain(fraction.bytes()) {
            let digit = i128::from(byte - b'0');
            units = units
                .checked_mul(10)
                .and_then(|units| units.checked_add(digit))
                .ok_or(ParseDecimalError::TooLarge)?;
        }
        Ok(Self {
            units: if negative { -units } else { units },
            scale,
        })
    }
}

macro_rules! decimal_from {
    ($($ty:ty),*) => {
        $(impl From<$ty> for Decimal {
            fn from(value: $ty) -> Self {
                Self { units: i128::from(value), scale: 0 }
            }
        })*
    };
}

decimal_from!(i8, i16, i32, i64, i128, u8, u16, u32, u64);

impl From<isize> for Decimal {
    fn from(value: isize) -> Self {
        // Sign extension into `i128` is lossless for every pointer width.
        Self {
            units: value as i128,
            scale: 0,
        }
    }
}

impl From<usize> for Decimal {
    fn from(value: usize) -> Self {
        // Zero extension into `i128` is lossless up to 64-bit pointers.
        Self {
            units: value as i128,
            scale: 0,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "decimal_tests.rs"]
mod tests;
