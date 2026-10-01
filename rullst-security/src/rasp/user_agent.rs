//! User-Agent handling for the `powershell` command keyword.
//!
//! The stock User-Agent of every PowerShell HTTP cmdlet names the shell as a
//! product token: `WindowsPowerShell/5.1.22621.2506` (Windows PowerShell 5.1)
//! or `PowerShell/7.4.1` (PowerShell 7, including `-preview.N` versions). Only
//! that exact form is exempt, and only in `User-Agent`: the token must start
//! the value or follow whitespace, carry a version that starts with a digit
//! and contains only ASCII letters, digits, `.` and `-`, and end the value or
//! precede whitespace. Execution syntax such as `powershell -enc`,
//! `powershell.exe`, `|powershell/7` or `PowerShell/7;whoami` still matches,
//! as does every other signature in the same value.

/// The command keyword that a PowerShell product token also contains.
pub(super) const POWERSHELL: &str = "powershell";

const WINDOWS_PREFIX: &[u8] = b"windows";

/// Whether `text` contains `powershell` (ASCII case-insensitive) anywhere
/// other than inside a PowerShell product token.
pub(super) fn has_powershell_outside_product_tokens(text: &str) -> bool {
    let bytes = text.as_bytes();
    let needle = POWERSHELL.as_bytes();
    bytes
        .windows(needle.len())
        .enumerate()
        .any(|(start, window)| {
            window.eq_ignore_ascii_case(needle) && !is_product_token(bytes, start)
        })
}

/// Whether the `powershell` occurrence at `start` is a complete
/// `[Windows]PowerShell/<version>` product token.
fn is_product_token(bytes: &[u8], start: usize) -> bool {
    let token_start = match start.checked_sub(WINDOWS_PREFIX.len()) {
        Some(prefix_start)
            if bytes
                .get(prefix_start..start)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(WINDOWS_PREFIX)) =>
        {
            prefix_start
        }
        _ => start,
    };
    let starts_token = match token_start.checked_sub(1) {
        None => true,
        Some(before) => bytes.get(before).is_some_and(u8::is_ascii_whitespace),
    };
    if !starts_token {
        return false;
    }

    let mut index = start + POWERSHELL.len();
    if bytes.get(index) != Some(&b'/') {
        return false;
    }
    index += 1;
    if !bytes.get(index).is_some_and(u8::is_ascii_digit) {
        return false;
    }
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    {
        index += 1;
    }
    bytes.get(index).is_none_or(u8::is_ascii_whitespace)
}
