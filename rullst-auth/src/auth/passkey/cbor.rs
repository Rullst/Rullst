// Custom lightweight CBOR parser for WebAuthn payload decoding
#[derive(Debug, Clone)]
#[allow(dead_code)] // Array variant retained for spec completeness; may be used by future attestation formats
pub enum CborValue {
    Integer(i64),
    ByteString(Vec<u8>),
    TextString(String),
    Array(Vec<CborValue>),
    Map(std::collections::HashMap<CborKey, CborValue>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CborKey {
    Integer(i64),
    TextString(String),
}

#[cfg_attr(mutants, mutants::skip)]
pub fn parse_cbor(bytes: &[u8]) -> Result<(CborValue, &[u8]), String> {
    if bytes.is_empty() {
        return Err("Unexpected EOF".to_string());
    }
    let head = bytes[0];
    let major = head >> 5;
    let info = head & 0x1F;
    let rest = &bytes[1..];

    let (val, rest) = match info {
        0..=23 => (info as u64, rest),
        24 => {
            if rest.is_empty() {
                return Err("Unexpected EOF".to_string());
            }
            (rest[0] as u64, &rest[1..])
        }
        25 => {
            if rest.len() < 2 {
                return Err("Unexpected EOF".to_string());
            }
            (u16::from_be_bytes([rest[0], rest[1]]) as u64, &rest[2..])
        }
        26 => {
            if rest.len() < 4 {
                return Err("Unexpected EOF".to_string());
            }
            (
                u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as u64,
                &rest[4..],
            )
        }
        27 => {
            if rest.len() < 8 {
                return Err("Unexpected EOF".to_string());
            }
            (
                u64::from_be_bytes([
                    rest[0], rest[1], rest[2], rest[3], rest[4], rest[5], rest[6], rest[7],
                ]),
                &rest[8..],
            )
        }
        _ => return Err(format!("Unsupported CBOR info: {}", info)),
    };

    match major {
        0 => Ok((CborValue::Integer(val as i64), rest)),
        1 => Ok((CborValue::Integer(-(val as i64) - 1), rest)),
        2 => {
            if rest.len() < val as usize {
                return Err("Unexpected EOF in byte string".to_string());
            }
            Ok((
                CborValue::ByteString(rest[..val as usize].to_vec()),
                &rest[val as usize..],
            ))
        }
        3 => {
            if rest.len() < val as usize {
                return Err("Unexpected EOF in text string".to_string());
            }
            let s = String::from_utf8(rest[..val as usize].to_vec())
                .map_err(|e| format!("Invalid UTF-8: {}", e))?;
            Ok((CborValue::TextString(s), &rest[val as usize..]))
        }
        4 => {
            let mut items = Vec::new();
            let mut current = rest;
            for _ in 0..val {
                let (item, next) = parse_cbor(current)?;
                items.push(item);
                current = next;
            }
            Ok((CborValue::Array(items), current))
        }
        5 => {
            let mut map = std::collections::HashMap::new();
            let mut current = rest;
            for _ in 0..val {
                let (key_val, next) = parse_cbor(current)?;
                let (val_val, next2) = parse_cbor(next)?;
                let key = match key_val {
                    CborValue::Integer(i) => CborKey::Integer(i),
                    CborValue::TextString(s) => CborKey::TextString(s),
                    _ => return Err("Invalid CBOR map key".to_string()),
                };
                map.insert(key, val_val);
                current = next2;
            }
            Ok((CborValue::Map(map), current))
        }
        _ => Err(format!("Unsupported CBOR major type: {}", major)),
    }
}
