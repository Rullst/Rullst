use super::{
    TrackedUpdate, capture, clean_save_return, generate_methods, snapshot_guard, tracked_update,
    update_columns,
};
use quote::quote;
use syn::{DeriveInput, parse_quote};

fn parse(input: DeriveInput) -> crate::parser::ParsedModel {
    crate::parser::parse(&input).expect("parse model")
}

fn tracked_post() -> crate::parser::ParsedModel {
    parse(parse_quote! {
        #[orm(table = "posts")]
        struct Post {
            id: i32,
            title: String,
            #[orm(db_default)]
            created_at: Option<String>,
            body: String,
            deleted_at: Option<String>,
            #[sqlx(skip)]
            snapshot: rullst_orm::RowSnapshot,
        }
    })
}

#[test]
fn update_columns_leave_out_the_key_database_defaults_and_the_soft_delete_marker() {
    let parsed = tracked_post();
    let columns: Vec<String> = update_columns(&parsed)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(columns, ["title", "body"]);
    assert_eq!(
        parsed.snapshot_field.as_ref().map(ToString::to_string),
        Some("snapshot".to_string())
    );
}

#[test]
fn untracked_models_get_no_tracking_code() {
    let parsed = parse(parse_quote! { struct Post { id: i32, title: String } });
    assert!(parsed.snapshot_field.is_none());
    assert!(generate_methods(&parsed, &quote! {}).is_empty());
    assert!(capture(&parsed, &quote! { row }).is_empty());
    assert!(clean_save_return(&parsed).is_empty());
    let (keep, restore) = snapshot_guard(&parsed);
    assert!(keep.is_empty() && restore.is_empty());
    let update = TrackedUpdate {
        assignments: &[],
        bindings: &[],
        tenant_clause: "",
        tenant_binding: &quote! {},
        rows_check: &quote! {},
    };
    assert!(tracked_update(&parsed, &update).is_empty());
}

#[test]
fn tracked_models_expose_the_dirty_tracking_api() {
    let parsed = tracked_post();
    let methods = generate_methods(&parsed, &quote! { prepare_tenant (); }).to_string();
    assert!(
        methods.contains(
            "const __RULLST_TRACKED_COLUMNS : [& 'static str ; 2usize] = [\"title\" , \"body\"]"
        ),
        "{methods}"
    );
    assert!(methods.contains(
        "[rullst_orm :: snapshot :: fingerprint (& self . title) , rullst_orm :: snapshot :: fingerprint (& self . body)]"
    ));
    assert!(methods.contains(
        "self . snapshot = rullst_orm :: RowSnapshot :: __from_fingerprints (& fingerprints)"
    ));
    assert!(methods.contains("pub fn dirty_columns (& self) -> Vec < & 'static str >"));
    assert!(methods.contains(
        "self . id == 0 || self . snapshot . __any_changed (& self . __rullst_fingerprints ())"
    ));
    assert!(methods.contains("self . snapshot = rullst_orm :: RowSnapshot :: default ()"));
    let noop = methods
        .split_once("fn __rullst_save_is_noop")
        .expect("fast path")
        .1;
    assert!(noop.contains("if self . id == 0 { return Ok (false) ; }"));
    assert!(noop.contains(". is_empty ()"));
    assert!(noop.contains("prepare_tenant () ;"));
    assert!(noop.contains("Ok (! self . snapshot . __any_changed"));

    assert_eq!(
        capture(&parsed, &quote! { row }).to_string(),
        "row . __rullst_capture_snapshot () ;"
    );
    assert_eq!(
        clean_save_return(&parsed).to_string(),
        "if self . __rullst_save_is_noop () ? { return Ok (()) ; }"
    );
    let (keep, restore) = snapshot_guard(&parsed);
    assert_eq!(
        keep.to_string(),
        "let original_snapshot = self . snapshot . clone () ;"
    );
    assert_eq!(restore.to_string(), "self . snapshot = original_snapshot ;");
}

#[test]
fn hooks_and_policies_disable_the_clean_save_shortcut() {
    for parsed in [
        parse(parse_quote! {
            #[orm(before_save = "stamp")]
            struct Post { id: i32, title: String, #[sqlx(skip)] snapshot: RowSnapshot }
        }),
        parse(parse_quote! {
            #[orm(policy = "PostPolicy")]
            struct Post { id: i32, title: String, #[sqlx(skip)] snapshot: RowSnapshot }
        }),
    ] {
        let methods = generate_methods(&parsed, &quote! {}).to_string();
        assert!(methods.contains("fn mark_all_dirty"));
        assert!(!methods.contains("__rullst_save_is_noop"), "{methods}");
        assert!(clean_save_return(&parsed).is_empty());
    }
}

fn update_parts(
    parsed: &crate::parser::ParsedModel,
    tenant_clause: &str,
    tenant_binding: proc_macro2::TokenStream,
) -> String {
    let assignments = vec!["title = ?".to_string(), "body = ?".to_string()];
    let bindings = vec![
        quote! { .bind(self.title.clone()) },
        quote! { .bind(self.body.clone()) },
    ];
    let rows_check = quote! { check_rows(); };
    tracked_update(
        parsed,
        &TrackedUpdate {
            assignments: &assignments,
            bindings: &bindings,
            tenant_clause,
            tenant_binding: &tenant_binding,
            rows_check: &rows_check,
        },
    )
    .to_string()
}

#[test]
fn a_tracked_update_writes_only_changed_columns_and_reloads_the_rest() {
    let parsed = tracked_post();
    let update = update_parts(&parsed, "", quote! {});
    let clean = update
        .split_once("for obs in & observers")
        .expect("updating observers")
        .0;
    assert!(
        clean.contains(
            "if ! self . snapshot . __any_changed (& fingerprints) { return Ok (false) ; }"
        )
    );
    assert!(update.contains("let written : [bool ; 2usize]"));
    assert!(update.contains("if written [0usize] { assignments . push (\"title = ?\") ; }"));
    assert!(update.contains("if written [1usize] { assignments . push (\"body = ?\") ; }"));
    assert!(
        update.contains("if written [0usize] { query = query . bind (self . title . clone ()) ; }")
    );
    assert!(
        update.contains("if written [1usize] { query = query . bind (self . body . clone ()) ; }")
    );
    assert!(update.contains("if assignments . is_empty () { return Ok (false) ; }"));
    assert!(update.contains("let returning = driver != \"mysql\""));
    assert!(update.contains("final_sql . push_str (\" RETURNING *\")"));
    assert!(update.contains("\"posts\""));
    assert!(update.contains("query . bind (self . id) ;"));
    assert!(update.contains("check_rows () ;"));
    assert!(update.contains("\"SELECT * FROM posts WHERE id = ?\""));
    assert!(update.contains("return Err (rullst_orm :: Error :: RecordNotFound) ;"));
    assert!(update.contains("stored . __rullst_decrypt_encrypted_fields () ? ;"));
    assert!(update.contains(
        "if ! written [0usize] { :: core :: mem :: swap (& mut self . title , & mut stored . title) ; }"
    ));
    assert!(update.contains(
        ":: core :: mem :: swap (& mut self . created_at , & mut stored . created_at) ;"
    ));
    assert!(update.contains(
        ":: core :: mem :: swap (& mut self . deleted_at , & mut stored . deleted_at) ;"
    ));
    assert!(update.ends_with("self . __rullst_capture_snapshot () ;"));
}

#[test]
fn a_tenant_scoped_tracked_update_keeps_the_tenant_predicate() {
    let parsed = parse(parse_quote! {
        #[orm(table = "notes", tenant_column = "org")]
        struct Note { id: i32, org: String, title: String, body: String, #[sqlx(skip)] snapshot: RowSnapshot }
    });
    let update = update_parts(&parsed, " AND org = ?", quote! { .bind(self.org.clone()) });
    assert!(update.contains("\" AND org = ?\""));
    assert!(update.contains("query . bind (self . id) . bind (self . org . clone ()) ;"));
    assert!(update.contains("\"SELECT * FROM notes WHERE id = ? AND org = ?\""));
    assert!(update.contains("\"record is outside the active tenant scope\""));
    assert!(!update.contains("RecordNotFound"));
}

#[test]
fn a_tracked_model_without_updatable_columns_never_updates() {
    let parsed = parse(parse_quote! {
        struct Ping { id: i32, #[orm(db_default)] seen_at: Option<String>, #[sqlx(skip)] snapshot: RowSnapshot }
    });
    let update = TrackedUpdate {
        assignments: &[],
        bindings: &[],
        tenant_clause: "",
        tenant_binding: &quote! {},
        rows_check: &quote! {},
    };
    assert_eq!(
        tracked_update(&parsed, &update).to_string(),
        "if ! self . snapshot . __any_changed (& []) { return Ok (false) ; }"
    );
    let methods = generate_methods(&parsed, &quote! {}).to_string();
    assert!(
        methods.contains("[& 'static str ; 0usize] = []"),
        "{methods}"
    );
}

#[test]
fn row_snapshot_fields_must_be_a_single_sqlx_skipped_field() {
    let rejected: [DeriveInput; 4] = [
        parse_quote! { struct Post { id: i32, snapshot: RowSnapshot } },
        parse_quote! { struct Post { id: i32, #[orm(skip)] snapshot: RowSnapshot } },
        parse_quote! {
            struct Post { id: i32, #[sqlx(skip)] a: RowSnapshot, #[sqlx(skip)] b: RowSnapshot }
        },
        parse_quote! {
            struct Post { id: i32, #[sqlx(skip)] #[orm(has_one = "Image")] image: RowSnapshot }
        },
    ];
    let messages: Vec<String> = rejected
        .iter()
        .map(|input| match crate::parser::parse(input) {
            Ok(_) => panic!("`{}` must be rejected", input.ident),
            Err(error) => error.to_string(),
        })
        .collect();
    assert!(
        messages[0].contains("must be declared `#[sqlx(skip)]`"),
        "{}",
        messages[0]
    );
    assert!(
        messages[1].contains("must be declared `#[sqlx(skip)]`"),
        "{}",
        messages[1]
    );
    assert!(
        messages[2].contains("at most one `RowSnapshot` field"),
        "{}",
        messages[2]
    );
    assert!(
        messages[3].contains("must be declared `#[sqlx(skip)]`"),
        "{}",
        messages[3]
    );

    let accepted = parse(parse_quote! {
        struct Post { id: i32, #[orm(skip)] #[sqlx(skip)] state: rullst::db::RowSnapshot }
    });
    assert_eq!(
        accepted.snapshot_field.as_ref().map(ToString::to_string),
        Some("state".to_string())
    );
    assert!(accepted.skipped_fields.iter().any(|field| field == "state"));
    // A generic type that merely ends in the same name is not the snapshot.
    let generic = parse(parse_quote! {
        struct Post { id: i32, #[sqlx(skip)] state: RowSnapshot<u8> }
    });
    assert!(generic.snapshot_field.is_none());
}

#[test]
fn json_columns_are_fingerprinted_canonically() {
    let parsed = parse(parse_quote! {
        struct Profile {
            id: i32,
            #[sqlx(json)]
            tags: Vec<String>,
            settings: rullst_orm::Json<Settings>,
            extra: Option<Json<Settings>>,
            raw: serde_json::Value,
            alias: JsonValue,
            name: String,
            other: Value,
            maybe: Option<String>,
            #[sqlx(skip)]
            snapshot: RowSnapshot,
        }
    });
    let methods = generate_methods(&parsed, &quote! {}).to_string();
    for column in ["tags", "settings", "extra", "raw", "alias"] {
        assert!(
            methods.contains(&format!("fingerprint_json (& self . {column})")),
            "{column}: {methods}"
        );
    }
    for column in ["name", "other", "maybe"] {
        assert!(
            methods.contains(&format!("snapshot :: fingerprint (& self . {column})")),
            "{column}: {methods}"
        );
    }
}
