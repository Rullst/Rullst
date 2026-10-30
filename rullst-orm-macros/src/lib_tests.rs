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
    assert!(models.contains("PostCommitScope :: new"));
    assert!(models.contains("after_commit"));
    assert!(models.contains("scout :: get_search_engine"));
}

#[test]
fn generated_query_cache_is_transaction_aware_and_strongly_namespaced() {
    let input: DeriveInput = parse_quote! {
        #[derive(Orm)]
        #[orm(table = "users", tenant = "organization_id")]
        pub struct User {
            pub id: i32,
            pub organization_id: i32,
            pub name: String,
        }
    };
    let (_, builder, models) = run_macro_generator(&input);

    assert!(builder.contains("query_cache :: query_key"));
    assert!(builder.contains("CURRENT_TX"));
    assert!(builder.contains("get_with_tx_internal (& mut * * tx , false)"));
    assert!(builder.contains("get_with_tx_internal (pool , true)"));
    assert!(builder.contains("TTL greater than zero"));
    assert!(!builder.contains("DefaultHasher"));
    assert!(models.contains("find_with_tx"));
    assert!(models.contains("Transaction < '_ >"));
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
            #[orm(belongs_to = "User", foreign_key = "user_id")]
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
fn generates_typed_inverse_polymorphic_loading() {
    let input: DeriveInput = parse_quote! {
        #[derive(Orm)]
        pub struct Comment {
            pub id: i32,
            pub commentable_id: i32,
            pub commentable_type: String,
            #[orm(morph_to = "Post", morph_name = "commentable")]
            post: Option<Post>,
        }
    };
    let (parsed, builder, models) = run_macro_generator(&input);

    assert_eq!(parsed.relations[0].rel_type, "morph_to");
    assert!(models.contains("self . commentable_type != stringify ! (Post)"));
    assert!(models.contains("self . commentable_id . clone"));
    assert!(builder.contains("related_by_id"));
    assert!(builder.contains("with_post"));
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
fn generated_query_spans_are_structured_and_do_not_capture_sql() {
    let input: DeriveInput = parse_quote! {
        #[derive(Orm)]
        pub struct Post {
            pub id: i32,
            pub title: String,
        }
    };
    let (_, builder, models) = run_macro_generator(&input);
    let generated = format!("{builder} {models}");

    assert!(generated.contains("rullst.orm.query"));
    assert!(generated.contains("orm . operation"));
    assert!(generated.contains("orm . table"));
    assert!(generated.contains("orm . model"));
    assert!(!generated.contains("fields ( sql"));
    assert!(!generated.contains("fields ( bindings"));
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
