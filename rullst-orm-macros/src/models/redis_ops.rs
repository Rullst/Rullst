use crate::parser::{EncryptedFieldKind, ParsedModel};
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_redis_hash_methods(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let normal_fields = &parsed.normal_fields;
    let skipped_fields = &parsed.skipped_fields;
    let column_enum_name = quote::format_ident!("{}Column", name);
    let redis_cfg = crate::feature_gates::redis();
    let RedisTenant {
        context: tenant_context,
        key_arg: tenant_arg,
        instance_check: instance_tenant_check,
        decoded_check: decoded_tenant_check,
        field_guard: tenant_field_guard,
    } = redis_tenant(parsed);

    let mut relation_field_idents = vec![];
    for rel in &parsed.relations {
        relation_field_idents.push(rel.field_name.clone());
    }

    let skip_tail = if skipped_fields.is_empty() {
        quote! {}
    } else {
        quote! { ..Default::default() }
    };
    let redis_get_default_bound = if skipped_fields.is_empty() {
        quote! {}
    } else {
        quote! { where Self: Default }
    };

    let mut to_hash_fields = vec![];
    let mut from_hash_fields = vec![];

    for field in normal_fields {
        let field_str = field.to_string();
        let encrypted_kind = parsed
            .encrypted_fields
            .iter()
            .find(|encrypted| encrypted.name == *field)
            .map(|encrypted| encrypted.kind);

        // Redis hashes store each field as JSON. Serialization failures are
        // returned to the caller instead of silently replacing data with null.
        // `#[orm(encrypted)]` fields keep the same authenticated envelope as
        // the SQL column, so Redis never receives their plaintext.
        let stored_value = match encrypted_kind {
            Some(EncryptedFieldKind::String) => quote! {
                rullst_orm::privacy::encrypt_model_field(&self.#field, #table_name, #field_str)?
            },
            Some(EncryptedFieldKind::OptionalString) => quote! {
                match self.#field.as_deref() {
                    Some(value) => Some(rullst_orm::privacy::encrypt_model_field(
                        value,
                        #table_name,
                        #field_str,
                    )?),
                    None => None,
                }
            },
            None => quote! { &self.#field },
        };
        to_hash_fields.push(quote! {
            (#field_str, rullst_orm::_serde_json::to_string(&#stored_value)?)
        });

        let decoded_value = match encrypted_kind {
            Some(EncryptedFieldKind::String) => quote! {
                let stored: String = rullst_orm::_serde_json::from_str(serialized)?;
                rullst_orm::privacy::decrypt_model_field(&stored, #table_name, #field_str)?
            },
            Some(EncryptedFieldKind::OptionalString) => quote! {
                let stored: Option<String> = rullst_orm::_serde_json::from_str(serialized)?;
                match stored {
                    Some(value) => Some(rullst_orm::privacy::decrypt_model_field(
                        &value,
                        #table_name,
                        #field_str,
                    )?),
                    None => None,
                }
            },
            None => quote! { rullst_orm::_serde_json::from_str(serialized)? },
        };
        // A missing or malformed cache field means the cached model is corrupt;
        // do not invent a Default value that the model never promised to have.
        from_hash_fields.push(quote! {
            #field: {
                let serialized = hash.get(#field_str).ok_or_else(|| {
                    rullst_orm::Error::CacheError(format!(
                        "Redis hash for {} is missing field {}",
                        #table_name,
                        #field_str,
                    ))
                })?;
                #decoded_value
            }
        });
    }

    quote! {
        /// Serializes each persisted field for a Redis hash.
        #redis_cfg
        fn __rullst_redis_hash_fields(&self) -> Result<Vec<(&'static str, String)>, rullst_orm::Error> {
            Ok(vec![
                #(#to_hash_fields),*
            ])
        }

        /// Rebuilds a model from a Redis hash written by `save_to_redis`.
        #redis_cfg
        fn __rullst_from_redis_hash(
            hash: &std::collections::HashMap<String, String>,
        ) -> Result<Self, rullst_orm::Error>
        #redis_get_default_bound
        {
            Ok(Self {
                #(#from_hash_fields,)*
                #(#relation_field_idents: None,)*
                #skip_tail
            })
        }

        #redis_cfg
        pub async fn save_to_redis(&self) -> Result<(), rullst_orm::Error> {
            use rullst_orm::_redis::AsyncCommands;
            #tenant_context
            #instance_tenant_check
            let fields = self.__rullst_redis_hash_fields()?;
            let redis_key = rullst_orm::query_cache::model_hash_key(
                #table_name,
                #tenant_arg,
                &self.id.to_string(),
            )?;
            let mut conn = rullst_orm::Orm::redis_manager()?;
            let _: () = conn.hset_multiple(&redis_key, &fields).await?;
            Ok(())
        }

        #redis_cfg
        pub async fn get_from_redis(id: impl std::fmt::Display) -> Result<Option<Self>, rullst_orm::Error>
        #redis_get_default_bound
        {
            use rullst_orm::_redis::AsyncCommands;
            #tenant_context
            let redis_key = rullst_orm::query_cache::model_hash_key(
                #table_name,
                #tenant_arg,
                &id.to_string(),
            )?;
            let mut conn = rullst_orm::Orm::redis_manager()?;
            let hash: std::collections::HashMap<String, String> = conn.hgetall(&redis_key).await?;

            if hash.is_empty() {
                return Ok(None);
            }

            let model = Self::__rullst_from_redis_hash(&hash)?;
            #decoded_tenant_check
            Ok(Some(model))
        }

        #redis_cfg
        pub async fn increment_redis_field(id: impl std::fmt::Display, field: #column_enum_name, amount: i64) -> Result<i64, rullst_orm::Error> {
            #tenant_context
            #tenant_field_guard
            let redis_key = rullst_orm::query_cache::model_hash_key(
                #table_name,
                #tenant_arg,
                &id.to_string(),
            )?;
            let mut conn = rullst_orm::Orm::redis_manager()?;
            let new_val: i64 = rullst_orm::_redis::cmd("HINCRBY")
                .arg(&redis_key)
                .arg(field.as_str())
                .arg(amount)
                .query_async(&mut conn)
                .await?;
            Ok(new_val)
        }
    }
}

/// Tenant fragments of the Redis hash methods: the active tenant (required
/// for tenant models), the key argument and the model/field checks.
struct RedisTenant {
    context: TokenStream,
    key_arg: TokenStream,
    instance_check: TokenStream,
    decoded_check: TokenStream,
    field_guard: TokenStream,
}

fn redis_tenant(parsed: &ParsedModel) -> RedisTenant {
    let table_name = &parsed.table_name;
    let Some(tenant_type) = parsed
        .normal_fields
        .iter()
        .zip(parsed.normal_fields_types.iter())
        .find(|(field, _)| *field == parsed.tenant_column.as_str())
        .map(|(_, ty)| ty)
    else {
        return RedisTenant {
            context: quote! {},
            key_arg: quote! { None },
            instance_check: quote! {},
            decoded_check: quote! {},
            field_guard: quote! {},
        };
    };
    let column = syn::Ident::new(&parsed.tenant_column, parsed.name.span());
    let column_name = &parsed.tenant_column;
    let outside = quote! {
        return Err(rullst_orm::Error::Validation(
            "record is outside the active tenant scope".to_string()
        ));
    };
    RedisTenant {
        context: quote! {
            let tenant = rullst_orm::tenant::get_tenant_id().ok_or_else(|| {
                rullst_orm::Error::Validation(format!(
                    "tenant context is required to use the Redis hash of `{}`",
                    #table_name
                ))
            })?;
            let expected_tenant: #tenant_type = tenant.clone().try_into().map_err(|_| {
                rullst_orm::Error::Validation(format!(
                    "tenant context type does not match `{}.{}`",
                    #table_name,
                    #column_name
                ))
            })?;
        },
        key_arg: quote! { Some(&tenant) },
        instance_check: quote! {
            if self.#column != expected_tenant { #outside }
        },
        decoded_check: quote! {
            if model.#column != expected_tenant { #outside }
        },
        field_guard: quote! {
            // The key already binds the tenant; the typed conversion above
            // only makes a mistyped context fail closed.
            let _ = expected_tenant;
            if field.as_str() == #column_name {
                return Err(rullst_orm::Error::Validation(
                    "the tenant column of a Redis model hash cannot be incremented".to_string()
                ));
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{DeriveInput, parse_quote};

    #[test]
    fn redis_deserialization_propagates_corrupt_cache_errors() {
        let input: DeriveInput = parse_quote! {
            struct CachedModel {
                id: i64,
                payload: Json<Payload>,
            }
        };
        let parsed = crate::parser::parse(&input).expect("test model should parse");
        let generated = generate_redis_hash_methods(&parsed).to_string();

        assert!(!generated.contains("unwrap_or_default"));
        assert!(!generated.contains("String :: from (\"null\")"));
        assert!(generated.contains("is missing field"));
        assert!(generated.contains("from_str (serialized) ?"));
    }

    #[test]
    fn hash_keys_bind_the_namespace_and_the_active_tenant() {
        let global: DeriveInput = parse_quote! {
            #[orm(table = "docs")]
            struct Doc { id: i32, title: String }
        };
        let parsed = crate::parser::parse(&global).expect("test model should parse");
        let generated = generate_redis_hash_methods(&parsed).to_string();
        assert_eq!(
            generated.matches("query_cache :: model_hash_key").count(),
            3
        );
        assert!(!generated.contains("\"orm:{}:{}\""));
        assert!(!generated.contains("get_tenant_id"));

        let tenant: DeriveInput = parse_quote! {
            #[orm(table = "invoices", tenant_column = "organization_id")]
            struct Invoice { id: i32, organization_id: String, total: i64 }
        };
        let parsed = crate::parser::parse(&tenant).expect("test model should parse");
        let generated = generate_redis_hash_methods(&parsed).to_string();
        assert_eq!(generated.matches("Some (& tenant)").count(), 3);
        assert_eq!(
            generated
                .matches("tenant context is required to use the Redis hash")
                .count(),
            3
        );
        assert!(generated.contains("if self . organization_id != expected_tenant"));
        assert!(generated.contains("if model . organization_id != expected_tenant"));
        assert!(generated.contains("cannot be incremented"));
    }

    #[test]
    fn encrypted_fields_are_enveloped_in_redis_hashes() {
        let input: DeriveInput = parse_quote! {
            #[orm(table = "patients")]
            struct Patient {
                id: i32,
                #[orm(encrypted)]
                diagnosis: String,
                #[orm(encrypted)]
                note: Option<String>,
            }
        };
        let parsed = crate::parser::parse(&input).expect("test model should parse");
        let generated = generate_redis_hash_methods(&parsed).to_string();

        assert!(
            generated.contains(
                "encrypt_model_field (& self . diagnosis , \"patients\" , \"diagnosis\")"
            )
        );
        assert!(
            generated.contains("decrypt_model_field (& stored , \"patients\" , \"diagnosis\")")
        );
        assert!(generated.contains("decrypt_model_field (& value , \"patients\" , \"note\" ,)"));
        assert!(!generated.contains("to_string (& & self . diagnosis)"));
    }
}
