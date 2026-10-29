use syn::{DeriveInput, parse_quote};

fn generated(input: DeriveInput) -> String {
    let parsed = crate::parser::parse(&input).expect("parse model");
    super::generate(&parsed).to_string()
}

#[test]
fn restore_reads_the_restored_row_under_the_query_timeout() {
    let generated = generated(parse_quote! {
        struct Note { id: i32, deleted_at: Option<String> }
    });
    let reread = generated
        .split_once("let restored_row")
        .expect("restored row lookup")
        .1
        .split_once("let restored =")
        .expect("restored row binding")
        .0;
    assert!(reread.contains("get_query_timeout"));
}

#[test]
fn force_delete_and_restore_run_policies_hooks_observers_and_effects() {
    let generated = generated(parse_quote! {
        #[orm(table = "notes", policy = "NotePolicy", before_delete = "before_hook", searchable)]
        struct Note { id: i32, deleted_at: Option<String> }
    });
    let (force_delete, restore) = generated
        .split_once("pub async fn restore (& self)")
        .expect("restore()");
    for expected in [
        "< NotePolicy as rullst_orm :: Policy < Self >> :: can_force_delete (self)",
        "\"Policy prevents force deleting this record\"",
        "rullst_orm :: __transaction_access :: run (self . before_hook ()) . await ?",
        "let observers = { let list = Self :: observers () . read ()",
        "observers . iter () . map (| obs | obs . deleting (& * self))",
        "let scoped_transaction = rullst_orm :: CURRENT_TX . try_with",
        "return self . __rullst_force_delete_with_tx (tx) . await ;",
        "\"DELETE FROM notes WHERE id = ?\"",
        "engine . delete (event . table , event . id)",
    ] {
        assert!(
            force_delete.contains(expected),
            "{expected}: {force_delete}"
        );
    }
    for expected in [
        "< NotePolicy as rullst_orm :: Policy < Self >> :: can_restore (self)",
        "\"Policy prevents restoring this record\"",
        "return self . __rullst_restore_with_tx (tx) . await ;",
        "\"orm:events:{}:updated\"",
        "\"orm:events:{}:saved\"",
        "observers . iter () . map (| observer | observer . committed (& event))",
        "engine . update (event . table , event . id , payload)",
    ] {
        assert!(restore.contains(expected), "{expected}: {restore}");
    }
}

#[test]
fn restore_clears_a_marker_whose_live_value_is_unset() {
    let generated = generated(parse_quote! {
        #[orm(table = "notes", soft_delete(column = "archived_at"))]
        struct Note { id: i32, archived_at: Option<String> }
    });
    assert!(
        generated.contains(
            "\"UPDATE notes SET archived_at = NULL WHERE id = ? AND archived_at IS NOT NULL\""
        ),
        "{generated}"
    );
}
