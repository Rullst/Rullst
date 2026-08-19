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

    let mut to_json_fields = vec![];
    for field_name in normal_fields {
        let field_name_str = field_name.to_string();
        if !hidden_fields.contains(field_name) {
            to_json_fields.push(quote! {
                map.insert(#field_name_str.to_string(), rullst_orm::_serde_json::json!(self.#field_name));
            });
        }
    }

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

        pub fn to_cache_json(&self) -> String {
            let mut map = rullst_orm::_serde_json::Map::new();
            #(
                map.insert(stringify!(#normal_fields).to_string(), rullst_orm::_serde_json::json!(self.#normal_fields));
            )*
            rullst_orm::_serde_json::Value::Object(map).to_string()
        }

        pub fn to_cache_json_array(models: &[Self]) -> String {
            let json_values: Vec<rullst_orm::_serde_json::Value> = models.iter().map(|m| {
                let mut map = rullst_orm::_serde_json::Map::new();
                #(
                    map.insert(stringify!(#normal_fields).to_string(), rullst_orm::_serde_json::json!(m.#normal_fields));
                )*
                rullst_orm::_serde_json::Value::Object(map)
            }).collect();
            rullst_orm::_serde_json::Value::Array(json_values).to_string()
        }

        pub fn from_cache_json(json_str: &str) -> Result<Self, rullst_orm::_serde_json::Error> {
            Self::from_json(json_str)
        }

        pub fn from_cache_json_array(json_str: &str) -> Result<Vec<Self>, rullst_orm::_serde_json::Error> {
            Self::from_json_array(json_str)
        }

        pub fn to_json(&self) -> String {
            let mut map = rullst_orm::_serde_json::Map::new();
            #(#to_json_fields)*
            rullst_orm::_serde_json::Value::Object(map).to_string()
        }
    }
}
