use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::parser::{EncryptedFieldKind, ParsedModel};

pub fn generate(parsed: &ParsedModel, input: &DeriveInput) -> Result<TokenStream, syn::Error> {
    validate(parsed, input)?;

    let name = &parsed.name;
    let table = &parsed.table_name;
    let fields = &parsed.normal_fields;
    let field_names = fields.iter().map(database_field_name).collect::<Vec<_>>();
    let types = &parsed.normal_fields_types;
    let id_index = field_names
        .iter()
        .position(|field| field == "id")
        .ok_or_else(|| syn::Error::new_spanned(input, "TursoModel requires an `id` field"))?;
    let id_field = &fields[id_index];
    let id_type = &types[id_index];

    let encoders = fields.iter().map(|field| {
        let field_name = database_field_name(field);
        match parsed
            .encrypted_fields
            .iter()
            .find(|encrypted| encrypted.name == *field)
            .map(|encrypted| encrypted.kind)
        {
            Some(EncryptedFieldKind::String) => quote! {
                rullst_orm::polyglot::TursoValue::Text(
                    rullst_orm::privacy::encrypt_model_field(
                        &self.#field,
                        #table,
                        #field_name,
                    )
                    .map_err(rullst_orm::polyglot::PolyglotError::serialization_public)?,
                )
            },
            Some(EncryptedFieldKind::OptionalString) => quote! {
                match self.#field.as_deref() {
                    Some(value) => rullst_orm::polyglot::TursoValue::Text(
                        rullst_orm::privacy::encrypt_model_field(value, #table, #field_name)
                            .map_err(rullst_orm::polyglot::PolyglotError::serialization_public)?,
                    ),
                    None => rullst_orm::polyglot::TursoValue::Null,
                }
            },
            None => quote! {
                rullst_orm::polyglot::TursoCodec::encode_turso(&self.#field)?
            },
        }
    });

    let decoders = fields.iter().zip(types).map(|(field, field_type)| {
        let field_name = database_field_name(field);
        let decoded = quote! {
            <#field_type as rullst_orm::polyglot::TursoCodec>::decode_turso(
                row.get(#field_name).ok_or_else(|| {
                    rullst_orm::polyglot::PolyglotError::Serialization(
                        format!("result is missing required column {}", #field_name),
                    )
                })?,
            )?
        };
        match parsed
            .encrypted_fields
            .iter()
            .find(|encrypted| encrypted.name == *field)
            .map(|encrypted| encrypted.kind)
        {
            Some(EncryptedFieldKind::String) => quote! {
                #field: rullst_orm::privacy::decrypt_model_field(
                    &#decoded,
                    #table,
                    #field_name,
                )
                .map_err(rullst_orm::polyglot::PolyglotError::serialization_public)?
            },
            Some(EncryptedFieldKind::OptionalString) => quote! {
                #field: match #decoded {
                    Some(value) => Some(
                        rullst_orm::privacy::decrypt_model_field(
                            &value,
                            #table,
                            #field_name,
                        )
                        .map_err(rullst_orm::polyglot::PolyglotError::serialization_public)?,
                    ),
                    None => None,
                }
            },
            None => quote! { #field: #decoded },
        }
    });

    let defaults = parsed
        .skipped_fields
        .iter()
        .chain(parsed.relations.iter().map(|relation| &relation.field_name))
        .map(|field| quote! { #field: ::core::default::Default::default() });

    Ok(quote! {
        impl rullst_orm::polyglot::TursoModel for #name {
            fn table_name() -> &'static str {
                #table
            }

            fn columns() -> &'static [&'static str] {
                &[#(#field_names),*]
            }

            fn primary_key_column() -> &'static str {
                "id"
            }

            fn encode_turso(
                &self,
            ) -> Result<Vec<rullst_orm::polyglot::TursoValue>, rullst_orm::polyglot::PolyglotError> {
                Ok(vec![#(#encoders),*])
            }

            fn decode_turso(
                row: &rullst_orm::polyglot::TursoRow,
            ) -> Result<Self, rullst_orm::polyglot::PolyglotError> {
                Ok(Self {
                    #(#decoders,)*
                    #(#defaults,)*
                })
            }

            fn primary_key_value(
                &self,
            ) -> Result<rullst_orm::polyglot::TursoValue, rullst_orm::polyglot::PolyglotError> {
                <#id_type as rullst_orm::polyglot::TursoCodec>::encode_turso(&self.#id_field)
            }

            fn primary_key_is_unset(&self) -> bool {
                <#id_type as rullst_orm::polyglot::TursoPrimaryKey>::is_unset(&self.#id_field)
            }

            fn assign_primary_key(
                &mut self,
                value: &rullst_orm::polyglot::TursoValue,
            ) -> Result<(), rullst_orm::polyglot::PolyglotError> {
                <#id_type as rullst_orm::polyglot::TursoPrimaryKey>::assign_turso(
                    &mut self.#id_field,
                    value,
                )
            }
        }

        impl #name {
            pub fn query() -> Result<
                rullst_orm::polyglot::TursoQuery<'static, Self>,
                rullst_orm::polyglot::PolyglotError,
            > {
                <Self as rullst_orm::polyglot::TursoActiveRecord>::query()
            }

            pub async fn all() -> Result<Vec<Self>, rullst_orm::polyglot::PolyglotError> {
                <Self as rullst_orm::polyglot::TursoActiveRecord>::all().await
            }

            pub async fn find<Key>(
                key: Key,
            ) -> Result<Option<Self>, rullst_orm::polyglot::PolyglotError>
            where
                Key: rullst_orm::polyglot::TursoCodec + Send,
            {
                <Self as rullst_orm::polyglot::TursoActiveRecord>::find(key).await
            }

            pub async fn save(
                &mut self,
            ) -> Result<(), rullst_orm::polyglot::PolyglotError> {
                <Self as rullst_orm::polyglot::TursoActiveRecord>::save(self).await
            }

            pub async fn create(
                &mut self,
            ) -> Result<(), rullst_orm::polyglot::PolyglotError> {
                <Self as rullst_orm::polyglot::TursoActiveRecord>::create(self).await
            }

            pub async fn delete(&self) -> Result<(), rullst_orm::polyglot::PolyglotError> {
                <Self as rullst_orm::polyglot::TursoActiveRecord>::delete(self).await
            }

            pub async fn count() -> Result<u64, rullst_orm::polyglot::PolyglotError> {
                <Self as rullst_orm::polyglot::TursoActiveRecord>::count().await
            }

            pub async fn paginate(
                page: usize,
                per_page: usize,
            ) -> Result<
                rullst_orm::PaginationResult<Self>,
                rullst_orm::polyglot::PolyglotError,
            > {
                <Self as rullst_orm::polyglot::TursoActiveRecord>::paginate(page, per_page).await
            }
        }
    })
}

fn database_field_name(field: &syn::Ident) -> String {
    let name = field.to_string();
    name.strip_prefix("r#").unwrap_or(&name).to_owned()
}

fn validate(parsed: &ParsedModel, input: &DeriveInput) -> Result<(), syn::Error> {
    let valid_table = !parsed.table_name.is_empty()
        && parsed.table_name.len() <= 128
        && parsed
            .table_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        && parsed
            .table_name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic() || character == '_');
    if !valid_table {
        return Err(syn::Error::new_spanned(
            input,
            "TursoModel table names must be 1-128 ASCII letters, digits, or underscores and cannot start with a digit",
        ));
    }
    if !parsed.relations.is_empty() {
        return Err(syn::Error::new_spanned(
            input,
            "Turso-primary models do not yet support #[orm(...)] relation attributes",
        ));
    }
    if parsed.has_soft_deletes {
        return Err(syn::Error::new_spanned(
            input,
            "Turso-primary models do not yet support soft-delete semantics",
        ));
    }
    let unsupported_model_behavior = parsed.auditable
        || parsed.searchable
        || !parsed.global_scope.is_empty()
        || !parsed.tenant_column.is_empty()
        || !parsed.policy.is_empty()
        || !parsed.before_save.is_empty()
        || !parsed.after_save.is_empty()
        || !parsed.before_delete.is_empty()
        || !parsed.after_delete.is_empty()
        || !parsed.after_fetch.is_empty()
        || !parsed.rag_context_fields.is_empty()
        || parsed.embedding_for.is_some();
    if unsupported_model_behavior {
        return Err(syn::Error::new_spanned(
            input,
            "this #[orm(...)] model behavior is not supported by the bounded Turso-primary profile",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use syn::parse_quote;

    #[test]
    fn unsupported_sqlx_model_semantics_fail_closed() {
        let relation: DeriveInput = parse_quote! {
            #[orm(table = "users", backend = "turso")]
            struct User {
                id: i64,
                #[orm(has_many = "Post")]
                posts: Vec<Post>,
            }
        };
        let parsed =
            parser::parse(&relation).expect("model should parse before backend validation");
        assert!(generate(&parsed, &relation).is_err());

        let soft_delete: DeriveInput = parse_quote! {
            #[orm(table = "users", backend = "turso")]
            struct User { id: i64, deleted_at: Option<String> }
        };
        let parsed =
            parser::parse(&soft_delete).expect("model should parse before backend validation");
        assert!(generate(&parsed, &soft_delete).is_err());
    }
}
