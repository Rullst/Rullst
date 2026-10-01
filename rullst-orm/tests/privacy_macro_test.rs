use rullst_orm::{PersonalData, SecretString, privacy::ComplianceModel};

#[derive(PersonalData, Default)]
pub struct UserData {
    pub id: i32,
    #[privacy]
    pub ssn: String,
    pub name: String,
}

#[derive(PersonalData, Default)]
pub struct PublicData {
    pub id: i32,
    pub name: String,
}

#[test]
fn test_personal_data_compliance_schema() {
    // `#[privacy]` declares personal data and redacts Debug; it does not
    // encrypt, and the table is the one the ORM derive would use.
    let report = UserData::compliance_schema();
    assert_eq!(report.table_name, "userdatas");
    assert!(!report.has_encrypted_data);
    assert!(report.encrypted_fields.is_empty());
    assert_eq!(UserData::personal_fields(), vec!["ssn"]);

    let report2 = PublicData::compliance_schema();
    assert_eq!(report2.table_name, "publicdatas");
    assert!(!report2.has_encrypted_data);
    assert!(report2.encrypted_fields.is_empty());
    assert!(PublicData::personal_fields().is_empty());
}

/// A persisted ORM model: its encrypted columns and explicit table.
#[derive(PersonalData, Clone, rullst_orm::FromRow, rullst_orm::Orm)]
#[orm(table = "patients")]
pub struct Patient {
    pub id: i32,
    #[privacy]
    #[orm(encrypted)]
    pub diagnosis: String,
    #[privacy]
    pub name: String,
    pub api_secret: SecretString,
}

#[test]
fn compliance_schema_reports_encryption_at_rest() {
    let report = Patient::compliance_schema();
    assert_eq!(report.table_name, "patients");
    assert!(report.has_encrypted_data);
    assert_eq!(report.encrypted_fields, vec!["diagnosis", "api_secret"]);
    assert_eq!(Patient::personal_fields(), vec!["diagnosis", "name"]);
    let patient = Patient {
        id: 1,
        diagnosis: "private".to_string(),
        name: "Ana".to_string(),
        api_secret: SecretString::new("not printed"),
    };
    assert_eq!(patient.id, 1);
}

#[test]
fn test_personal_data_debug() {
    let user = UserData {
        id: 1,
        ssn: "123-45-678".to_string(),
        name: "Alice".to_string(),
    };

    let debug_str = format!("{:?}", user);
    assert!(debug_str.contains("id: 1"));
    assert!(debug_str.contains("name: \"Alice\""));
    assert!(debug_str.contains("ssn: \"[REDACTED_BY_RULLST_SHIELD]\""));
    assert!(!debug_str.contains("123-45-678"));

    let pub_data = PublicData {
        id: 2,
        name: "Bob".to_string(),
    };

    let debug_str_pub = format!("{:?}", pub_data);
    assert!(debug_str_pub.contains("id: 2"));
    assert!(debug_str_pub.contains("name: \"Bob\""));
}
