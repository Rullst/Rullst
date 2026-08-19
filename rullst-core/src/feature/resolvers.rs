use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

// ─── Deterministic Hashing & Resolvers ──────────────────────────────────────────

/// Deterministically calculates a bucket index from 0 to 99 for a given flag and identifier.
/// This ensures a stable user-to-flag assignment without persistent storage.
#[cfg_attr(mutants, mutants::skip)]
pub fn calculate_hash_bucket(flag: &str, identifier: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    hasher.write(flag.as_bytes());
    hasher.write(identifier.as_bytes());
    let hash_val = hasher.finish();
    (hash_val % 100) as u32
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
pub fn resolve_variant(variants: &[(String, u32)], bucket: u32) -> Option<String> {
    let mut accumulator = 0;
    for (name, pct) in variants {
        accumulator += pct;
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
