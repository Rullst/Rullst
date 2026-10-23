//! Batch form parsing and SQL for registered Nexus models.

use super::*;
use crate::nexus::types::FieldMeta;

fn entry(fields: Vec<FieldMeta>) -> RegistryEntry {
    RegistryEntry {
        table: "users",
        label: "Users",
        icon: "👥",
        pk: "id",
        pk_columns: Vec::new(),
        tenant_column: None,
        fields,
    }
}

#[test]
fn builds_parameterized_delete_for_each_sql_dialect() {
    let entry = entry(vec![]);
    assert_eq!(
        build_batch_sql(&entry, "delete", 2, "postgres", false).as_deref(),
        Some("DELETE FROM users WHERE id IN ($1,$2)")
    );
    assert_eq!(
        build_batch_sql(&entry, "delete", 2, "sqlite", false).as_deref(),
        Some("DELETE FROM users WHERE id IN (?,?)")
    );
}

#[test]
fn deactivate_requires_a_writable_conventional_boolean_field() {
    let active = entry(vec![FieldMeta::new(
        "is_active",
        "Active",
        FieldKind::Boolean,
    )]);
    assert!(supports_deactivation(&active));
    assert_eq!(
        build_batch_sql(&active, "deactivate", 1, "mysql", false).as_deref(),
        Some("UPDATE users SET is_active = FALSE WHERE id IN (?)")
    );
    assert_eq!(
        build_batch_sql(&active, "deactivate", 2, "postgres", false).as_deref(),
        Some("UPDATE users SET is_active = '0' WHERE id IN ($1,$2)")
    );

    for protected in [
        FieldMeta::new("is_active", "Active", FieldKind::Boolean).hidden(),
        FieldMeta::new("active", "Active", FieldKind::Boolean).readonly(),
    ] {
        let protected = entry(vec![protected]);
        assert!(!supports_deactivation(&protected));
        assert!(build_batch_sql(&protected, "deactivate", 1, "sqlite", false).is_none());
    }

    let absent = entry(vec![FieldMeta::new("status", "Status", FieldKind::Text)]);
    assert!(!supports_deactivation(&absent));
    assert!(build_batch_sql(&absent, "deactivate", 1, "sqlite", false).is_none());
    assert!(build_batch_sql(&active, "archive", 1, "sqlite", false).is_none());
}

#[test]
fn batch_size_is_bounded() {
    let entry = entry(vec![]);
    assert!(build_batch_sql(&entry, "delete", 0, "sqlite", false).is_none());
    assert!(build_batch_sql(&entry, "delete", MAX_BATCH_RECORDS + 1, "sqlite", false).is_none());
}

#[test]
fn tenant_scoped_batch_adds_one_bound_scope_predicate() {
    let mut entry = entry(vec![]);
    entry.tenant_column = Some("tenant_id");
    assert_eq!(
        build_batch_sql(&entry, "delete", 2, "postgres", true).as_deref(),
        Some("DELETE FROM users WHERE id IN ($1,$2) AND tenant_id = $3")
    );
    assert_eq!(
        build_batch_sql(&entry, "delete", 2, "sqlite", true).as_deref(),
        Some("DELETE FROM users WHERE id IN (?,?) AND tenant_id = ?")
    );
    assert_eq!(
        build_batch_sql(&entry, "delete", 1, "mysql", true).as_deref(),
        Some(
            "DELETE FROM users WHERE id IN (?) \
             AND CAST(tenant_id AS BINARY) = CAST(? AS BINARY)"
        )
    );
}

#[test]
fn composite_batches_match_each_key_and_bind_the_tenant_last() {
    let mut entry = entry(vec![FieldMeta::new(
        "is_active",
        "Active",
        FieldKind::Boolean,
    )]);
    entry.table = "memberships";
    entry.pk = "user_id";
    entry.pk_columns = vec!["user_id", "team_id"];
    entry.tenant_column = Some("tenant_id");
    assert_eq!(
        build_batch_sql(&entry, "delete", 2, "postgres", true).as_deref(),
        Some(
            "DELETE FROM memberships WHERE ((user_id = $1 AND team_id = $2) \
             OR (user_id = $3 AND team_id = $4)) AND tenant_id = $5"
        )
    );
    assert_eq!(
        build_batch_sql(&entry, "delete", 1, "postgres", true).as_deref(),
        Some(
            "DELETE FROM memberships WHERE ((user_id = $1 AND team_id = $2)) \
             AND tenant_id = $3"
        )
    );
    assert_eq!(
        build_batch_sql(&entry, "delete", 3, "postgres", true).as_deref(),
        Some(
            "DELETE FROM memberships WHERE ((user_id = $1 AND team_id = $2) \
             OR (user_id = $3 AND team_id = $4) OR (user_id = $5 AND team_id = $6)) \
             AND tenant_id = $7"
        )
    );
    assert_eq!(
        build_batch_sql(&entry, "deactivate", 1, "sqlite", false).as_deref(),
        Some(
            "UPDATE memberships SET is_active = FALSE \
             WHERE ((user_id = ? AND team_id = ?))"
        )
    );
}

#[test]
fn parses_repeated_html_checkbox_values_with_strict_bounds() {
    let form = parse_batch_form(
        b"q=active+users&action=deactivate&selected_ids=100&selected_ids=user%2Ftwo",
    )
    .expect("valid browser form");
    assert_eq!(form.action, "deactivate");
    assert_eq!(form.selected_ids, ["100", "user/two"]);

    assert!(parse_batch_form(b"action=delete&action=deactivate").is_err());
    assert!(parse_batch_form(b"action=delete&admin=true").is_err());
    assert!(parse_batch_form(&vec![b'x'; MAX_BATCH_FORM_BYTES + 1]).is_err());
}

#[test]
fn batch_forms_hold_at_most_64_kib() {
    let form = |len: usize| {
        let mut body = b"action=delete&q=".to_vec();
        body.resize(len, b'x');
        body
    };
    assert_eq!(form(65_536).len(), 65_536);
    assert_eq!(
        parse_batch_form(&form(65_536)).map(|form| form.action),
        Ok("delete".to_string())
    );
    assert_eq!(
        parse_batch_form(&form(65_537)).map(|form| form.action),
        Err("Batch form exceeds the 64 KiB limit")
    );
}

#[test]
fn batch_forms_reject_duplicate_actions_and_unbounded_ids() {
    let error = |body: &[u8]| parse_batch_form(body).map(|form| form.selected_ids);
    assert_eq!(
        error(b"action=delete&action=delete"),
        Err("Batch form contains duplicate actions")
    );
    let bounds = Err("Batch record IDs must contain between 1 and 256 bytes");
    assert_eq!(error(b"action=delete&selected_ids="), bounds);
    let longest = format!("action=delete&selected_ids={}", "k".repeat(256));
    assert_eq!(error(longest.as_bytes()), Ok(vec!["k".repeat(256)]));
    let too_long = format!("action=delete&selected_ids={}", "k".repeat(257));
    assert_eq!(error(too_long.as_bytes()), bounds);
    assert_eq!(error(b"selected_ids=1"), Err("Batch action is required"));
}

#[test]
fn a_batch_may_select_exactly_1000_records() {
    let entry = entry(vec![]);
    let sql = build_batch_sql(&entry, "delete", 1_000, "postgres", false)
        .expect("1000 records are allowed");
    assert!(sql.ends_with(",$1000)"), "{sql}");
    assert!(build_batch_sql(&entry, "delete", 1_001, "postgres", false).is_none());
}
