//! How `html!` writes values into its buffer without an intermediate
//! `String`: text escaped as [`super::escape_str`] escapes it, and integers
//! in decimal as `Display` writes them.

/// Appends `s` to `out`, escaped exactly as [`super::escape_str`] escapes it.
pub(super) fn push_escaped_str(out: &mut String, s: &str) {
    let mut last_pos = 0;
    for (i, &b) in s.as_bytes().iter().enumerate() {
        let replacement = match b {
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'&' => "&amp;",
            b'"' => "&quot;",
            b'\'' => "&#x27;",
            _ => continue,
        };
        out.push_str(&s[last_pos..i]);
        out.push_str(replacement);
        last_pos = i + 1;
    }
    out.push_str(&s[last_pos..]);
}

/// Appends `value` in decimal, after a `-` when `negative`, as `Display`
/// writes an integer of 64 bits or fewer.
pub(super) fn push_decimal(out: &mut String, negative: bool, mut value: u64) {
    // `u64::MAX` has 20 digits.
    let mut digits = [0u8; 20];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    push_digits(out, negative, &digits[start..]);
}

/// [`push_decimal`] for 128-bit integers.
pub(super) fn push_wide_decimal(out: &mut String, negative: bool, mut value: u128) {
    // `u128::MAX` has 39 digits.
    let mut digits = [0u8; 39];
    let mut start = digits.len();
    loop {
        start -= 1;
        digits[start] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    push_digits(out, negative, &digits[start..]);
}

fn push_digits(out: &mut String, negative: bool, digits: &[u8]) {
    if negative {
        out.push('-');
    }
    // Only ASCII digits were written, so the conversion cannot fail.
    out.push_str(std::str::from_utf8(digits).unwrap_or_default());
}
