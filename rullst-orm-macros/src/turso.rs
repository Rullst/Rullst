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

    let opaque_columns = fields
        .iter()
        .filter(|field| {
            parsed
                .encrypted_fields
                .iter()
                .any(|encrypted| encrypted.name == **field)
        })
        .map(database_field_name)
        .collect::<Vec<_>>();

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

            fn opaque_columns() -> &'static [&'static str] {
                &[#(#opaque_columns),*]
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
    // The parser already limits table and column names to 1-64 ASCII
    // letters, digits or underscores that do not start with a digit.
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
        || !parsed.db_default_fields.is_empty()
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

        // The Turso codec writes every field, so it cannot honor db_default.
        let db_default: DeriveInput = parse_quote! {
            #[orm(table = "users", backend = "turso")]
            struct User { id: i64, #[orm(db_default)] created_at: Option<String> }
        };
        let parsed =
            parser::parse(&db_default).expect("model should parse before backend validation");
        assert!(generate(&parsed, &db_default).is_err());
        let plain: DeriveInput = parse_quote! {
            #[orm(table = "users", backend = "turso")]
            struct User { id: i64, created_at: Option<String> }
        };
        let parsed = parser::parse(&plain).expect("plain Turso model");
        assert!(generate(&parsed, &plain).is_ok());
    }

    fn generated(input: &DeriveInput) -> Result<String, String> {
        let parsed = parser::parse(input).expect("model should parse before backend validation");
        generate(&parsed, input)
            .map(|tokens| tokens.to_string())
            .map_err(|error| error.to_string())
    }

    #[test]
    fn each_unsupported_model_behavior_fails_on_its_own() {
        let models: [DeriveInput; 14] = [
            parse_quote! { #[orm(backend = "turso", auditable)] struct A { id: i64 } },
            parse_quote! { #[orm(backend = "turso", searchable)] struct A { id: i64 } },
            parse_quote! { #[orm(backend = "turso", global_scope = "live")] struct A { id: i64 } },
            parse_quote! {
                #[orm(backend = "turso", tenant_column = "tenant_id")]
                struct A { id: i64, tenant_id: String }
            },
            parse_quote! { #[orm(backend = "turso", policy = "APolicy")] struct A { id: i64 } },
            parse_quote! { #[orm(backend = "turso", before_save = "hook")] struct A { id: i64 } },
            parse_quote! { #[orm(backend = "turso", after_save = "hook")] struct A { id: i64 } },
            parse_quote! { #[orm(backend = "turso", before_delete = "hook")] struct A { id: i64 } },
            parse_quote! { #[orm(backend = "turso", after_delete = "hook")] struct A { id: i64 } },
            parse_quote! { #[orm(backend = "turso", after_fetch = "hook")] struct A { id: i64 } },
            parse_quote! {
                #[orm(backend = "turso")]
                struct A { id: i64, #[orm(rag_context)] bio: String }
            },
            parse_quote! {
                #[orm(backend = "turso")]
                struct A { id: i64, #[orm(db_default)] created_at: Option<String> }
            },
            parse_quote! {
                #[orm(backend = "turso")]
                struct A {
                    id: i64,
                    bio: String,
                    #[orm(embedding_for = "bio")]
                    embedding: Option<Vector>,
                }
            },
            parse_quote! { #[orm(backend = "turso")] struct A { id: i64 } },
        ];
        let (supported, unsupported) = models.split_last().expect("models");
        for model in unsupported {
            assert_eq!(
                generated(model).expect_err("unsupported behavior must fail"),
                "this #[orm(...)] model behavior is not supported by the bounded Turso-primary profile",
                "{}",
                quote!(#model)
            );
        }
        assert!(generated(supported).is_ok());
    }

    #[test]
    fn turso_models_keep_a_leading_underscore_table() {
        let underscored: DeriveInput = parse_quote! {
            #[orm(table = "_legacy_users", backend = "turso")]
            struct User { id: i64 }
        };
        assert!(
            generated(&underscored)
                .expect("a leading underscore is portable")
                .contains("fn table_name () -> & 'static str { \"_legacy_users\" }")
        );
    }

    #[test]
    fn generated_turso_model_keys_on_id_and_encrypts_only_encrypted_columns() {
        let input: DeriveInput = parse_quote! {
            #[orm(table = "notes", backend = "turso")]
            struct Note {
                title: String,
                id: i64,
                kind: String,
                #[orm(encrypted)]
                secret: String,
                #[orm(encrypted)]
                hint: Option<String>,
            }
        };
        let tokens = generated(&input).expect("Turso model");
        let normalized = tokens.replace(' ', "");
        assert!(
            normalized.contains("&[\"title\",\"id\",\"kind\",\"secret\",\"hint\"]"),
            "{tokens}"
        );
        assert!(
            normalized
                .contains("fnopaque_columns()->&'static[&'staticstr]{&[\"secret\",\"hint\"]}")
        );
        assert!(
            normalized.contains("<i64asrullst_orm::polyglot::TursoCodec>::encode_turso(&self.id)")
        );
        assert!(
            normalized.contains("rullst_orm::polyglot::TursoCodec::encode_turso(&self.title)?")
        );
        assert!(normalized.contains(
            "rullst_orm::privacy::encrypt_model_field(&self.secret,\"notes\",\"secret\",)"
        ));
        assert!(normalized.contains(
            "matchself.hint.as_deref(){Some(value)=>rullst_orm::polyglot::TursoValue::Text(rullst_orm::privacy::encrypt_model_field(value,\"notes\",\"hint\")"
        ));
        assert!(
            normalized.contains("title:<Stringasrullst_orm::polyglot::TursoCodec>::decode_turso")
        );
        assert!(normalized.contains("secret:rullst_orm::privacy::decrypt_model_field(&<Stringasrullst_orm::polyglot::TursoCodec>::decode_turso(row.get(\"secret\")"));
        assert!(normalized.contains("hint:match<Option<String>asrullst_orm::polyglot::TursoCodec>::decode_turso(row.get(\"hint\")"));
        assert!(!normalized.contains("encrypt_model_field(&self.title"));
        assert!(!normalized.contains("encrypt_model_field(&self.id"));
    }
}
