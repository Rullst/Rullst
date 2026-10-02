//! Storage of the generated `save_to_redis`/`get_from_redis`/
//! `increment_redis_field` model hashes.
//!
//! 12.2 keys each hash by the application namespace, an opaque tenant digest
//! (or the global scope) and a table digest. Releases up to 12.1 used the
//! shared key `orm:<table>:<id>`. For a model without a tenant scope that
//! legacy key held the same data, so it is still read when the namespaced key
//! is missing and is moved to the namespaced key by the next write. A tenant
//! model never reads or moves it: the legacy key was shared by every tenant.

use std::collections::HashMap;

use super::{digest_bytes, scope_segment, validate_namespace};
use crate::{Error, Orm, RullstValue};

const HASH_KEY_PREFIX: &str = "rullst:orm:hash:v1:";
const LEGACY_HASH_KEY_PREFIX: &str = "orm:";

/// Moves a legacy hash (`KEYS[2]`) to the namespaced key (`KEYS[1]`) unless
/// the namespaced key already exists, in which case the stale legacy hash is
/// removed. `RENAME` keeps every legacy field, including ones the next write
/// does not set.
const MIGRATE_PRELUDE: &str = r"
if redis.call('EXISTS', KEYS[1]) == 0 then
  if redis.call('EXISTS', KEYS[2]) == 1 then
    redis.call('RENAME', KEYS[2], KEYS[1])
  end
else
  redis.call('DEL', KEYS[2])
end
";
/// Writes field/value pairs (`ARGV`) to the namespaced key.
const WRITE_BODY: &str = r"
redis.call('HSET', KEYS[1], unpack(ARGV))
return 1
";
/// Increments field `ARGV[1]` of the namespaced key by `ARGV[2]`.
const INCREMENT_BODY: &str = r"
return redis.call('HINCRBY', KEYS[1], ARGV[1], ARGV[2])
";

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

/// The 12.1 key of a global model's hash; tenant models have none, because
/// that key was shared by every tenant.
fn legacy_model_hash_key(tenant: Option<&RullstValue>, table: &str, id: &str) -> Option<String> {
    tenant
        .is_none()
        .then(|| format!("{LEGACY_HASH_KEY_PREFIX}{table}:{id}"))
}

/// Reads a generated model hash. A global model whose namespaced hash is
/// missing falls back to its 12.1 key; the hash is empty when neither exists.
#[doc(hidden)]
pub async fn read_model_hash(
    table: &str,
    tenant: Option<&RullstValue>,
    id: &str,
) -> Result<HashMap<String, String>, Error> {
    use crate::_redis::AsyncCommands;

    let key = model_hash_key(table, tenant, id)?;
    let mut connection = Orm::redis_manager()?;
    let hash: HashMap<String, String> = connection.hgetall(&key).await?;
    match legacy_model_hash_key(tenant, table, id) {
        Some(legacy) if hash.is_empty() => Ok(connection.hgetall(&legacy).await?),
        _ => Ok(hash),
    }
}

/// Writes a generated model hash, first moving a global model's 12.1 hash to
/// the namespaced key so the data migrates on its next write.
#[doc(hidden)]
pub async fn write_model_hash(
    table: &str,
    tenant: Option<&RullstValue>,
    id: &str,
    fields: &[(&str, String)],
) -> Result<(), Error> {
    use crate::_redis::AsyncCommands;

    let key = model_hash_key(table, tenant, id)?;
    let mut connection = Orm::redis_manager()?;
    let Some(legacy) = legacy_model_hash_key(tenant, table, id) else {
        let _: () = connection.hset_multiple(&key, fields).await?;
        return Ok(());
    };
    let mut script = crate::_redis::cmd("EVAL");
    script
        .arg(format!("{MIGRATE_PRELUDE}{WRITE_BODY}"))
        .arg(2)
        .arg(&key)
        .arg(&legacy);
    for (field, value) in fields {
        script.arg(*field).arg(value);
    }
    let _: i64 = script.query_async(&mut connection).await?;
    Ok(())
}

/// Increments one field of a generated model hash, first moving a global
/// model's 12.1 hash to the namespaced key.
#[doc(hidden)]
pub async fn increment_model_hash(
    table: &str,
    tenant: Option<&RullstValue>,
    id: &str,
    field: &str,
    amount: i64,
) -> Result<i64, Error> {
    let key = model_hash_key(table, tenant, id)?;
    let mut connection = Orm::redis_manager()?;
    let incremented = match legacy_model_hash_key(tenant, table, id) {
        None => {
            crate::_redis::cmd("HINCRBY")
                .arg(&key)
                .arg(field)
                .arg(amount)
                .query_async(&mut connection)
                .await?
        }
        Some(legacy) => {
            crate::_redis::cmd("EVAL")
                .arg(format!("{MIGRATE_PRELUDE}{INCREMENT_BODY}"))
                .arg(2)
                .arg(&key)
                .arg(&legacy)
                .arg(field)
                .arg(amount)
                .query_async(&mut connection)
                .await?
        }
    };
    Ok(incremented)
}

#[cfg(test)]
mod tests {
    use super::{build_model_hash_key, legacy_model_hash_key};
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
    fn only_global_models_have_a_legacy_hash_key() {
        assert_eq!(
            legacy_model_hash_key(None, "invoices", "42").as_deref(),
            Some("orm:invoices:42")
        );
        let tenant = RullstValue::String("acme".to_string());
        assert_eq!(legacy_model_hash_key(Some(&tenant), "invoices", "42"), None);
    }
}
