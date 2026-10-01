// ─── Deterministic Hashing & Resolvers ──────────────────────────────────────────

/// Domain separator of the version 1 bucket hash.
const BUCKET_DOMAIN_V1: &[u8] = b"rullst.feature-bucket.v1\0";

/// Deterministically calculates a bucket index from 0 to 99 for a given flag and identifier.
/// This ensures a stable user-to-flag assignment without persistent storage.
///
/// The bucket is the first eight bytes of
/// `SHA-256("rullst.feature-bucket.v1\0" || u64_le(len(flag)) || flag || identifier)`,
/// read big-endian, modulo 100. It is identical across Rust toolchains,
/// platforms and replicas. Releases before this hash used `std`'s unspecified
/// `DefaultHasher` without a length prefix, so upgrading reassigns buckets once.
#[cfg_attr(mutants, mutants::skip)]
pub fn calculate_hash_bucket(flag: &str, identifier: &str) -> u32 {
    let hash = bucket_hash(flag, identifier);
    // `hash % 100` is below 100, so the narrowing is lossless.
    (hash % 100) as u32
}

fn bucket_hash(flag: &str, identifier: &str) -> u64 {
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    context.update(BUCKET_DOMAIN_V1);
    context.update(&(flag.len() as u64).to_le_bytes());
    context.update(flag.as_bytes());
    context.update(identifier.as_bytes());
    let digest = context.finish();
    let mut prefix = [0_u8; 8];
    prefix.copy_from_slice(&digest.as_ref()[..8]);
    u64::from_be_bytes(prefix)
}

/// Parses a rollout percentage string (e.g. "30%") into a number.
pub fn parse_rollout(s: &str) -> Option<u32> {
    let cleaned = s.trim().trim_end_matches('%');
    cleaned.parse::<u32>().ok()
}

/// Parses an A/B split configuration string (e.g. "variant-a:50,variant-b:50")
/// into a vector of variant names and their percentage weights.
pub fn parse_variants(s: &str) -> Vec<(String, u32)> {
    let mut parsed = Vec::new();
    for part in s.split(',') {
        let mut split = part.split(':');
        if let (Some(name), Some(pct_str)) = (split.next(), split.next())
            && let Ok(pct) = pct_str.trim().parse::<u32>()
        {
            parsed.push((name.trim().to_string(), pct));
        }
    }
    parsed
}

/// Evaluates a hash bucket index against a list of variants and returns the matching name.
///
/// Weights accumulate with saturation, so oversized weights (for example a
/// mistyped database row) cannot overflow: a weight that reaches `u32::MAX`
/// covers every remaining bucket.
pub fn resolve_variant(variants: &[(String, u32)], bucket: u32) -> Option<String> {
    let mut accumulator = 0_u32;
    for (name, pct) in variants {
        accumulator = accumulator.saturating_add(*pct);
        if bucket < accumulator {
            return Some(name.clone());
        }
    }
    None
}

/// Helper function to parse feature toggles string formats uniformly
#[cfg_attr(mutants, mutants::skip)]
pub(crate) fn parse_feature_string_value(
    value: &str,
    flag: &str,
    identifier: Option<&str>,
) -> Option<String> {
    let cleaned = value.trim();
    if cleaned.is_empty() {
        return None;
    }

    // 1. Check if simple boolean
    #[cfg_attr(mutants, mutants::skip)]
    if cleaned == "true" || cleaned == "1" || cleaned == "yes" {
        return Some("enabled".to_string());
    }
    if cleaned == "false" || cleaned == "0" || cleaned == "no" {
        return Some("disabled".to_string());
    }

    // 2. Check if percentage rollout (e.g., "30%")
    if cleaned.ends_with('%')
        && let Some(pct) = parse_rollout(cleaned)
    {
        if let Some(ident) = identifier {
            let bucket = calculate_hash_bucket(flag, ident);
            return Some(if bucket < pct {
                "enabled".to_string()
            } else {
                "disabled".to_string()
            });
        }
        return Some("disabled".to_string());
    }

    // 3. Check if A/B splits (e.g., "variant-a:50,variant-b:50")
    if cleaned.contains(':') {
        let variants = parse_variants(cleaned);
        if !variants.is_empty()
            && let Some(ident) = identifier
        {
            let bucket = calculate_hash_bucket(flag, ident);
            return resolve_variant(&variants, bucket);
        }
    }

    Some(cleaned.to_string())
}
