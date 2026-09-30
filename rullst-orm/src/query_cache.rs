use sha2::{Digest, Sha256};

use crate::{Error, Orm, RullstValue};

const KEY_PREFIX: &str = "rullst:orm:cache:v3:";
const HASH_KEY_PREFIX: &str = "rullst:orm:hash:v1:";
const MAX_NAMESPACE_LEN: usize = 64;
const MAX_INVALIDATION_KEYS: usize = 10_000;
/// Suffix of the per-table set that indexes the table's cached entries. It is
/// not hexadecimal, so it can never equal an entry's digest segment.
const INDEX_SUFFIX: &str = "keys";

/// Stores an entry, records its key in the table index and extends the
/// index's lifetime to the longest entry TTL, as one atomic step.
const STORE_SCRIPT: &str = r"
redis.call('SET', KEYS[1], ARGV[1], 'EX', ARGV[2])
redis.call('SADD', KEYS[2], KEYS[1])
if redis.call('TTL', KEYS[2]) < tonumber(ARGV[2]) then
  redis.call('EXPIRE', KEYS[2], ARGV[2])
end
return 1
";

/// Removes one batch of indexed entries together with their index members.
/// Keys added concurrently either leave in this batch or stay indexed.
const INVALIDATE_SCRIPT: &str = r"
local keys = redis.call('SPOP', KEYS[1], ARGV[1])
if #keys > 0 then
  redis.call('UNLINK', unpack(keys))
end
return #keys
";
const INVALIDATION_BATCH: usize = 500;

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
        "{}:{}",
        table_prefix(namespace, tenant, table)?,
        hex_digest(&digest.finalize())
    ))
}

/// Common prefix of one namespace, tenant scope and table's cache keys.
fn table_prefix(
    namespace: &str,
    tenant: Option<&RullstValue>,
    table: &str,
) -> Result<String, Error> {
    validate_namespace(namespace)?;
    Ok(format!(
        "{KEY_PREFIX}{namespace}:{}:table-{}",
        scope_segment(tenant),
        digest_bytes(table.as_bytes())
    ))
}

/// The index of the table that a generated entry key belongs to.
fn entry_index_key(cache_key: &str) -> Result<String, Error> {
    match cache_key.rsplit_once(':') {
        Some((prefix, digest))
            if prefix.starts_with(KEY_PREFIX)
                && prefix.contains(":table-")
                && digest.len() == 64
                && digest.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
        {
            Ok(format!("{prefix}:{INDEX_SUFFIX}"))
        }
        _ => Err(Error::Validation(
            "query cache key is not a generated entry key".to_string(),
        )),
    }
}

/// Stores one generated query result and indexes its key under its table,
/// so a committed write can invalidate the table without scanning Redis.
#[doc(hidden)]
pub async fn store_entry(cache_key: &str, payload: &str, ttl_seconds: u64) -> Result<(), Error> {
    if ttl_seconds == 0 {
        return Err(Error::Validation(
            "query cache TTL must be greater than zero".to_string(),
        ));
    }
    let index = entry_index_key(cache_key)?;
    let mut connection = Orm::redis_manager()?;
    let _: i64 = crate::_redis::cmd("EVAL")
        .arg(STORE_SCRIPT)
        .arg(2)
        .arg(cache_key)
        .arg(&index)
        .arg(payload)
        .arg(ttl_seconds)
        .query_async(&mut connection)
        .await?;
    Ok(())
}

/// Deletes every indexed query-cache entry for a table.
///
/// An unconfigured Redis adapter is a no-op because model writes do not require
/// caching. Once Redis is configured, transport errors fail visibly. The work
/// follows the table's own index, never the rest of the Redis keyspace, and is
/// capped at `MAX_INVALIDATION_KEYS` entries per call; entries beyond the cap
/// stay indexed for the next write and otherwise expire through their TTL, as
/// do entries written by earlier versions that were never indexed.
pub async fn invalidate_table(table: &str) -> Result<usize, Error> {
    use crate::_redis::AsyncCommands;

    let Ok(namespace) = Orm::redis_cache_namespace() else {
        return Ok(0);
    };
    let tenant = crate::tenant::get_tenant_id();
    let index = index_key(namespace, tenant.as_ref(), table)?;
    let mut connection = Orm::redis_manager()?;
    let mut deleted = 0_usize;
    while deleted < MAX_INVALIDATION_KEYS {
        let removed: usize = crate::_redis::cmd("EVAL")
            .arg(INVALIDATE_SCRIPT)
            .arg(1)
            .arg(&index)
            .arg(INVALIDATION_BATCH)
            .query_async(&mut connection)
            .await?;
        deleted += removed;
        if removed < INVALIDATION_BATCH {
            return Ok(deleted);
        }
    }
    let remaining: usize = connection.scard(&index).await?;
    if remaining > 0 {
        return Err(Error::CacheError(format!(
            "table cache invalidation exceeded {MAX_INVALIDATION_KEYS} keys; the rest expire by TTL"
        )));
    }
    Ok(deleted)
}

/// Builds the key of a generated `save_to_redis` model hash.
///
/// The key binds the configured application namespace, an opaque digest of
/// the model's tenant (`None` for global models) and of the table, and the
/// record ID, so applications and tenants sharing a Redis database never
/// read or overwrite each other's hashes. IDs are 1-64 ASCII letters, digits,
/// '-' or '_'.
#[doc(hidden)]
pub fn model_hash_key(
    table: &str,
    tenant: Option<&RullstValue>,
    id: &str,
) -> Result<String, Error> {
    build_model_hash_key(Orm::redis_cache_namespace()?, tenant, table, id)
}

fn build_model_hash_key(
    namespace: &str,
    tenant: Option<&RullstValue>,
    table: &str,
    id: &str,
) -> Result<String, Error> {
    validate_namespace(namespace)?;
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(Error::Validation(
            "Redis model hash ID must contain 1-64 ASCII letters, digits, '-' or '_'".to_string(),
        ));
    }
    Ok(format!(
        "{HASH_KEY_PREFIX}{namespace}:{}:table-{}:{id}",
        scope_segment(tenant),
        digest_bytes(table.as_bytes())
    ))
}

/// The set indexing one namespace, tenant scope and table's cache entries.
fn index_key(namespace: &str, tenant: Option<&RullstValue>, table: &str) -> Result<String, Error> {
    Ok(format!(
        "{}:{INDEX_SUFFIX}",
        table_prefix(namespace, tenant, table)?
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
    use super::{build_key, build_model_hash_key, entry_index_key, index_key};
    use crate::RullstValue;

    #[test]
    fn model_hash_keys_bind_namespace_tenant_and_table() {
        let tenant = RullstValue::String("private-school-name".to_string());
        let key = build_model_hash_key("academy", Some(&tenant), "invoices", "42")
            .expect("tenant hash key");
        assert!(key.starts_with("rullst:orm:hash:v1:academy:tenant-"));
        assert!(key.ends_with(":42"));
        assert!(!key.contains("private-school-name"));
        assert!(!key.contains("invoices"));
        for other in [
            build_model_hash_key("billing", Some(&tenant), "invoices", "42"),
            build_model_hash_key(
                "academy",
                Some(&RullstValue::String("other".to_string())),
                "invoices",
                "42",
            ),
            build_model_hash_key("academy", None, "invoices", "42"),
            build_model_hash_key("academy", Some(&tenant), "orders", "42"),
        ] {
            assert_ne!(key, other.expect("comparison key"));
        }
        for invalid in ["", "4:2", "4 2", &"9".repeat(65)] {
            assert!(build_model_hash_key("academy", None, "invoices", invalid).is_err());
        }
        assert!(build_model_hash_key("bad namespace", None, "invoices", "1").is_err());
    }

    #[test]
    fn entry_keys_resolve_to_their_table_index() {
        let tenant = RullstValue::Int(7);
        let entry =
            build_key("academy", Some(&tenant), "users", "SELECT *", &[]).expect("entry key");
        let index = index_key("academy", Some(&tenant), "users").expect("index key");
        assert_eq!(entry_index_key(&entry).expect("entry index"), index);
        assert_ne!(index, entry);
        for foreign in [
            "",
            "session:abc",
            "rullst:orm:cache:v3:academy:global:table-x:not-a-digest",
            index.as_str(),
        ] {
            assert!(entry_index_key(foreign).is_err(), "{foreign}");
        }
    }

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
        let tenant_index =
            index_key("academy", Some(&tenant), "users").expect("tenant invalidation index");
        let global_index = index_key("academy", None, "users").expect("global invalidation index");

        assert_ne!(tenant_index, global_index);
        assert!(tenant_index.contains(":tenant-"));
        assert!(global_index.contains(":global:"));
        assert!(!tenant_index.contains("private-school-name"));
        assert!(
            !tenant_index.contains('*'),
            "invalidation never uses a key pattern"
        );
    }
}
