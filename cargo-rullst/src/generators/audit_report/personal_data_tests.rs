#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::generators::audit_report::tests::project;

const MODELS: &str = r#"
use rullst_orm::{Orm, PersonalData};

#[derive(Debug, Clone, rullst_orm::Orm, PersonalData)]
#[orm(table = "customers")]
pub struct Customer {
    pub id: i64,
    #[privacy]
    pub full_name: String,
    #[privacy(sensitive)]
    #[orm(encrypted)]
    pub tax_id: String,
    #[orm(encrypted)]
    pub notes: String,
    #[orm(masked)]
    pub phone_number: String,
    pub api_token: SecretString,
    #[orm(skip)]
    pub cached_secret: Option<SecretString>,
    pub email: String,
    pub client_ip: String,
    pub zip: String,
    pub birthdate: String,
}

pub mod nested {
    #[derive(Orm)]
    pub struct Address { pub address_line: String }
}

/// Not an ORM model: names alone are not flagged.
pub struct ContactForm { pub email: String }

#[cfg(test)]
mod tests {
    #[derive(Orm)]
    struct Fixture { email: String }
}
"#;

fn row(data: &PersonalData, field: &str) -> Option<(String, bool)> {
    data.fields
        .iter()
        .find(|row| row.field == field)
        .map(|row| (row.classification.clone(), row.encrypted))
}

#[test]
fn fields_are_classified_from_the_real_orm_attributes() {
    let root = project(&[("src/models.rs", MODELS)]);
    let sources = ProjectSources::load(root.path());
    let data = inventory(&sources);
    let personal = |classification: &str, encrypted| Some((classification.to_string(), encrypted));
    assert_eq!(row(&data, "full_name"), personal("personal", false));
    assert_eq!(row(&data, "tax_id"), personal("personal", true));
    assert_eq!(row(&data, "notes"), personal("encrypted", true));
    assert_eq!(row(&data, "phone_number"), personal("masked", false));
    assert_eq!(row(&data, "api_token"), personal("encrypted", true));
    // `#[orm(skip)]` keeps the column out of the table.
    assert_eq!(row(&data, "cached_secret"), None);
    assert_eq!(row(&data, "email"), personal("review", false));
    assert_eq!(row(&data, "client_ip"), personal("review", false));
    assert_eq!(row(&data, "birthdate"), personal("review", false));
    assert_eq!(row(&data, "address_line"), personal("review", false));
    assert_eq!(row(&data, "zip"), None);
    assert_eq!(row(&data, "id"), None);
    // Only the Customer and Address rows; ContactForm and the test model are skipped.
    assert_eq!(data.fields.len(), 9, "{:?}", data.fields);
    assert_eq!(data.review_count(), 4);
    assert_eq!(data.status, EvidenceStatus::Observed(9));
    assert!(data.detail.contains("field-name heuristic"));
    let email = data.fields.iter().find(|row| row.field == "email").unwrap();
    assert_eq!(
        (email.model.as_str(), email.file.as_str()),
        ("Customer", "src/models.rs")
    );
    assert_eq!(email.line, 20);
}

#[test]
fn the_name_heuristic_matches_whole_segments_or_prefixes() {
    for name in [
        "email",
        "emails",
        "phone",
        "cpf",
        "cnpj",
        "ssn",
        "birth_date",
        "home_address",
        "document_number",
        "ip",
        "last_login_ip",
    ] {
        assert!(likely_personal(name), "{name}");
    }
    for name in ["zip", "ship_to", "title", "description", "tipo", "id"] {
        assert!(!likely_personal(name), "{name}");
    }
}

#[test]
fn unparsable_files_are_reported_and_missing_sources_are_not_checked() {
    let root = project(&[("src/broken.rs", "pub struct {")]);
    let data = inventory(&ProjectSources::load(root.path()));
    assert!(data.detail.contains("1 source file(s) could not be parsed"));
    let empty = project(&[]);
    let data = inventory(&ProjectSources::load(empty.path()));
    assert!(matches!(data.status, EvidenceStatus::NotChecked(_)));
}

const MARKERS: &str = r#"
#[derive(rullst_orm::Orm)]
pub struct Vault {
    pub backup_token: Option<SecretString>,
    pub recovery_codes: Vec<SecretString>,
    #[sqlx(encrypted)]
    pub sqlx_note: String,
    #[sqlx(masked)]
    pub sqlx_hint: String,
    pub email: String,
    pub phone: String,
}
"#;

#[test]
fn only_secret_options_and_orm_markers_count_as_protected() {
    let root = project(&[("src/vault.rs", MARKERS)]);
    let data = inventory(&ProjectSources::load(root.path()));
    assert_eq!(
        row(&data, "backup_token"),
        Some(("encrypted".to_string(), true))
    );
    // Another wrapper, and `encrypted`/`masked` under `sqlx`, are not markers.
    assert_eq!(row(&data, "recovery_codes"), None);
    assert_eq!(row(&data, "sqlx_note"), None);
    assert_eq!(row(&data, "sqlx_hint"), None);
    assert_eq!(row(&data, "email"), Some(("review".to_string(), false)));
    assert!(
        data.detail
            .starts_with("3 classified or flagged field(s); 2 need review."),
        "{}",
        data.detail
    );
}
