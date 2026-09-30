use super::{card_mask_count, is_textual_response, mask_card_numbers, mask_pii};
use std::time::{Duration, Instant};

/// The former per-position rescan, kept as a differential oracle for the
/// masking result. It is quadratic on long runs; only call it on short input.
fn legacy_mask_card_numbers(chars: &mut [char]) {
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut digit_indices = vec![i];
            let mut j = i + 1;
            let mut non_digits = 0;
            while j < chars.len() && non_digits < 3 {
                let c = chars[j];
                if c.is_ascii_digit() {
                    digit_indices.push(j);
                    non_digits = 0;
                } else if c == ' ' || c == '-' {
                    non_digits += 1;
                } else {
                    break;
                }
                j += 1;
            }
            if let Some(mask_count) = card_mask_count(digit_indices.len()) {
                for index in digit_indices.iter().take(mask_count) {
                    chars[*index] = '*';
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
}

fn assert_matches_legacy(input: &str) {
    let mut expected: Vec<char> = input.chars().collect();
    legacy_mask_card_numbers(&mut expected);
    let mut actual: Vec<char> = input.chars().collect();
    mask_card_numbers(&mut actual);
    assert_eq!(
        actual.iter().collect::<String>(),
        expected.iter().collect::<String>(),
        "input: {input:?}"
    );
}

#[test]
fn long_digit_runs_keep_legacy_trailing_window_masking() {
    for (input, expected) in [
        ("1234567890123456789", "***************6789"),
        ("12345678901234567890", "1***************7890"),
        ("1234567890123456789012345", "123456***************2345"),
        ("x12345678901234567890123 y", "x1234***************0123 y"),
        (
            "1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1",
            "1 1 * * * * * * * * * * * * * * * 1 1 1 1",
        ),
        (
            "1-2-3-4-5-6-7-8-9-0-1-2-3-4-5-6-7-8-9-0-1-2",
            "1-2-3-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-9-0-1-2",
        ),
        (
            "1234  5678 1234 5678 9999 1111",
            "1234  5*** **** **** **** 1111",
        ),
        (
            "1234   5678123456789999 1111",
            "1234   5*************** 1111",
        ),
    ] {
        assert_eq!(mask_pii(input), expected, "input: {input:?}");
    }
}

#[test]
fn linear_card_scan_matches_legacy_rescan() {
    for digits in 1..=45 {
        for separator in ["", " ", "-", "  ", "- ", "   ", "x"] {
            let run = vec!["7"; digits].join(separator);
            assert_matches_legacy(&run);
            assert_matches_legacy(&format!("a{run}  -12 34@b.co"));
        }
    }

    let alphabet: Vec<char> = "01234567890123456789012345678901234567  --- x@.*"
        .chars()
        .collect();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..3_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let len = (state % 64) as usize;
        let input: String = (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                alphabet[(state % alphabet.len() as u64) as usize]
            })
            .collect();
        assert_matches_legacy(&input);
    }
}

#[test]
fn over_long_digit_runs_are_masked_in_linear_time() {
    // Linear scanning finishes in milliseconds. The former per-position
    // rescan performs roughly n^2 / 2 steps (about 2 * 10^10 here) plus one
    // allocation per start position, which takes minutes.
    let started = Instant::now();

    let digits = "7".repeat(200_000);
    let masked = mask_pii(&digits);
    assert_eq!(masked.len(), digits.len());
    assert_eq!(&masked[..199_981], &digits[..199_981]);
    assert_eq!(&masked[199_981..], "***************7777");

    let spaced = "1 ".repeat(100_000);
    let masked = mask_pii(&spaced);
    assert_eq!(masked.len(), spaced.len());
    let tail_start = spaced.len() - 38;
    assert_eq!(&masked[..tail_start], &spaced[..tail_start]);
    assert_eq!(
        &masked[tail_start..],
        format!("{}1 1 1 1 ", "* ".repeat(15))
    );

    assert!(
        started.elapsed() < Duration::from_secs(5),
        "masking 400,000 characters took {:?}",
        started.elapsed()
    );
}

#[test]
fn textual_responses_are_recognized_in_any_ascii_case() {
    use axum::http::{HeaderMap, HeaderValue, header};
    for media_type in [
        "application/problem+JSON",
        "Application/Vnd.Api+Json; charset=utf-8",
        "APPLICATION/JSON",
        "Application/Atom+XML",
        "Text/HTML",
    ] {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(media_type));
        assert!(is_textual_response(&headers), "{media_type}");
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("Text/Event-Stream"),
    );
    assert!(!is_textual_response(&headers));
}
