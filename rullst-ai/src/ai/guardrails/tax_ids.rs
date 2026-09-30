//! Check-digit-validated Brazilian CPF and CNPJ masking for outbound AI text.
//!
//! A candidate is a maximal token of ASCII digits joined by single `.`, `/` or
//! `-` separators. Only an unformatted 11-digit CPF or 14-digit CNPJ, or the
//! canonical `###.###.###-##` and `##.###.###/####-##` layouts, is considered.
//! The token must not touch a letter, digit or `_`, must not repeat one digit
//! throughout, and must carry valid check digits. Every digit of a match
//! becomes `*` and its separators are kept. Alphanumeric CNPJs are not
//! recognized. Each byte is examined a constant number of times, so the pass is
//! linear in the input length.

use core::ops::Range;

const CPF_DIGITS: usize = 11;
const CNPJ_DIGITS: usize = 14;
/// Canonical CPF layout; `9` marks a digit position.
const CPF_LAYOUT: &[u8] = b"999.999.999-99";
/// Canonical CNPJ layout; `9` marks a digit position.
const CNPJ_LAYOUT: &[u8] = b"99.999.999/9999-99";
const CNPJ_FIRST_WEIGHTS: [u32; 12] = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
const CNPJ_SECOND_WEIGHTS: [u32; 13] = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];

/// Replaces the digits of every recognized CPF or CNPJ with `*`.
pub(super) fn mask_tax_ids(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut matches: Vec<Range<usize>> = Vec::new();
    let mut start = 0;
    while let Some(&byte) = bytes.get(start) {
        if !byte.is_ascii_digit() {
            start += 1;
            continue;
        }
        let end = token_end(bytes, start);
        if is_isolated(text, start, end) && bytes.get(start..end).is_some_and(is_valid_tax_id) {
            matches.push(start..end);
        }
        start = end;
    }
    if matches.is_empty() {
        return text.to_owned();
    }

    let mut masked = String::with_capacity(text.len());
    let mut pending = matches.iter().peekable();
    for (offset, character) in text.char_indices() {
        while pending.peek().is_some_and(|range| range.end <= offset) {
            pending.next();
        }
        let inside = pending.peek().is_some_and(|range| range.contains(&offset));
        masked.push(if inside && character.is_ascii_digit() {
            '*'
        } else {
            character
        });
    }
    masked
}

/// Returns the exclusive end of the token that starts with the digit at
/// `start`. A separator belongs to the token only when a digit follows it, so
/// the token always ends with a digit.
fn token_end(bytes: &[u8], start: usize) -> usize {
    let mut index = start;
    let mut end = start;
    while let Some(&byte) = bytes.get(index) {
        if byte.is_ascii_digit() {
            index += 1;
            end = index;
        } else if matches!(byte, b'.' | b'/' | b'-')
            && bytes
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_digit())
        {
            index += 1;
        } else {
            break;
        }
    }
    end
}

/// Rejects tokens embedded in a word or identifier, such as `id12345678909`.
fn is_isolated(text: &str, start: usize, end: usize) -> bool {
    let joins_word = |character: char| character.is_alphanumeric() || character == '_';
    let before = text
        .get(..start)
        .and_then(|prefix| prefix.chars().next_back());
    let after = text.get(end..).and_then(|suffix| suffix.chars().next());
    !before.is_some_and(joins_word) && !after.is_some_and(joins_word)
}

fn is_valid_tax_id(token: &[u8]) -> bool {
    let all_digits = token.iter().all(u8::is_ascii_digit);
    if (all_digits && token.len() == CPF_DIGITS) || matches_layout(token, CPF_LAYOUT) {
        return digit_values::<CPF_DIGITS>(token).is_some_and(|digits| is_valid_cpf(&digits));
    }
    if (all_digits && token.len() == CNPJ_DIGITS) || matches_layout(token, CNPJ_LAYOUT) {
        return digit_values::<CNPJ_DIGITS>(token).is_some_and(|digits| is_valid_cnpj(&digits));
    }
    false
}

fn matches_layout(token: &[u8], layout: &[u8]) -> bool {
    token.len() == layout.len()
        && token.iter().zip(layout).all(|(byte, expected)| {
            if *expected == b'9' {
                byte.is_ascii_digit()
            } else {
                byte == expected
            }
        })
}

fn digit_values<const N: usize>(token: &[u8]) -> Option<[u8; N]> {
    let mut digits = [0; N];
    let mut count = 0;
    for byte in token.iter().filter(|byte| byte.is_ascii_digit()) {
        *digits.get_mut(count)? = byte - b'0';
        count += 1;
    }
    (count == N).then_some(digits)
}

fn is_valid_cpf(digits: &[u8; CPF_DIGITS]) -> bool {
    !is_repeated(digits)
        && cpf_check_digit(&digits[..9]) == digits[9]
        && cpf_check_digit(&digits[..10]) == digits[10]
}

fn cpf_check_digit(digits: &[u8]) -> u8 {
    let first_weight = digits.len() as u32 + 1;
    let sum: u32 = digits
        .iter()
        .zip((2..=first_weight).rev())
        .map(|(digit, weight)| u32::from(*digit) * weight)
        .sum();
    let remainder = (sum * 10) % 11;
    if remainder == 10 { 0 } else { remainder as u8 }
}

fn is_valid_cnpj(digits: &[u8; CNPJ_DIGITS]) -> bool {
    !is_repeated(digits)
        && cnpj_check_digit(&digits[..12], &CNPJ_FIRST_WEIGHTS) == digits[12]
        && cnpj_check_digit(&digits[..13], &CNPJ_SECOND_WEIGHTS) == digits[13]
}

fn cnpj_check_digit(digits: &[u8], weights: &[u32]) -> u8 {
    let sum: u32 = digits
        .iter()
        .zip(weights)
        .map(|(digit, weight)| u32::from(*digit) * weight)
        .sum();
    let remainder = sum % 11;
    if remainder < 2 {
        0
    } else {
        (11 - remainder) as u8
    }
}

/// Repeated-digit sequences pass both checksums but are not issued numbers.
fn is_repeated(digits: &[u8]) -> bool {
    digits.windows(2).all(|pair| pair[0] == pair[1])
}

#[cfg(test)]
mod tests {
    use super::mask_tax_ids;
    use std::time::{Duration, Instant};

    #[test]
    fn masks_formatted_and_unformatted_valid_numbers() {
        for (input, expected) in [
            ("CPF 123.456.789-09.", "CPF ***.***.***-**."),
            ("cpf:12345678909", "cpf:***********"),
            ("CNPJ 12.345.678/0001-95", "CNPJ **.***.***/****-**"),
            ("(12345678000195)", "(**************)"),
            (
                "a 111.444.777-35 e 11144477735",
                "a ***.***.***-** e ***********",
            ),
            ("\"12345678909\",", "\"***********\","),
            ("é 123.456.789-09 ✓", "é ***.***.***-** ✓"),
        ] {
            assert_eq!(mask_tax_ids(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn leaves_invalid_embedded_and_other_digit_runs_unchanged() {
        for input in [
            // Wrong check digits.
            "123.456.789-00",
            "12345678900",
            "12.345.678/0001-90",
            "12345678000190",
            // Repeated digits pass the checksum but are not issued numbers.
            "000.000.000-00",
            "11111111111",
            "00000000000000",
            // Other lengths, including runs that contain a valid number.
            "1234567890",
            "123456789091",
            "0123456789090",
            "412345678000195",
            // Non-canonical separators or partial layouts.
            "123.456.78909",
            "123-456-789-09",
            "12.345.678.0001-95",
            "1.123.456.789-09",
            "123.456.789-09.1",
            "123.456.789-09/2026",
            // Embedded in words, identifiers or decimals.
            "id12345678909",
            "12345678909abc",
            "user_12345678909",
            "x123.456.789-09",
            "2026-09-30 10:00:00",
            "R$ 12.345.678,90",
        ] {
            assert_eq!(mask_tax_ids(input), input, "input: {input:?}");
        }
    }

    #[test]
    fn long_digit_and_separator_runs_are_scanned_in_linear_time() {
        let started = Instant::now();
        let digits = "7".repeat(400_000);
        assert_eq!(mask_tax_ids(&digits), digits);
        let separated = "1.".repeat(200_000);
        assert_eq!(mask_tax_ids(&separated), separated);
        let many = "123.456.789-09 ".repeat(20_000);
        assert_eq!(mask_tax_ids(&many), "***.***.***-** ".repeat(20_000));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "tax-ID masking took {:?}",
            started.elapsed()
        );
    }
}
