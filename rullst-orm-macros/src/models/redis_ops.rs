use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_redis_hash_methods(parsed: &ParsedModel) -> TokenStream {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    let normal_fields = &parsed.normal_fields;
    let skipped_fields = &parsed.skipped_fields;
    let column_enum_name = quote::format_ident!("{}Column", name);

    let mut relation_field_idents = vec![];
    for rel in &parsed.relations {
        relation_field_idents.push(rel.field_name.clone());
    }

    let skip_tail = if skipped_fields.is_empty() {
        quote! {}
    } else {
        quote! { ..Default::default() }
    };

    let mut to_hash_fields = vec![];
    let mut from_hash_fields = vec![];

    for field in normal_fields {
        let field_str = field.to_string();

        // When saving, convert each field to a JSON string
        to_hash_fields.push(quote! {
            (#field_str, rullst_orm::_serde_json::to_string(&self.#field).unwrap_or_else(|_| String::from("null")))
        });

        // When loading, parse each field from its JSON string representation
        // If missing, default to string "null" for parsing
        from_hash_fields.push(quote! {
            #field: rullst_orm::_serde_json::from_str(
                hash.get(#field_str).unwrap_or(&String::from("null"))
            ).unwrap_or_default()
        });
    }

    quote! {
        #[cfg(feature = "redis")]
        pub async fn save_to_redis(&self) -> Result<(), rullst_orm::Error> {
            use rullst_orm::_redis::AsyncCommands;
            let mut conn = rullst_orm::Orm::redis_manager()?;

            // Assuming primary key is 'id' and can be formatted
            // Note: In real scenarios, primary key could be different, but we assume id for now
            let redis_key = format!("orm:{}:{}", #table_name, self.id);

            let fields: Vec<(&str, String)> = vec![
                #(#to_hash_fields),*
            ];

            let _: () = conn.hset_multiple(&redis_key, &fields).await?;
            Ok(())
        }

        #[cfg(feature = "redis")]
        pub async fn get_from_redis(id: impl std::fmt::Display) -> Result<Option<Self>, rullst_orm::Error> {
            use rullst_orm::_redis::AsyncCommands;
            let mut conn = rullst_orm::Orm::redis_manager()?;

            let redis_key = format!("orm:{}:{}", #table_name, id);

            let hash: std::collections::HashMap<String, String> = conn.hgetall(&redis_key).await?;

            if hash.is_empty() {
                return Ok(None);
            }

            let instance = Self {
                #(#from_hash_fields,)*
                #(#relation_field_idents: None,)*
                #skip_tail
            };

            Ok(Some(instance))
        }

        #[cfg(feature = "redis")]
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
