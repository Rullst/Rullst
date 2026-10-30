use super::{
    deleted_effects, execute_mutation, instance_hook, saved_redis_effects, tenant_guard,
    tenant_scoped_invalidation,
};
use syn::{DeriveInput, parse_quote};

fn parse(input: DeriveInput) -> crate::parser::ParsedModel {
    crate::parser::parse(&input).expect("parse model")
}

#[test]
fn tenant_guard_compares_the_handle_with_the_active_tenant() {
    let tenant = parse(parse_quote! {
        #[orm(table = "posts", tenant_column = "tenant_id")]
        struct Post { id: i32, tenant_id: String }
    });
    let guard = tenant_guard(&tenant).to_string();
    assert!(
        guard.contains("let expected_tenant : String = tenant . try_into ()"),
        "{guard}"
    );
    assert!(
        guard.contains("if self . tenant_id != expected_tenant"),
        "{guard}"
    );
    assert!(guard.contains("\"tenant context is required to mutate `{}`\" , \"posts\""));

    let global = parse(parse_quote! { struct Post { id: i32 } });
    assert!(tenant_guard(&global).is_empty());
}

#[test]
fn instance_hooks_run_inside_the_mutation() {
    let parsed = parse(parse_quote! { struct Post { id: i32 } });
    assert_eq!(
        instance_hook(&parsed, "before_delete_hook").to_string(),
        "rullst_orm :: __transaction_access :: run (self . before_delete_hook ()) . await ? ;"
    );
    assert!(instance_hook(&parsed, "").is_empty());
}

#[test]
fn deleted_effects_publish_notify_observers_and_remove_from_search() {
    let searchable = parse(parse_quote! {
        #[orm(table = "posts", searchable)]
        struct Post { id: i32 }
    });
    let effects = deleted_effects(&searchable).to_string();
    for expected in [
        "\"orm:events:{}:deleted\"",
        "rullst_orm :: ModelOperation :: Deleted",
        "observer . committed (& event)",
        "engine . delete (event . table , event . id)",
        "rullst_orm :: query_cache :: invalidate_model_table",
    ] {
        assert!(effects.contains(expected), "{expected}: {effects}");
    }
    let plain = parse(parse_quote! { #[orm(table = "posts")] struct Post { id: i32 } });
    assert!(
        !deleted_effects(&plain)
            .to_string()
            .contains("engine . delete")
    );
}

#[test]
fn saved_effects_publish_the_operation_and_saved_topics() {
    let parsed = parse(parse_quote! { #[orm(table = "posts")] struct Post { id: i32 } });
    let effects = saved_redis_effects(&parsed).to_string();
    for expected in [
        "\"orm:events:{}:{}\"",
        "\"orm:events:{}:saved\"",
        "rullst_orm :: query_cache :: invalidate_model_table",
        "\"posts\"",
    ] {
        assert!(effects.contains(expected), "{expected}: {effects}");
    }
}

#[test]
fn mutations_execute_on_the_savepoint_under_the_query_timeout() {
    let execute = execute_mutation().to_string();
    assert!(
        execute.contains(
            "rullst_orm :: _tokio :: time :: timeout (t , exec . execute (& mut * * tx))"
        )
    );
    assert!(execute.contains("\"Query execution timed out\""));
    assert!(execute.contains("exec . execute (& mut * * tx) . await ?"));
    let invalidation = tenant_scoped_invalidation().to_string();
    assert!(
        invalidation
            .contains("if Self :: __RULLST_TENANT_SCOPED_CACHE { cache_tenant } else { None }")
    );
}
