//! ISO 4217 minor-unit exponents shared by exact money conversions.

/// Returns the ISO 4217 minor-unit exponent of a three-letter currency code.
///
/// The comparison ignores ASCII case. Codes that are not listed use two
/// decimals, the ISO 4217 exponent of most circulating currencies.
pub(crate) fn minor_unit_exponent(currency: &str) -> u32 {
    match currency.to_ascii_uppercase().as_str() {
        "BIF" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW" | "PYG" | "RWF" | "UGX"
        | "UYI" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
        "CLF" | "UYW" => 4,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::minor_unit_exponent;

    #[test]
    fn exponents_follow_iso_4217_and_ignore_case() {
        assert_eq!(minor_unit_exponent("JPY"), 0);
        assert_eq!(minor_unit_exponent("jpy"), 0);
        assert_eq!(minor_unit_exponent("KWD"), 3);
        assert_eq!(minor_unit_exponent("CLF"), 4);
        assert_eq!(minor_unit_exponent("BRL"), 2);
        assert_eq!(minor_unit_exponent("usd"), 2);
    }
}
