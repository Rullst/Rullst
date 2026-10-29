//! Edges of the syntax rules, one fixture per case: review rules report only
//! their first location in a file, so a combined fixture hides a case that
//! stopped matching.

use super::*;

fn found(source: &str) -> Vec<(&'static str, usize)> {
    scan_with(source, "lib.rs", &WorkspaceFacts::default())
}

#[test]
fn tuple_and_or_catch_all_arms_cover_new_field_kinds() {
    for source in [
        "fn a(k: &FieldKind, f: bool) -> u8 { match (k, f) { (FieldKind::Text, _) => 1, (_, _) => 0 } }",
        "fn a(k: &FieldKind) -> u8 { match k { FieldKind::Text => 1, FieldKind::Date | _ => 0 } }",
    ] {
        assert_eq!(found(source), [], "{source}");
    }
    assert_eq!(
        found(
            "fn a(k: &FieldKind, f: bool) -> u8 { match (k, f) { (FieldKind::Text, _) => 1, (_, true) => 0 } }"
        ),
        [("V13-NEXUS-FIELD-KIND-MATCH", 1)]
    );
}

#[test]
fn optional_text_timestamps_count_and_other_containers_do_not() {
    let model = |field: &str| {
        format!(
            "#[derive(Debug, FromRow, Orm)]\npub struct Post {{\n    pub id: i32,\n    pub {field},\n}}\n"
        )
    };
    assert_eq!(
        found(&model("updated_at: Option<String>")),
        [("V13-TEXT-TIMESTAMPS", 4)]
    );
    assert_eq!(found(&model("created_at: Vec<String>")), []);
}

#[test]
fn only_empty_string_timestamps_are_reviewed() {
    let user =
        |field: &str| format!("fn user() -> User {{ User {{ {field}, ..User::default() }} }}");
    for field in [
        "created_at: String::new()",
        "updated_at: String::from(\"\")",
        "created_at: \"\".to_string()",
        "updated_at: \"\".into()",
        "created_at: \"\".to_owned()",
    ] {
        assert_eq!(
            found(&user(field)),
            [("V13-EMPTY-TIMESTAMPS", 1)],
            "{field}"
        );
    }
    for field in [
        "id: String::new()",
        "created_at: String::from(\"2024-01-01\")",
        "created_at: Text::from(\"\")",
        "created_at: \"2024-01-01\".to_string()",
        "created_at: \"\".len()",
        "created_at: utc_timestamp()",
    ] {
        assert_eq!(found(&user(field)), [], "{field}");
    }
}

#[test]
fn credential_lengths_need_len_and_a_registration_limit() {
    for source in [
        "fn a(password: &str) -> bool { password.len() < 12 }",
        "fn a(name: &str) -> bool { 120 < name.len() }",
    ] {
        assert_eq!(found(source), [("V13-UTF16-LENGTHS", 1)], "{source}");
    }
    for source in [
        "fn a(password: &str) -> bool { password.chars().count() < 12 }",
        "fn a(password: &str) -> bool { password.len(1) < 12 }",
        "fn a(total: usize) -> bool { 12 > total }",
        "fn a(password: &str) -> bool { password.len() < 13 }",
    ] {
        assert_eq!(found(source), [], "{source}");
    }
}

#[test]
fn methods_in_impl_blocks_are_scanned() {
    let source = "\nstruct Pages;\nimpl Pages {\n    pub fn index_page(&self, posts: Vec<Post>) -> String {\n        String::new()\n    }\n}\n";
    assert_eq!(found(source), [("V13-PAGE-CSP-NONCE", 4)]);
}
