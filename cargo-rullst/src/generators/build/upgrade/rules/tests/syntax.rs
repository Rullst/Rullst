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
        "fn a(k: &FieldKind) -> u8 { match k { FieldKind::Text => 1, other => 0 } }",
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

#[test]
fn searchable_tables_must_be_lowercase_identifiers() {
    let model = |table: &str| {
        format!(
            "#[derive(Orm)]\n#[orm(searchable, table = \"{table}\")]\npub struct Post {{\n    pub id: i32,\n}}\n"
        )
    };
    for table in ["posts", "post_tags", "posts2"] {
        assert_eq!(found(&model(table)), [], "{table}");
    }
    for table in ["9posts", "pOsts", "Posts", "post-tags"] {
        assert_eq!(
            found(&model(table)),
            [("V13-ORM-SEARCHABLE-TABLE", 2)],
            "{table}"
        );
    }
}

#[test]
fn only_the_turso_backend_keeps_text_timestamps() {
    let model = |backend: &str| {
        format!(
            "#[derive(Orm)]\n#[orm(table = \"posts\", backend = \"{backend}\")]\npub struct Post {{\n    pub created_at: String,\n}}\n"
        )
    };
    assert_eq!(found(&model("turso")), []);
    assert_eq!(found(&model("postgres")), [("V13-TEXT-TIMESTAMPS", 4)]);
}

#[test]
fn relation_keys_that_the_relation_ignores_must_change() {
    let field = |options: &str| {
        format!(
            "#[derive(Orm)]\npub struct Post {{\n    #[orm({options})]\n    pub author: i64,\n}}\n"
        )
    };
    for options in [
        "belongs_to = \"User\", foreign_key = \"user_id\", local_key = \"id\"",
        "morph_to = \"owner\", local_key = \"id\"",
        "has_many = \"Comment\", related_key = \"post_id\"",
        "has_one = \"Profile\", related_key = \"user_id\"",
    ] {
        assert_eq!(
            found(&field(options)),
            [("V13-ORM-IGNORED-RELATION-KEY", 3)],
            "{options}"
        );
    }
    for options in [
        "belongs_to = \"User\", foreign_key = \"user_id\", related_key = \"id\"",
        "has_many = \"Comment\", local_key = \"id\"",
    ] {
        assert_eq!(found(&field(options)), [], "{options}");
    }
}

#[test]
fn a_nexus_model_annotates_one_primary_key() {
    let twice = "#[derive(Nexus)]\npub struct Product {\n    #[nexus(primary_key)]\n    pub id: i64,\n    #[nexus(primary_key)]\n    pub code: String,\n}\n";
    assert_eq!(found(twice), [("V13-NEXUS-PRIMARY-KEY", 5)]);
    let once = "#[derive(Nexus)]\npub struct Product {\n    #[nexus(primary_key)]\n    pub id: i64,\n    #[nexus(hidden)]\n    pub secret: String,\n    #[nexus(label = \"Code\")]\n    pub code: String,\n}\n";
    assert_eq!(found(once), []);
}
