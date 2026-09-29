use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_json_methods(parsed: &ParsedModel) -> TokenStream {
    let normal_fields = &parsed.normal_fields;
    let hidden_fields = &parsed.hidden_fields;
    let skipped_fields = &parsed.skipped_fields;
    let mut relation_field_idents = vec![];
    for rel in &parsed.relations {
        relation_field_idents.push(rel.field_name.clone());
    }

    // `to_json()` feeds audit rows and committed events: hidden fields are
    // omitted and encrypted/masked values are replaced by the audit marker.
    // The Scout document omits every hidden, encrypted and masked field.
    let mut to_json_fields = vec![];
    let mut search_fields = vec![];
    for field_name in normal_fields {
        let field_name_str = field_name.to_string();
        if hidden_fields.contains(field_name) {
            continue;
        }
        if parsed.is_redacted(field_name) {
            to_json_fields.push(quote! {
                map.insert(
                    #field_name_str.to_string(),
                    rullst_orm::_serde_json::Value::String(rullst_orm::audit::REDACTED_VALUE.to_string()),
                );
            });
        } else {
            let insert = quote! {
                map.insert(#field_name_str.to_string(), rullst_orm::_serde_json::json!(self.#field_name));
            };
            to_json_fields.push(insert.clone());
            search_fields.push(insert);
        }
    }
    let search_json = if parsed.searchable {
        quote! {
            /// Search-provider document without hidden, encrypted or masked fields.
            fn __rullst_search_json(&self) -> String {
                rullst_orm::privacy::with_redacted_secrets(|| {
                    let mut map = rullst_orm::_serde_json::Map::new();
                    #(#search_fields)*
                    rullst_orm::_serde_json::Value::Object(map).to_string()
                })
            }
        }
    } else {
        quote! {}
    };
    let redacted_changes = if parsed.auditable {
        let comparisons = normal_fields
            .iter()
            .filter(|field| !hidden_fields.contains(field) && parsed.is_redacted(field))
            .map(|field| {
                let key = field.to_string();
                let compared_in_memory = parsed.secret_fields.contains(field)
                    || parsed.encrypted_fields.iter().any(|encrypted| encrypted.name == *field);
                if compared_in_memory {
                    quote! {
                        if self.#field != previous.#field {
                            changed.push(#key);
                        }
                    }
                } else {
                    quote! {
                        if rullst_orm::audit::redacted_value_changed(&self.#field, &previous.#field) {
                            changed.push(#key);
                        }
                    }
                }
            });
        quote! {
            /// Names of redacted fields whose in-memory values differ from `previous`.
            fn __rullst_redacted_changes(&self, previous: &Self) -> Vec<&'static str> {
                #[allow(unused_mut)]
                let mut changed = Vec::new();
                #(#comparisons)*
                changed
            }
        }
    } else {
        quote! {}
    };

    let skip_tail = if skipped_fields.is_empty() {
        // No `#[orm(skip)]` / `#[sqlx(skip)]` fields, so the
        // exhaustive struct literal is fine and we don't force the
        // user model to implement `Default`.
        quote! {}
    } else {
        // When a model has skipped fields the struct literal
        // intentionally omits them; trailing `..Default::default()`
        // fills them in. Users must therefore add
        // `#[derive(Default)]` (or implement `Default` manually) on
        // any model that opts into `#[orm(skip)]`.
        quote! { ..Default::default() }
    };

    quote! {
        pub fn from_json(json_str: &str) -> Result<Self, rullst_orm::_serde_json::Error> {
            let value: rullst_orm::_serde_json::Value = rullst_orm::_serde_json::from_str(json_str)?;
            Self::from_json_value(value)
        }

        pub fn from_json_value(value: rullst_orm::_serde_json::Value) -> Result<Self, rullst_orm::_serde_json::Error> {
            Ok(Self {
                #(
                    #normal_fields: rullst_orm::_serde_json::from_value(value[stringify!(#normal_fields)].clone())?,
                )*
                #(
                    #relation_field_idents: None,
                )*
                #skip_tail
            })
        }

        pub fn from_json_array(json_str: &str) -> Result<Vec<Self>, rullst_orm::_serde_json::Error> {
            let value: rullst_orm::_serde_json::Value = rullst_orm::_serde_json::from_str(json_str)?;
            if let rullst_orm::_serde_json::Value::Array(arr) = value {
                let mut results = Vec::with_capacity(arr.len());
                for item in arr {
                    results.push(Self::from_json_value(item)?);
                }
                Ok(results)
            } else {
                Ok(vec![])
            }
        }

        /// Full persisted state for the query cache and revision restore.
        /// `SecretString` values serialize as encrypted envelopes, so this fails
        /// when no encryption key is configured.
        fn __rullst_cache_json_value(&self) -> Result<rullst_orm::_serde_json::Value, rullst_orm::_serde_json::Error> {
            let mut map = rullst_orm::_serde_json::Map::new();
            #(
                map.insert(
                    stringify!(#normal_fields).to_string(),
                    rullst_orm::_serde_json::to_value(&self.#normal_fields)?,
                );
            )*
            Ok(rullst_orm::_serde_json::Value::Object(map))
        }

        fn __rullst_try_cache_json_array(models: &[Self]) -> Result<String, rullst_orm::_serde_json::Error> {
            let values = models
                .iter()
                .map(Self::__rullst_cache_json_value)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rullst_orm::_serde_json::Value::Array(values).to_string())
        }

        /// Serializes every persisted field for caching. Returns an empty
        /// string when a field cannot be serialized, for example a
        /// `SecretString` without a configured encryption key.
        pub fn to_cache_json(&self) -> String {
            self.__rullst_cache_json_value()
                .map(|value| value.to_string())
                .unwrap_or_default()
        }

        /// Array form of [`Self::to_cache_json`]; empty string on failure.
        pub fn to_cache_json_array(models: &[Self]) -> String {
            Self::__rullst_try_cache_json_array(models).unwrap_or_default()
        }

        pub fn from_cache_json(json_str: &str) -> Result<Self, rullst_orm::_serde_json::Error> {
            Self::from_json(json_str)
        }

        pub fn from_cache_json_array(json_str: &str) -> Result<Vec<Self>, rullst_orm::_serde_json::Error> {
            Self::from_json_array(json_str)
        }

        /// Audit/event projection: hidden fields are omitted and
        /// `#[orm(encrypted)]`, `#[orm(masked)]` and `SecretString` values
        /// (including nested ones) become `"***"`.
        pub fn to_json(&self) -> String {
            rullst_orm::privacy::with_redacted_secrets(|| {
                let mut map = rullst_orm::_serde_json::Map::new();
                #(#to_json_fields)*
                rullst_orm::_serde_json::Value::Object(map).to_string()
            })
        }

        #search_json
        #redacted_changes
    }
}
