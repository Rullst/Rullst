// src/builder/query_cache.rs — Generated Redis cache-aside fragments.

use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_cache_read(
    name: &syn::Ident,
    table_name: &str,
    decrypt_results: &TokenStream,
    hook_after_fetch: &TokenStream,
    eager_loads: &TokenStream,
) -> TokenStream {
    quote! {
        #[cfg(feature = "redis")]
        let cache_key = if _allow_cache && self.remember_ttl.is_some() {
            Some(rullst_orm::query_cache::query_key(
                #table_name,
                &query_str,
                &self.bindings,
            )?)
        } else {
            None
        };

        #[cfg(feature = "redis")]
        if let Some(cache_key) = cache_key.as_ref() {
            use rullst_orm::_redis::AsyncCommands;
            let mut conn = rullst_orm::Orm::redis_manager()?;
            if let Ok(cached_data) = conn.get::<_, String>(cache_key).await {
                if !cached_data.is_empty() {
                    if let Ok(mut results) = #name::from_cache_json_array(&cached_data) {
                        #decrypt_results
                        #hook_after_fetch
                        #eager_loads
                        return Ok(results);
                    }
                }
            }
        }
    }
}

pub fn generate_cache_write(name: &syn::Ident) -> TokenStream {
    quote! {
        #[cfg(feature = "redis")]
        if let (Some(ttl), Some(cache_key)) = (self.remember_ttl, cache_key.as_ref()) {
            use rullst_orm::_redis::AsyncCommands;
            let ttl = u64::try_from(ttl).map_err(|_| rullst_orm::Error::Validation(
                "remember() TTL exceeds the Redis-supported range".to_string()
            ))?;
            let serialized = #name::to_cache_json_array(&results);
            let mut conn = rullst_orm::Orm::redis_manager()?;
            let _: Result<(), rullst_orm::_redis::RedisError> = conn.set_ex(cache_key, serialized, ttl).await;
        }
    }
}
