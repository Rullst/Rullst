use super::{portable_subquery, raw_fragment, replace_placeholders};
use crate::RullstValue;

#[test]
fn raw_fragments_carry_exactly_their_own_bindings() {
    let sql = "SELECT id FROM notes WHERE body = ? AND tag <> '?'";
    assert!(raw_fragment(sql, vec![text("x")]).is_ok());
    assert!(raw_fragment(sql, Vec::new()).is_err());
    assert!(raw_fragment(sql, vec![text("x"), text("y")]).is_err());
    let (portable, ordered) =
        raw_fragment("SELECT $2, $1", vec![text("a"), text("b")]).expect("numbered");
    assert_eq!(portable, "SELECT ?, ?");
    assert_eq!(described(&ordered), described(&[text("b"), text("a")]));
}

fn text(value: &str) -> RullstValue {
    RullstValue::String(value.to_string())
}

fn described(values: &[RullstValue]) -> Vec<String> {
    values.iter().map(|value| format!("{value:?}")).collect()
}

#[test]
fn portable_subquery_restores_textual_marker_order() {
    let rendered = replace_placeholders(
        "SELECT * FROM comments WHERE (tenant_id = ?) AND (status = ?) AND body = '$9'",
    );
    let (sql, bindings) =
        portable_subquery(&rendered, vec![text("acme"), text("open")]).expect("portable");
    assert_eq!(
        sql,
        "SELECT * FROM comments WHERE (tenant_id = ?) AND (status = ?) AND body = '$9'"
    );
    assert_eq!(
        described(&bindings),
        described(&[text("acme"), text("open")])
    );

    let (sql, bindings) = portable_subquery(
        "SELECT 1 WHERE b = $2 AND a = $1 AND c = $2",
        vec![text("a"), text("b")],
    )
    .expect("custom numbering is reordered");
    assert_eq!(sql, "SELECT 1 WHERE b = ? AND a = ? AND c = ?");
    assert_eq!(
        described(&bindings),
        described(&[text("b"), text("a"), text("b")])
    );

    let (sql, bindings) =
        portable_subquery("SELECT 1 WHERE a = ?", vec![text("a")]).expect("already portable");
    assert_eq!(sql, "SELECT 1 WHERE a = ?");
    assert_eq!(described(&bindings), described(&[text("a")]));
}

#[test]
fn portable_subquery_rejects_ambiguous_numbering() {
    for (sql, bindings) in [
        (
            "SELECT 1 WHERE a = ? AND b = $1",
            vec![text("a"), text("b")],
        ),
        ("SELECT 1 WHERE a = $2", vec![text("a")]),
        ("SELECT 1 WHERE a = $0", vec![text("a")]),
        ("SELECT 1 WHERE a = $1", vec![text("a"), text("unused")]),
    ] {
        assert!(matches!(
            portable_subquery(sql, bindings),
            Err(crate::Error::Validation(_))
        ));
    }
}

#[test]
fn preserves_quoted_text_comments_and_dollar_quotes() {
    let sql = "SELECT '?', \"?\", `?`, $$?$$, $body$?$body$ /* ? /* ? */ */ FROM t -- ?\nWHERE id = ? AND name = ?";
    assert_eq!(
        replace_placeholders(sql),
        "SELECT '?', \"?\", `?`, $$?$$, $body$?$body$ /* ? /* ? */ */ FROM t -- ?\nWHERE id = $1 AND name = $2"
    );
}

#[test]
fn backslashes_escape_only_inside_escape_strings() {
    // Under standard_conforming_strings '\' is a complete literal.
    assert_eq!(
        replace_placeholders("name LIKE ? ESCAPE '\\' AND tenant_id = ?"),
        "name LIKE $1 ESCAPE '\\' AND tenant_id = $2"
    );
    assert_eq!(
        replace_placeholders("code = '\\' AND a = ? AND note = 'x\\y?'"),
        "code = '\\' AND a = $1 AND note = 'x\\y?'"
    );
    // E'...' strings honour backslash escapes, including an escaped quote.
    assert_eq!(
        replace_placeholders("SELECT E'it\\'s ?', e'\\\\' WHERE a = ?"),
        "SELECT E'it\\'s ?', e'\\\\' WHERE a = $1"
    );
    // An identifier ending in `e` does not start an escape string.
    assert_eq!(
        replace_placeholders("SELECT typee'\\' WHERE a = ?"),
        "SELECT typee'\\' WHERE a = $1"
    );
}

#[test]
fn preserves_json_operators_and_continues_existing_parameters() {
    let sql = "SELECT payload ? 'key', payload ?| array['a'], payload ?& array['b'], payload @? '$.a' FROM events WHERE tenant_id = $4 AND id = ?";
    assert_eq!(
        replace_placeholders(sql),
        "SELECT payload ? 'key', payload ?| array['a'], payload ?& array['b'], payload @? '$.a' FROM events WHERE tenant_id = $4 AND id = $5"
    );
}

#[test]
fn every_quoted_construct_hides_its_markers_wherever_it_starts() {
    for (sql, expected) in [
        ("-- a?b\nx = ?", "-- a?b\nx = $1"),
        ("\"a?b\" = ?", "\"a?b\" = $1"),
        ("x = \"a\"\"?b\" AND y = ?", "x = \"a\"\"?b\" AND y = $1"),
        ("`a?b` = ?", "`a?b` = $1"),
        ("x = `a``?b` AND y = ?", "x = `a``?b` AND y = $1"),
        ("'a?b' = ?", "'a?b' = $1"),
        ("x = 'a''?b' AND y = ?", "x = 'a''?b' AND y = $1"),
        ("/* a?b */ x = ?", "/* a?b */ x = $1"),
        (
            "x /* a /* b?c */ d?e */ = ?",
            "x /* a /* b?c */ d?e */ = $1",
        ),
        ("$tag$ a?b $tag$ = ?", "$tag$ a?b $tag$ = $1"),
        (
            "x = $a_b$ c?d $a_b$ AND y = ?",
            "x = $a_b$ c?d $a_b$ AND y = $1",
        ),
        (
            "x = $_t$ c?d $_t$ AND y = ?",
            "x = $_t$ c?d $_t$ AND y = $1",
        ),
        ("$$ a?b $$ = ?", "$$ a?b $$ = $1"),
        // Only the whole tag closes the string, not a prefix of it.
        (
            "x = $tag$ $ta, $tag, ?, $tag$ AND y = ?",
            "x = $tag$ $ta, $tag, ?, $tag$ AND y = $1",
        ),
        ("$ = ? AND b = ?", "$ = $1 AND b = $2"),
        ("$12 = x AND y = ?", "$12 = x AND y = $13"),
        ("E'a\\'?b' = ?", "E'a\\'?b' = $1"),
        // Empty literals close on their second quote.
        ("x = '' AND y = ?", "x = '' AND y = $1"),
        ("x = \"\" AND y = ?", "x = \"\" AND y = $1"),
        ("x = `` AND y = ?", "x = `` AND y = $1"),
    ] {
        assert_eq!(replace_placeholders(sql), expected, "{sql}");
    }
}

#[test]
fn only_a_standalone_e_prefix_opens_an_escape_string() {
    // After an identifier character the backslash is a complete literal.
    for prefix in ["x_e", "x$e", "xée"] {
        let sql = format!("SELECT {prefix}'\\' WHERE a = ?");
        assert_eq!(
            replace_placeholders(&sql),
            format!("SELECT {prefix}'\\' WHERE a = $1"),
            "{prefix}"
        );
    }
}
