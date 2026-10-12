//! Nesting through arrays, map keys and map values stops after 64 levels.
use super::*;

fn nesting_error(bytes: &[u8]) -> bool {
    matches!(parse_cbor(bytes), Err(AuthError::CborParseError(message)) if message.contains("nesting limit"))
}

#[test]
fn arrays_nest_to_exactly_64_levels() {
    let mut deepest = vec![0x81; 64];
    deepest.push(0x00);
    assert!(parse_cbor(&deepest).is_ok());
    deepest.insert(0, 0x81);
    assert!(nesting_error(&deepest));
}

#[test]
fn map_keys_and_values_count_toward_the_nesting_limit() {
    // {0: {0: ... {0: 0}}}, nested through values.
    let mut values = [0xA1, 0x00].repeat(64);
    values.push(0x00);
    assert!(parse_cbor(&values).is_ok());
    values.splice(0..0, [0xA1, 0x00]);
    assert!(nesting_error(&values));
    // {{{...{0: 0}...}: 0}: 0}, nested through keys.
    let mut keys = vec![0xA1; 65];
    keys.extend([0x00; 2]);
    keys.extend([0x00; 64]);
    assert!(nesting_error(&keys));
}
