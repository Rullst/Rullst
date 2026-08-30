use sha2::{Digest, Sha256};

use crate::{Error, Orm, RullstValue};

const CACHE_KEY_VERSION: &str = "v3";
const MAX_NAMESPACE_LEN: usize = 64;
const MAX_INVALIDATION_KEYS: usize = 10_000;

pub(crate) fn validate_namespace(namespace: &str) -> Result<(), Error> {
    if namespace.is_empty()
        || namespace.len() > MAX_NAMESPACE_LEN
        || !namespace
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(Error::Validation(
            "Redis cache namespace must contain 1-64 ASCII letters, digits, '.', '-' or '_'"
                .to_string(),
        ));
    }
    Ok(())
}

/// Builds the versioned key used by generated `.remember(...)` queries.
pub fn query_key(table: &str, query: &str, bindings: &[RullstValue]) -> Result<String, Error> {
    let namespace = Orm::redis_cache_namespace()?;
    let tenant = crate::tenant::get_tenant_id();
    build_key(namespace, tenant.as_ref(), table, query, bindings)
}

fn build_key(
    namespace: &str,
    tenant: Option<&RullstValue>,
    table: &str,
    query: &str,
    bindings: &[RullstValue],
) -> Result<String, Error> {
    validate_namespace(namespace)?;

    let scope = scope_segment(tenant);
    let mut digest = Sha256::new();
    update_field(&mut digest, 1, namespace.as_bytes());
    if let Some(value) = tenant {
        update_value(&mut digest, value);
    } else {
        update_field(&mut digest, 2, b"global");
    }
    update_field(&mut digest, 3, table.as_bytes());
    update_field(&mut digest, 4, query.as_bytes());
    for binding in bindings {
        update_value(&mut digest, binding);
    }

    Ok(format!(
        "rullst:orm:cache:{CACHE_KEY_VERSION}:{namespace}:{scope}:table-{}:{}",
        digest_bytes(table.as_bytes()),
        hex_digest(&digest.finalize())
    ))
}

/// Deletes every generated query-cache entry for a table.
///
/// An unconfigured Redis adapter is a no-op because model writes do not require
/// caching. Once Redis is configured, transport errors fail visibly. The scan
/// is bounded to prevent an accidental unbounded post-commit operation.
pub async fn invalidate_table(table: &str) -> Result<usize, Error> {
    use crate::_redis::AsyncCommands;

    let Ok(namespace) = Orm::redis_cache_namespace() else {
        return Ok(0);
    };
    let tenant = crate::tenant::get_tenant_id();
    let pattern = invalidation_pattern(namespace, tenant.as_ref(), table)?;
    let mut scanner = Orm::redis_manager()?;
    let mut deleter = scanner.clone();
    let mut keys = scanner.scan_match::<_, String>(pattern).await?;
    let mut deleted = 0_usize;

    while let Some(key) = keys.next_item().await {
        if deleted >= MAX_INVALIDATION_KEYS {
            return Err(Error::CacheError(format!(
                "table cache invalidation exceeded {MAX_INVALIDATION_KEYS} keys"
            )));
        }
        let _: usize = deleter.unlink(key?).await?;
        deleted += 1;
    }

    Ok(deleted)
}

fn invalidation_pattern(
    namespace: &str,
    tenant: Option<&RullstValue>,
    table: &str,
) -> Result<String, Error> {
    validate_namespace(namespace)?;
    Ok(format!(
        "rullst:orm:cache:{CACHE_KEY_VERSION}:{namespace}:{}:table-{}:*",
        scope_segment(tenant),
        digest_bytes(table.as_bytes())
    ))
}

fn scope_segment(tenant: Option<&RullstValue>) -> String {
    match tenant {
        Some(value) => format!("tenant-{}", short_digest(value)),
        None => "global".to_string(),
    }
}

fn short_digest(value: &RullstValue) -> String {
    let mut digest = Sha256::new();
    digest.update(b"rullst:orm:tenant:v1");
    update_value(&mut digest, value);
    let output = digest.finalize();
    hex_digest(&output[..16])
}

fn digest_bytes(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"rullst:orm:cache-table:v1");
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
    hex_digest(&digest.finalize()[..16])
}

fn update_value(digest: &mut Sha256, value: &RullstValue) {
    match value {
        RullstValue::String(value) => update_field(digest, 10, value.as_bytes()),
        RullstValue::Int(value) => update_field(digest, 11, &value.to_be_bytes()),
        RullstValue::Float(value) => update_field(digest, 12, &value.to_bits().to_be_bytes()),
        RullstValue::Bool(value) => update_field(digest, 13, &[*value as u8]),
    }
}

fn update_field(digest: &mut Sha256, tag: u8, bytes: &[u8]) {
    digest.update([tag]);
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{build_key, invalidation_pattern};
    use crate::RullstValue;

    #[test]
    fn keys_are_versioned_deterministic_and_domain_separated() {
        let bindings = vec![RullstValue::String("12".to_string())];
        let first =
            build_key("academy", None, "users", "SELECT *", &bindings).expect("build cache key");
        let repeated =
            build_key("academy", None, "users", "SELECT *", &bindings).expect("repeat cache key");
        let typed = build_key(
            "academy",
            None,
            "users",
            "SELECT *",
            &[RullstValue::Int(12)],
        )
        .expect("typed cache key");

        assert_eq!(first, repeated);
        assert!(first.starts_with("rullst:orm:cache:v3:academy:global:table-"));
        assert_ne!(first, typed);
    }

    #[test]
    fn table_segment_is_explicit_but_does_not_expose_the_table_name() {
        let users =
            build_key("academy", None, "private_users", "SELECT *", &[]).expect("users cache key");
        let posts =
            build_key("academy", None, "private_posts", "SELECT *", &[]).expect("posts cache key");

        assert_ne!(users, posts);
        assert!(!users.contains("private_users"));
        assert!(!posts.contains("private_posts"));
    }

    #[test]
    fn tenant_and_application_namespaces_cannot_share_keys_or_leak_ids() {
        let tenant = RullstValue::String("private-school-name".to_string());
        let first = build_key("academy", Some(&tenant), "users", "SELECT *", &[])
            .expect("tenant cache key");
        let other_tenant = build_key(
            "academy",
            Some(&RullstValue::String("other".to_string())),
            "users",
            "SELECT *",
            &[],
        )
        .expect("other tenant cache key");
        let other_application = build_key("billing", Some(&tenant), "users", "SELECT *", &[])
            .expect("other application cache key");

        assert_ne!(first, other_tenant);
        assert_ne!(first, other_application);
        assert!(!first.contains("private-school-name"));
    }

    #[test]
    fn invalid_namespaces_fail_closed() {
        for invalid in ["", "has spaces", "tenant:escape", "../escape"] {
            assert!(build_key(invalid, None, "users", "SELECT *", &[]).is_err());
        }
        assert!(build_key(&"a".repeat(65), None, "users", "SELECT *", &[]).is_err());
    }

    #[test]
    fn invalidation_is_limited_to_the_active_opaque_tenant_scope() {
        let tenant = RullstValue::String("private-school-name".to_string());
        let tenant_pattern = invalidation_pattern("academy", Some(&tenant), "users")
            .expect("tenant invalidation pattern");
        let global_pattern =
            invalidation_pattern("academy", None, "users").expect("global invalidation pattern");

        assert_ne!(tenant_pattern, global_pattern);
        assert!(tenant_pattern.contains(":tenant-"));
        assert!(global_pattern.contains(":global:"));
        assert!(!tenant_pattern.contains("private-school-name"));
    }
}
