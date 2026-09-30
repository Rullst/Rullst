// src/builder/query_cache.rs — Generated Redis cache-aside fragments.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_cache_read(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let decrypt_model = if parsed.encrypted_fields.is_empty() {
        quote! {}
    } else {
        quote! { model.__rullst_decrypt_encrypted_fields()?; }
    };
    let redis_cfg = crate::feature_gates::redis();
    quote! {
        #redis_cfg
        let cache_key = if _allow_cache && self.remember_ttl.is_some() {
            Some(rullst_orm::query_cache::query_key(
                #table_name,
                &query_str,
                &query_bindings,
            )?)
        } else {
            None
        };

        #redis_cfg
        if let Some(cache_key) = cache_key.as_ref() {
            use rullst_orm::_redis::AsyncCommands;
            let mut conn = rullst_orm::Orm::redis_manager()?;
            if let Ok(cached_data) = conn.get::<_, String>(cache_key).await {
                // Only a JSON array of rows that all decode and decrypt is a
                // hit. Anything else (for example `null`, or ciphertext under
                // a retired key) is a miss that the database read overwrites.
                if let Ok(rullst_orm::_serde_json::Value::Array(items)) =
                    rullst_orm::_serde_json::from_str::<rullst_orm::_serde_json::Value>(&cached_data)
                {
                    let decoded: Result<Vec<#name>, rullst_orm::Error> = items
                        .into_iter()
                        .map(|item| {
                            let mut model = #name::from_json_value(item)?;
                            #decrypt_model
                            Ok(model)
                        })
                        .collect();
                    if let Ok(results) = decoded {
                        return Ok(results);
                    }
                }
            }
        }
    }
}

pub fn generate_cache_write(name: &syn::Ident) -> TokenStream {
    let redis_cfg = crate::feature_gates::redis();
    quote! {
        #redis_cfg
        if let (Some(ttl), Some(cache_key)) = (self.remember_ttl, cache_key.as_ref()) {
            let ttl = u64::try_from(ttl).map_err(|_| rullst_orm::Error::Validation(
                "remember() TTL exceeds the Redis-supported range".to_string()
            ))?;
            // A model that cannot be serialized safely (for example a
            // `SecretString` without a configured key) is not cached. The
            // entry is indexed under its table for commit-time invalidation.
            if let Ok(serialized) = #name::__rullst_try_cache_json_array(&results) {
                let _: Result<(), rullst_orm::Error> =
                    rullst_orm::query_cache::store_entry(cache_key, &serialized, ttl).await;
            }
        }
    }
}
