use rullst_orm::{PersonalData, privacy::ComplianceModel};

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
    let report = UserData::compliance_schema();
    assert_eq!(report.table_name, "userdata");
    assert!(report.has_encrypted_data);
    assert_eq!(report.encrypted_fields.len(), 1);
    assert_eq!(report.encrypted_fields[0], "ssn");

    let report2 = PublicData::compliance_schema();
    assert_eq!(report2.table_name, "publicdata");
    assert!(!report2.has_encrypted_data);
    assert!(report2.encrypted_fields.is_empty());
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
