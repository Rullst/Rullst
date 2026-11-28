use super::{generate, model_qualifies, single_statement};
use quote::quote;
use syn::{DeriveInput, parse_quote};

fn parse(input: DeriveInput) -> crate::parser::ParsedModel {
    crate::parser::parse(&input).expect("parse model")
}

fn tracked(input: DeriveInput) -> String {
    single_statement(&parse(input)).to_string()
}

#[test]
fn only_models_without_audit_hooks_or_policy_qualify() {
    assert!(model_qualifies(&parse(parse_quote! {
        struct Tally { id: i32, hits: i32, #[sqlx(skip)] snapshot: RowSnapshot }
    })));
    assert!(model_qualifies(&parse(parse_quote! {
        #[orm(full_row_save, tenant_column = "org")]
        struct Tally { id: i32, org: String, hits: i32 }
    })));
    for input in [
        parse_quote! { #[orm(auditable, full_row_save)] struct Tally { id: i32, hits: i32 } },
        parse_quote! { #[orm(before_save = "stamp", full_row_save)] struct Tally { id: i32, hits: i32 } },
        parse_quote! { #[orm(after_save = "notify", full_row_save)] struct Tally { id: i32, hits: i32 } },
        parse_quote! { #[orm(policy = "TallyPolicy", full_row_save)] struct Tally { id: i32, hits: i32 } },
    ] {
        let parsed = parse(input);
        assert!(!model_qualifies(&parsed), "{}", parsed.name);
        let autocommit = generate(&parsed, &quote! {}, &quote! {}, &quote! {});
        assert!(autocommit.attempt.is_empty());
        assert!(autocommit.methods.is_empty());
    }
}

#[test]
fn full_row_saves_always_send_one_statement() {
    let parsed = parse(parse_quote! {
        #[orm(full_row_save)]
        struct Ledger { id: i32, code: String, #[orm(db_default)] created_at: Option<String> }
    });
    assert_eq!(single_statement(&parsed).to_string(), "true");
}

#[test]
fn tracked_updates_that_read_a_row_back_keep_their_transaction() {
    // `#[orm(db_default)]` and soft-delete columns are read back after every
    // UPDATE; an id-only model never updates. Only inserts qualify.
    for input in [
        parse_quote! {
            struct Stamp { id: i32, title: String, #[orm(db_default)] created_at: Option<String>, #[sqlx(skip)] snapshot: RowSnapshot }
        },
        parse_quote! {
            #[orm(soft_delete)]
            struct Note { id: i32, title: String, deleted_at: Option<String>, #[sqlx(skip)] snapshot: RowSnapshot }
        },
        parse_quote! { struct Ping { id: i32, #[sqlx(skip)] snapshot: RowSnapshot } },
    ] {
        assert_eq!(tracked(input), "self . id == 0");
    }
}

#[test]
fn tracked_updates_qualify_when_every_column_changed() {
    let condition = tracked(parse_quote! {
        struct Card { id: i32, code: String, title: String, #[sqlx(skip)] snapshot: RowSnapshot }
    });
    assert_eq!(
        condition,
        "self . id == 0 || self . __rullst_fingerprints () . iter () . enumerate () . all (| (index , fingerprint) | self . snapshot . __is_changed (index , * fingerprint))"
    );
}

#[test]
fn a_qualifying_save_checks_observers_and_the_timeout_then_runs_on_a_pool_connection() {
    let parsed = parse(parse_quote! {
        struct Card { id: i32, code: String, title: String, #[sqlx(skip)] snapshot: RowSnapshot }
    });
    let autocommit = generate(
        &parsed,
        &quote! { apply_tenant(); },
        &quote! { keep_snapshot(); },
        &quote! { restore_snapshot(); },
    );
    let attempt = autocommit.attempt.to_string();
    assert!(attempt.contains(". is_empty () ;"), "{attempt}");
    assert!(attempt.ends_with(
        "if self . __rullst_saves_in_one_statement (observed) ? { return self . __rullst_save_autocommit () . await ; }"
    ), "{attempt}");

    let methods = autocommit.methods.to_string();
    let (check, run) = methods
        .split_once("async fn __rullst_save_autocommit")
        .expect("both methods");
    assert!(check.contains(
        "if observed || rullst_orm :: schema :: get_query_timeout () . is_some () { return Ok (false) ; } apply_tenant () ; Ok (self . id == 0 ||"
    ), "{check}");
    let expected = [
        "let original_id = self . id ; keep_snapshot () ;",
        "let mut connection = rullst_orm :: Orm :: pool () ? . acquire () . await ? ;",
        "rullst_orm :: post_commit :: PostCommitScope :: new () ;",
        ". run (self . __rullst_save_observed (& mut * connection , is_new , Vec :: new () , true))",
        "drop (connection) ;",
        "if let Err (save_error) = save_result { self . id = original_id ; restore_snapshot () ; return Err (save_error) ; }",
        "post_commit . commit () . await",
    ];
    let mut rest = run;
    for part in expected {
        let (_, after) = rest
            .split_once(part)
            .unwrap_or_else(|| panic!("{part} in order: {run}"));
        rest = after;
    }
    assert!(!methods.contains("begin_transaction"), "{methods}");
}
