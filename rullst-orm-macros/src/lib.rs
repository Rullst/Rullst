extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod builder;
mod enums;
mod factory_observer;
mod models;
mod nexus;
#[cfg_attr(mutants, mutants::skip)]
mod parser;
mod privacy;
mod relationships;
mod turso;

#[cfg_attr(test, mutants::skip)]
#[proc_macro_attribute]
pub fn test(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;
    let vis = &input_fn.vis;
    let block = &input_fn.block;
    let attrs = &input_fn.attrs;

    let expanded = quote::quote! {
        #(#attrs)*
        #[::tokio::test]
        #vis async fn #fn_name() {
            // Ensure DB is initialized (if already initialized in parallel, it ignores the error)
            let _ = ::rullst_orm::Orm::init(&::std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string())).await;

            // Start transaction for Sandbox isolation
            let tx = ::rullst_orm::Orm::begin_transaction().await.expect("Failed to begin sandbox transaction");
            let tx_arc = ::std::sync::Arc::new(::tokio::sync::Mutex::new(Some(tx)));

            // Scope the transaction globally for this tokio task
            ::rullst_orm::CURRENT_TX.scope(tx_arc.clone(), async move {
                // Execute user's test
                let __test_closure = async move {
                    #block
                };
                __test_closure.await;
            }).await;

            // Automatic Rollback
            if let Some(tx) = tx_arc.lock().await.take() {
                let _ = tx.rollback().await;
            }
        }
    };
    TokenStream::from(expanded)
}

#[cfg_attr(test, mutants::skip)]
#[proc_macro_derive(PersonalData, attributes(privacy))]
pub fn derive_personal_data(input: TokenStream) -> TokenStream {
    privacy::derive_personal_data_impl(input)
}

#[cfg_attr(test, mutants::skip)]
#[proc_macro_derive(Enum)]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    enums::derive_enum_impl(input.into()).into()
}

#[cfg_attr(test, mutants::skip)]
#[proc_macro_derive(Nexus, attributes(nexus))]
pub fn derive_nexus(input: TokenStream) -> TokenStream {
    nexus::derive_nexus_impl(input)
}

#[cfg_attr(test, mutants::skip)]
#[proc_macro_derive(Orm, attributes(orm, sqlx))]
pub fn rullst_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Parse the input
    let parsed = match parser::parse(&input) {
        Ok(p) => p,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    if parsed.backend == "turso" {
        return match turso::generate(&parsed, &input) {
            Ok(expanded) => expanded.into(),
            Err(error) => error.to_compile_error().into(),
        };
    }

    // Generate relationships
    let rels = relationships::generate(&parsed);

    // Generate the builder
    let builder_code = builder::generate(
        &parsed,
        &rels.flags,
        &rels.inits,
        &rels.methods,
        &rels.eager_loads,
    );

    // Generate factory and observers
    let factory_observer_code = factory_observer::generate(&parsed);

    // Generate the model impl
    let model_code = models::generate(&parsed, &rels.model_methods);

    // Combine
    let expanded = quote::quote! {
        #builder_code
        #factory_observer_code
        #model_code
    };

    TokenStream::from(expanded)
}

/// Derives the typed Turso/libSQL model contract without generating SQLx code.
#[cfg_attr(test, mutants::skip)]
#[proc_macro_derive(TursoModel, attributes(orm, sqlx))]
pub fn derive_turso_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let parsed = match parser::parse(&input) {
        Ok(parsed) => parsed,
        Err(error) => return error.to_compile_error().into(),
    };
    match turso::generate(&parsed, &input) {
        Ok(expanded) => expanded.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::core::prelude::v1::test;
    use syn::parse_quote;

    fn run_macro_generator(input: &DeriveInput) -> (parser::ParsedModel, String, String) {
        let parsed = parser::parse(input).unwrap();
        let rels = relationships::generate(&parsed);
        let builder = builder::generate(
            &parsed,
            &rels.flags,
            &rels.inits,
            &rels.methods,
            &rels.eager_loads,
        );
        let _factory = factory_observer::generate(&parsed);
        let models = models::generate(&parsed, &rels.model_methods);
        (parsed, builder.to_string(), models.to_string())
    }

    fn assert_generated_runtime_is_zero_panic(label: &str, generated: &str) {
        for forbidden in [
            ". unwrap (",
            ". expect (",
            "panic !",
            "todo !",
            "unimplemented !",
        ] {
            assert!(
                !generated.contains(forbidden),
                "{label} emitted forbidden runtime code `{forbidden}`: {generated}"
            );
        }
    }

    #[test]
    fn test_basic_model() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            #[orm(table = "users", searchable)]
            pub struct User {
                pub id: i32,
                pub name: String,
                pub email: String,
            }
        };
        let (parsed, builder, models) = run_macro_generator(&input);
        assert_eq!(parsed.table_name, "users");
        assert!(builder.contains("where_id"));
        assert!(models.contains("fn delete"));
        assert!(models.contains("fn search"));
    }

    #[test]
    fn test_model_with_relations() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            pub struct Post {
                pub id: i32,
                pub title: String,
                #[orm(has_many = "Comment", foreign_key = "post_id", local_key = "id")]
                comments: Option<Vec<Comment>>,
                #[orm(has_one = "Author", foreign_key = "post_id", local_key = "id")]
                author: Option<Author>,
                #[orm(belongs_to = "User", foreign_key = "user_id", local_key = "id")]
                user: Option<User>,
                #[orm(belongs_to_many = "Tag", pivot_table = "post_tags", foreign_key = "post_id", related_key = "tag_id")]
                tags: Option<Vec<Tag>>,
                #[orm(morph_one = "Image", morph_name = "imageable")]
                image: Option<Image>,
                #[orm(morph_many = "Comment", morph_name = "commentable")]
                morph_comments: Option<Vec<Comment>>,
            }
        };
        let (parsed, _, _) = run_macro_generator(&input);
        assert!(!parsed.relations.is_empty());
    }

    #[test]
    fn generated_runtime_code_has_no_panicking_calls() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            #[orm(soft_delete(field = "deleted_at", value = "null", delval = "now()"))]
            pub struct Post {
                pub id: i32,
                pub title: String,
                pub deleted_at: Option<String>,
                #[orm(has_many = "Comment", foreign_key = "post_id", local_key = "id")]
                comments: Option<Vec<Comment>>,
            }
        };
        let (_, builder, models) = run_macro_generator(&input);

        assert_generated_runtime_is_zero_panic("ORM builder", &builder);
        assert_generated_runtime_is_zero_panic("ORM model", &models);
    }

    #[test]
    fn inconsistent_soft_delete_state_emits_compile_error() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            pub struct Post {
                pub id: i32,
                pub deleted_at: Option<String>,
            }
        };
        let mut parsed = parser::parse(&input).unwrap();
        assert!(parsed.has_soft_deletes);
        parsed.soft_delete = None;
        let rels = relationships::generate(&parsed);

        let builder = builder::generate(
            &parsed,
            &rels.flags,
            &rels.inits,
            &rels.methods,
            &rels.eager_loads,
        )
        .to_string();
        let models = models::generate(&parsed, &rels.model_methods).to_string();

        assert!(builder.contains("compile_error"));
        assert!(models.contains("compile_error"));
    }

    #[test]
    fn test_model_with_soft_deletes() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            pub struct User {
                pub id: i32,
                pub name: String,
                pub deleted_at: Option<String>,
            }
        };
        let (parsed, builder, _) = run_macro_generator(&input);
        assert!(parsed.has_soft_deletes);
        assert!(builder.contains("deleted_at IS NULL"));
    }

    #[test]
    fn test_model_with_hidden_fields() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            pub struct User {
                pub id: i32,
                pub name: String,
                #[orm(hidden)]
                pub password: String,
            }
        };
        let (parsed, _, models) = run_macro_generator(&input);
        assert_eq!(parsed.hidden_fields.len(), 1);
        assert!(models.contains("password"));
    }

    #[test]
    fn test_model_with_explicit_soft_delete_config() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            #[orm(soft_delete(field = "is_deleted", value = "0", delval = "1"))]
            pub struct Post {
                pub id: i32,
                pub title: String,
                pub is_deleted: i32,
            }
        };
        let (parsed, _, _) = run_macro_generator(&input);
        assert!(parsed.has_soft_deletes);
    }

    #[test]
    fn test_model_with_all_hooks_and_scopes() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            #[orm(global_scope = "active", tenant_column = "account_id", before_save = "hash_pwd", after_save = "log_evt", before_delete = "check_perm", after_delete = "clear_cache", after_fetch = "decrypt_data")]
            pub struct User {
                pub id: i32,
                pub account_id: String,
            }
        };
        let (parsed, _, _) = run_macro_generator(&input);
        assert_eq!(parsed.global_scope, "active");
        assert_eq!(parsed.tenant_column, "account_id");
        assert_eq!(parsed.before_save, "hash_pwd");
        assert_eq!(parsed.after_save, "log_evt");
        assert_eq!(parsed.before_delete, "check_perm");
        assert_eq!(parsed.after_delete, "clear_cache");
        assert_eq!(parsed.after_fetch, "decrypt_data");
    }

    #[test]
    fn test_model_with_soft_delete_null_sentinel() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            #[orm(soft_delete(field = "deleted_at", value = "null", delval = "now()"))]
            pub struct Audit {
                pub id: i32,
                pub message: String,
                pub deleted_at: Option<String>,
            }
        };
        run_macro_generator(&input);
    }

    #[test]
    fn test_model_with_soft_delete_bigint_timestamp() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            #[orm(soft_delete(field = "deleted_at", value = "0", delval = "UNIX_TIMESTAMP()"))]
            pub struct Article {
                pub id: i32,
                pub title: String,
                pub deleted_at: i64,
            }
        };
        run_macro_generator(&input);
    }

    #[test]
    fn test_model_with_orm_skip_field() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            pub struct Account {
                pub id: i32,
                pub name: String,
                #[orm(skip)]
                pub password_hash: String,
            }
        };
        run_macro_generator(&input);
    }

    #[test]
    fn test_model_with_sqlx_skip_field() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            pub struct Account {
                pub id: i32,
                pub name: String,
                #[sqlx(skip)]
                pub password_hash: String,
            }
        };
        run_macro_generator(&input);
    }

    #[test]
    fn test_model_with_combined_soft_delete_and_skip() {
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            #[orm(soft_delete(field = "is_active", value = "true", delval = "false"))]
            pub struct User {
                pub id: i32,
                pub name: String,
                pub is_active: bool,
                #[sqlx(skip)]
                pub internal_note: String,
            }
        };
        run_macro_generator(&input);
    }

    #[test]
    fn test_parser_errors() {
        // lowercase relation model
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            pub struct Post {
                pub id: i32,
                #[orm(has_many = "comment")]
                comments: Option<Vec<Comment>>,
            }
        };
        let res = parser::parse(&input);
        println!(
            "PARSE RESULT FOR LOWERCASE RELATION: {:?}",
            res.as_ref().map(|p| &p.table_name)
        );
        assert!(res.is_err());

        // empty table name
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            #[orm(table = "")]
            pub struct Post {
                pub id: i32,
            }
        };
        assert!(parser::parse(&input).is_err());

        // empty has_many
        let input: DeriveInput = parse_quote! {
            #[derive(Orm)]
            pub struct Post {
                pub id: i32,
                #[orm(has_many = "")]
                comments: Option<Vec<Comment>>,
            }
        };
        assert!(parser::parse(&input).is_err());
    }
}
