use syn::{DeriveInput, parse_quote};

fn save_doc(input: DeriveInput) -> String {
    let parsed = crate::parser::parse(&input).expect("parse model");
    let generated = super::generate(&parsed).to_string();
    let (before_save, _) = generated
        .split_once("pub async fn save (& mut self)")
        .expect("save()");
    before_save
        .rsplit_once("# [doc =")
        .expect("save() rustdoc")
        .1
        .to_string()
}

#[test]
fn save_documents_the_full_row_write_on_every_untracked_model() {
    for input in [
        parse_quote! { struct Lesson { id: i32, title: String } },
        parse_quote! {
            #[orm(table = "lessons", auditable)]
            struct Lesson { id: i32, title: String }
        },
    ] {
        let doc = save_doc(input);
        assert!(doc.contains("the last writer wins"), "{doc}");
        assert!(doc.contains("`rullst_orm::RowSnapshot`"), "{doc}");
        assert!(doc.contains("use `update_partial()`"), "{doc}");
        assert!(
            doc.contains("UPDATE users SET xp = xp + ? WHERE id = ?"),
            "{doc}"
        );
        assert!(
            doc.contains("database column defaults do not apply"),
            "{doc}"
        );
        assert!(doc.contains("marked `#[orm(db_default)]`"), "{doc}");
    }
}

#[test]
fn save_documents_the_changed_column_write_on_every_tracked_model() {
    for input in [
        parse_quote! {
            struct Lesson { id: i32, title: String, #[sqlx(skip)] snapshot: RowSnapshot }
        },
        parse_quote! {
            #[orm(table = "lessons", auditable)]
            struct Lesson { id: i32, title: String, #[sqlx(skip)] snapshot: RowSnapshot }
        },
    ] {
        let doc = save_doc(input);
        assert!(
            doc.contains("updates the columns this handle changed"),
            "{doc}"
        );
        assert!(doc.contains("see `dirty_columns()`"), "{doc}");
        assert!(
            doc.contains("`mark_all_dirty()`, writes every column"),
            "{doc}"
        );
        assert!(doc.contains("the last writer wins"), "{doc}");
        assert!(
            doc.contains("UPDATE users SET xp = xp + ? WHERE id = ?"),
            "{doc}"
        );
        assert!(doc.contains("marked `#[orm(db_default)]`"), "{doc}");
        assert!(!doc.contains("writes every persisted column"), "{doc}");
    }
}

#[test]
fn tracked_saves_skip_unchanged_records_and_restore_the_snapshot_on_failure() {
    for input in [
        parse_quote! {
            struct Lesson { id: i32, title: String, #[sqlx(skip)] snapshot: RowSnapshot }
        },
        parse_quote! {
            #[orm(table = "lessons", auditable)]
            struct Lesson { id: i32, title: String, #[sqlx(skip)] snapshot: RowSnapshot }
        },
    ] {
        let input: DeriveInput = input;
        let parsed = crate::parser::parse(&input).expect("parse model");
        let generated = super::generate(&parsed).to_string();
        for entrypoint in [
            "pub async fn save (& mut self)",
            "pub async fn save_with_tx",
        ] {
            let body = generated.split_once(entrypoint).expect("entrypoint").1;
            let prologue = body.split_once("original_id").expect("original id").0;
            assert!(
                prologue.contains("if self . __rullst_save_is_noop () ? { return Ok (()) ; }"),
                "{entrypoint}: {prologue}"
            );
        }
        assert_eq!(
            generated
                .matches("let original_snapshot = self . snapshot . clone () ;")
                .count(),
            2
        );
        assert_eq!(
            generated
                .matches("self . id = original_id ; self . snapshot = original_snapshot ;")
                .count(),
            4
        );
    }
    let untracked: DeriveInput = parse_quote! { struct Lesson { id: i32, title: String } };
    let parsed = crate::parser::parse(&untracked).expect("parse model");
    let generated = super::generate(&parsed).to_string();
    assert!(!generated.contains("snapshot"), "{generated}");
    assert!(!generated.contains("__rullst_save_is_noop"));
}

#[test]
fn an_auditable_save_audits_only_a_write() {
    let input: DeriveInput = parse_quote! {
        #[orm(table = "lessons", auditable)]
        struct Lesson { id: i32, title: String }
    };
    let parsed = crate::parser::parse(&input).expect("parse model");
    let generated = super::generate(&parsed).to_string();
    let write = generated
        .split_once("if self . save_with_tx_internal (& mut * * tx) . await ? {")
        .expect("conditional audit")
        .1;
    assert!(
        write
            .trim_start()
            .starts_with("if is_new { rullst_orm :: audit :: log_audit_with_tx")
    );
    assert!(!generated.contains(". map (| _ | ())"));
    let plain: DeriveInput = parse_quote! { struct Lesson { id: i32, title: String } };
    let parsed = crate::parser::parse(&plain).expect("parse model");
    let generated = super::generate(&parsed).to_string();
    assert!(
        generated.contains("self . save_with_tx_internal (& mut * * tx) . await . map (| _ | ())")
    );
}

#[test]
fn auditable_save_locks_its_pre_image_before_the_update() {
    let input: DeriveInput = parse_quote! {
        #[orm(table = "lessons", auditable)]
        struct Lesson { id: i32, title: String }
    };
    let parsed = crate::parser::parse(&input).expect("parse model");
    let generated = super::generate(&parsed).to_string();
    let save = generated
        .split_once("pub async fn save_with_tx")
        .expect("save_with_tx")
        .1;
    let before_write = save
        .split_once("save_with_tx_internal")
        .expect("full-row write")
        .0;
    assert!(before_write.contains("WHERE id = ? FOR UPDATE"));
    assert!(before_write.contains("get_query_timeout"));
    let restore = generated
        .split_once("pub async fn restore_revision_with_tx")
        .expect("restore_revision_with_tx")
        .1;
    assert!(restore.contains("WHERE id = ? FOR UPDATE"));
    assert!(restore.contains("get_query_timeout"));
}
