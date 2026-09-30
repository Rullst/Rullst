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
            let fields = self.__rullst_redis_hash_fields()?;
            let mut conn = rullst_orm::Orm::redis_manager()?;

            // Assuming primary key is 'id' and can be formatted
            // Note: In real scenarios, primary key could be different, but we assume id for now
            let redis_key = format!("orm:{}:{}", #table_name, self.id);

            let _: () = conn.hset_multiple(&redis_key, &fields).await?;
            Ok(())
        }

        #redis_cfg
        pub async fn get_from_redis(id: impl std::fmt::Display) -> Result<Option<Self>, rullst_orm::Error>
        #redis_get_default_bound
        {
            use rullst_orm::_redis::AsyncCommands;
            let mut conn = rullst_orm::Orm::redis_manager()?;

            let redis_key = format!("orm:{}:{}", #table_name, id);

            let hash: std::collections::HashMap<String, String> = conn.hgetall(&redis_key).await?;

            if hash.is_empty() {
                return Ok(None);
            }

            Self::__rullst_from_redis_hash(&hash).map(Some)
        }

        #redis_cfg
        pub async fn increment_redis_field(id: impl std::fmt::Display, field: #column_enum_name, amount: i64) -> Result<i64, rullst_orm::Error> {
            let mut conn = rullst_orm::Orm::redis_manager()?;
            let redis_key = format!("orm:{}:{}", #table_name, id);

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
