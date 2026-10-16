#![allow(clippy::expect_used)]

use super::*;

#[test]
fn foreign_key_actions_render_their_sql_keywords() {
    assert_eq!(ForeignKeyAction::Cascade.sql(), "CASCADE");
    assert_eq!(ForeignKeyAction::Restrict.sql(), "RESTRICT");
    assert_eq!(ForeignKeyAction::SetNull.sql(), "SET NULL");
    assert_eq!(ForeignKeyAction::NoAction.sql(), "NO ACTION");
}

#[test]
fn constraint_names_are_portable_identifiers_of_at_most_63_bytes() {
    assert!(validate_constraint_name("users_email_unique").is_ok());
    assert!(validate_constraint_name(&"n".repeat(63)).is_ok());
    assert!(matches!(
        validate_constraint_name(&"n".repeat(64)),
        Err(Error::Validation(message)) if message.contains(".name(")
    ));
    assert!(validate_constraint_name("users.email").is_err());
    assert!(validate_constraint_name("users email").is_err());
    assert!(validate_constraint_name("").is_err());
}

#[test]
fn check_expressions_accept_ordinary_predicates() {
    for expression in [
        "price >= 0",
        "status IN ('draft', 'published')",
        "(starts_on IS NULL) OR (ends_on > starts_on)",
        "note <> 'it''s'",
        "\"position\" BETWEEN 1 AND 10",
        "length(code) = 6",
    ] {
        assert!(
            validate_check_expression(expression).is_ok(),
            "{expression}"
        );
    }
    assert!(validate_check_expression(&"x".repeat(MAX_CHECK_EXPRESSION_BYTES)).is_ok());
}

#[test]
fn check_expressions_reject_text_that_could_leave_the_clause() {
    for expression in [
        "",
        "   ",
        "price > 0); DROP TABLE users; --",
        "price > 0) OR (1 = 1",
        "price > 0 -- comment",
        "price > 0 /* comment */",
        "price */ 0",
        "name = 'unterminated",
        "name = \"unterminated",
        "(price > 0",
        "price > 0)",
        "a = 'x\\'",
        "a = $$x$$",
        "a = `b`",
        "a = 1\n",
        "a = 1\0",
    ] {
        assert!(
            matches!(
                validate_check_expression(expression),
                Err(Error::Validation(_))
            ),
            "{expression:?}"
        );
    }
    assert!(validate_check_expression(&"x".repeat(MAX_CHECK_EXPRESSION_BYTES + 1)).is_err());
    // A parenthesis inside a quoted literal does not count.
    assert!(validate_check_expression("note <> ')'").is_ok());
    assert!(validate_check_expression("note <> '('").is_ok());
}

#[test]
fn checks_render_with_or_without_a_name() {
    let mut check = Check::new("price >= 0".to_string());
    assert_eq!(check.sql().expect("unnamed check"), "CHECK (price >= 0)");
    check.name("items_price_check");
    assert_eq!(
        check.sql().expect("named check"),
        "CONSTRAINT items_price_check CHECK (price >= 0)"
    );
    check.name("bad name");
    assert!(check.sql().is_err());
}

#[test]
fn indexes_need_valid_columns_and_name_their_kind() {
    let index = Index::new(&["user_id", "position"], IndexKind::Plain);
    assert_eq!(index.columns_sql().expect("columns"), "user_id, position");
    assert_eq!(index.default_name("posts"), "posts_user_id_position_index");
    assert!(!index.is_unique());
    let unique = Index::new(&["email"], IndexKind::Unique);
    assert_eq!(unique.default_name("users"), "users_email_unique");
    assert!(unique.is_unique());
    assert!(Index::new(&["email"], IndexKind::UniqueConstraint).is_unique());
    assert!(Index::new(&[], IndexKind::Plain).columns_sql().is_err());
    assert!(
        Index::new(&["users.email"], IndexKind::Plain)
            .columns_sql()
            .is_err()
    );
    assert!(
        Index::new(&["email); DROP TABLE users"], IndexKind::Plain)
            .columns_sql()
            .is_err()
    );
}

#[test]
fn foreign_keys_need_a_valid_reference() {
    let mut foreign = ForeignKey::new("user_id".to_string());
    assert!(matches!(
        foreign.body_sql("sqlite"),
        Err(Error::Validation(message)) if message.contains("references(table, column)")
    ));
    assert!(foreign.column_reference_sql("sqlite").is_err());
    foreign.references("users", "id");
    assert_eq!(
        foreign.body_sql("postgres").expect("body"),
        "FOREIGN KEY (user_id) REFERENCES \"users\" (id)"
    );
    foreign
        .on_delete(ForeignKeyAction::Cascade)
        .on_update(ForeignKeyAction::NoAction);
    assert_eq!(
        foreign.body_sql("mysql").expect("body"),
        "FOREIGN KEY (user_id) REFERENCES `users` (id) ON DELETE CASCADE ON UPDATE NO ACTION"
    );
    assert_eq!(
        foreign.column_reference_sql("sqlite").expect("reference"),
        "REFERENCES \"users\" (id) ON DELETE CASCADE ON UPDATE NO ACTION"
    );
    assert_eq!(foreign.default_name("posts"), "posts_user_id_foreign");

    for (column, table, referenced) in [
        ("user id", "users", "id"),
        ("user_id", "users; DROP TABLE x", "id"),
        ("user_id", "public.users", "id"),
        ("user_id", "users", "id)"),
    ] {
        let mut foreign = ForeignKey::new(column.to_string());
        foreign.references(table, referenced);
        assert!(
            foreign.body_sql("sqlite").is_err(),
            "{column} {table} {referenced}"
        );
    }
    let mut foreign = ForeignKey::new("user_id".to_string());
    foreign.references("users", "id)");
    assert!(foreign.column_reference_sql("sqlite").is_err());
}
