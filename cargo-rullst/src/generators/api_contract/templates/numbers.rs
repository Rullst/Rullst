// Preserve JSON integer semantics without first rounding fractional wire values
// through binary floating point. Input syntax was already checked by Security.
fn canonical_numbers(bytes: &[u8]) -> Result<Vec<u8>, ContractError> {
    let mut output = Vec::with_capacity(bytes.len());
    let mut offset = 0;
    while offset < bytes.len() {
        let byte = bytes[offset];
        if byte == b'"' {
            output.push(byte); offset += 1;
            while offset < bytes.len() {
                let byte = bytes[offset]; output.push(byte); offset += 1;
                if byte == b'\\' {
                    if let Some(byte) = bytes.get(offset) { output.push(*byte); offset += 1; }
                } else if byte == b'"' { break; }
            }
        } else if byte == b'-' || byte.is_ascii_digit() {
            let start = offset;
            while bytes.get(offset).is_some_and(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-')) { offset += 1; }
            let token = std::str::from_utf8(&bytes[start..offset]).map_err(|_| ContractError::Payload)?;
            output.extend_from_slice(exact_integer(token)?.to_string().as_bytes());
        } else { output.push(byte); offset += 1; }
        if output.len() > MAX_WIRE_BYTES * 4 { return Err(ContractError::Payload); }
    }
    Ok(output)
}
fn exact_integer(token: &str) -> Result<i64, ContractError> {
    if token.len() > 128 { return Err(ContractError::Payload); }
    let (mantissa, exponent) = match token.split_once(['e', 'E']) {
        Some((m,e)) => (m,e.parse::<i32>().map_err(|_| ContractError::Payload)?),
        None => (token,0),
    };
    if !(-128..=128).contains(&exponent) { return Err(ContractError::Payload); }
    let negative = mantissa.starts_with('-');
    let mantissa = mantissa.strip_prefix('-').unwrap_or(mantissa);
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa,""));
    let joined = format!("{whole}{fraction}");
    let mut digits = joined.trim_start_matches('0').to_owned();
    if digits.is_empty() { return Ok(0); }
    let scale = exponent - fraction.len() as i32;
    if scale >= 0 {
        if digits.len() + scale as usize > 16 { return Err(ContractError::Payload); }
        digits.extend(std::iter::repeat_n('0', scale as usize));
    } else {
        let remove = scale.unsigned_abs() as usize;
        let keep = digits.len().checked_sub(remove).ok_or(ContractError::Payload)?;
        if !digits[keep..].bytes().all(|b| b == b'0') { return Err(ContractError::Payload); }
        digits.truncate(keep);
    }
    let value = digits.parse::<i64>().map_err(|_| ContractError::Payload)?;
    if value > 9_007_199_254_740_991 { return Err(ContractError::Payload); }
    Ok(if negative { -value } else { value })
}
