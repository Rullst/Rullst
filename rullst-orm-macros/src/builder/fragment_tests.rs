//! Generated query-builder fragments asserted on their tokens.

use syn::{DeriveInput, parse_quote};

fn parse(input: DeriveInput) -> crate::parser::ParsedModel {
    crate::parser::parse(&input).expect("parse model")
}

#[test]
fn pluck_string_refuses_only_nullable_encrypted_columns() {
    let parsed = parse(parse_quote! {
        #[orm(table = "people")]
        struct Person {
            id: i32,
            #[orm(encrypted)]
            ssn: String,
            #[orm(encrypted)]
            nickname: Option<String>,
        }
    });
    let methods = super::pluck::generate_pluck_methods(&parsed).to_string();
    assert!(
        methods.contains("const ENCRYPTED_OPTIONAL_COLUMNS : & [& str] = & [\"nickname\"] ;"),
        "{methods}"
    );
    assert!(methods.contains("pub async fn pluck_string"));
    assert!(methods.contains("pub async fn pluck_i32"));
    assert!(methods.contains("decrypt_model_field (& value , \"people\" , declared)"));
    assert!(methods.contains("async fn pluck_integers_with_executor"));
}

#[test]
fn cached_reads_decrypt_only_models_with_encrypted_fields() {
    let encrypted = parse(parse_quote! {
        #[orm(table = "people", tenant_column = "tenant_id")]
        struct Person { id: i32, tenant_id: String, #[orm(encrypted)] ssn: String }
    });
    let read = super::query_cache::generate_cache_read(&encrypted).to_string();
    assert!(
        read.contains("model . __rullst_decrypt_encrypted_fields () ?"),
        "{read}"
    );
    assert!(
        read.contains("query_key_for_model (\"people\" , true , & query_str , & query_bindings")
    );
    assert!(read.contains("let mut model = Person :: from_json_value (item) ?"));

    let plain = parse(parse_quote! { #[orm(table = "notes")] struct Note { id: i32 } });
    let read = super::query_cache::generate_cache_read(&plain).to_string();
    assert!(!read.contains("__rullst_decrypt_encrypted_fields"));
    assert!(read.contains("query_key_for_model (\"notes\" , false"));

    let write = super::query_cache::generate_cache_write(&plain.name).to_string();
    assert!(
        write.contains("Note :: __rullst_try_cache_json_array (& results)"),
        "{write}"
    );
    assert!(
        write.contains("rullst_orm :: query_cache :: store_entry (cache_key , & serialized , ttl)")
    );
    assert!(write.contains("\"remember() TTL exceeds the Redis-supported range\""));
}

#[test]
fn relevance_order_ranks_the_table_qualified_id() {
    let parsed = parse(parse_quote! { #[orm(table = "posts")] struct Post { id: i32 } });
    let relevance = super::relevance::generate(&parsed).to_string();
    assert!(
        relevance.contains("order . push_str (\"posts.id\")"),
        "{relevance}"
    );
    assert!(
        relevance.contains("fn __rullst_order_by_relevance (mut self , ids : & [i32]) -> Self")
    );
    assert!(relevance.contains("fn __rullst_order_bindings (& self)"));
}
