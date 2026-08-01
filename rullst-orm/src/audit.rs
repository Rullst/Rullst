use crate::Orm;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i32,
    pub model_type: String,
    pub model_id: i32,
    pub event: String,
    pub old_values: Option<String>,
    pub new_values: Option<String>,
    pub created_at: Option<String>,
}

#[cfg_attr(test, mutants::skip)]
fn validate_and_prepare_payloads(
    model_type: &str,
    event: &str,
    mut old_values: Option<String>,
    mut new_values: Option<String>,
) -> Result<(Option<String>, Option<String>), crate::Error> {
    const MAX_PAYLOAD_LEN: usize = 5 * 1024 * 1024; // 5 MB

    if model_type.len() > 255 || event.len() > 50 {
        return Err(crate::Error::Validation(
            "Audit model_type or event string too long".to_string(),
        ));
    }

    if let Some(val) = &old_values
        && val.len() > MAX_PAYLOAD_LEN
    {
        old_values = Some(r#"{"error":"payload_too_large"}"#.to_string());
    }

    if let Some(val) = &new_values
        && val.len() > MAX_PAYLOAD_LEN
    {
        new_values = Some(r#"{"error":"payload_too_large"}"#.to_string());
    }

    Ok((old_values, new_values))
}

#[cfg_attr(test, mutants::skip)]
pub async fn log_audit(
    model_type: &str,
    model_id: i32,
    event: &str,
    old_values: Option<String>,
    new_values: Option<String>,
) -> Result<(), crate::Error> {
    let (old_values, new_values) =
        validate_and_prepare_payloads(model_type, event, old_values, new_values)?;

    let pool = Orm::pool();
    let driver = Orm::driver();

    if driver == "postgres" {
        sqlx::query(
            "INSERT INTO rullst_audits (model_type, model_id, event, old_values, new_values) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(model_type)
        .bind(model_id)
        .bind(event)
        .bind(old_values)
        .bind(new_values)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO rullst_audits (model_type, model_id, event, old_values, new_values) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(model_type)
        .bind(model_id)
        .bind(event)
        .bind(old_values)
        .bind(new_values)
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn is_sensitive(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    k.contains("password")
        || k.contains("token")
        || k.contains("secret")
        || k.contains("senha")
        || k.contains("api_key")
        || k.contains("cvv")
        || k.contains("ssn")
        || k.contains("credit_card")
        || k.contains("auth_code")
}

fn mask_if_sensitive(key: &str, value: serde_json::Value) -> serde_json::Value {
    if is_sensitive(key) {
        serde_json::Value::String("***".to_string())
    } else {
        value
    }
}

#[cfg_attr(test, mutants::skip)]
pub fn compute_diff(old_json: &str, new_json: &str) -> (Option<String>, Option<String>) {
    if old_json == new_json {
        return (None, None);
    }

    let old_val: serde_json::Value =
        serde_json::from_str(old_json).unwrap_or(serde_json::Value::Null);
    let new_val: serde_json::Value =
        serde_json::from_str(new_json).unwrap_or(serde_json::Value::Null);

    let mut diff_old = serde_json::Map::new();
    let mut diff_new = serde_json::Map::new();

    if let (serde_json::Value::Object(old_obj), serde_json::Value::Object(new_obj)) =
        (old_val, new_val)
    {
        for (k, v) in &old_obj {
            if let Some(new_v) = new_obj.get(k) {
                #[allow(clippy::collapsible_if)]
                if v != new_v {
                    let masked_v = mask_if_sensitive(k, v.clone());
                    let masked_new_v = mask_if_sensitive(k, new_v.clone());
                    diff_new.insert(k.clone(), masked_new_v);
                    diff_old.insert(k.clone(), masked_v);
                }
            } else {
                let masked_v = mask_if_sensitive(k, v.clone());
                diff_new.insert(k.clone(), serde_json::Value::Null);
                diff_old.insert(k.clone(), masked_v);
            }
        }
        for (k, new_v) in &new_obj {
            if !old_obj.contains_key(k) {
                let masked_new_v = mask_if_sensitive(k, new_v.clone());
                diff_old.insert(k.clone(), serde_json::Value::Null);
                diff_new.insert(k.clone(), masked_new_v);
            }
        }
    }

    if diff_old.is_empty() && diff_new.is_empty() {
        return (None, None); // Nothing changed
    }

    let final_old = serde_json::to_string(&diff_old).ok();
    let final_new = serde_json::to_string(&diff_new).ok();

    (final_old, final_new)
}

#[cfg_attr(test, mutants::skip)]
pub async fn log_audit_diff(
    model_type: &str,
    model_id: i32,
    event: &str,
    old_json: &str,
    new_json: &str,
) -> Result<(), crate::Error> {
    const MAX_PAYLOAD_LEN: usize = 5 * 1024 * 1024; // 5 MB

    if old_json.len() > MAX_PAYLOAD_LEN || new_json.len() > MAX_PAYLOAD_LEN {
        return log_audit(
            model_type,
            model_id,
            event,
            Some(r#"{"error":"payload_too_large_for_diff"}"#.to_string()),
            Some(r#"{"error":"payload_too_large_for_diff"}"#.to_string()),
        )
        .await;
    }

    let (final_old, final_new) = compute_diff(old_json, new_json);
    if final_old.is_none() && final_new.is_none() {
        return Ok(()); // Nothing changed
    }
    log_audit(model_type, model_id, event, final_old, final_new).await
}

#[cfg_attr(test, mutants::skip)]
pub async fn create_audit_table() -> Result<(), crate::Error> {
    let pool = Orm::pool();
    let driver = Orm::driver();

    let query = if driver == "postgres" {
        r#"
        CREATE TABLE IF NOT EXISTS rullst_audits (
            id SERIAL PRIMARY KEY,
            model_type VARCHAR(255) NOT NULL,
            model_id INT NOT NULL,
            event VARCHAR(50) NOT NULL,
            old_values TEXT,
            new_values TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#
    } else if driver == "mysql" {
        r#"
        CREATE TABLE IF NOT EXISTS rullst_audits (
            id INT AUTO_INCREMENT PRIMARY KEY,
            model_type VARCHAR(255) NOT NULL,
            model_id INT NOT NULL,
            event VARCHAR(50) NOT NULL,
            old_values TEXT,
            new_values TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#
    } else {
        r#"
        CREATE TABLE IF NOT EXISTS rullst_audits (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            model_type TEXT NOT NULL,
            model_id INTEGER NOT NULL,
            event TEXT NOT NULL,
            old_values TEXT,
            new_values TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#
    };

    sqlx::query(query).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::AuditLog;

    #[test]
    fn test_audit_log_serialization_round_trip() {
        let log = AuditLog {
            id: 1,
            model_type: "User".to_string(),
            model_id: 42,
            event: "created".to_string(),
            old_values: None,
            new_values: Some(r#"{"name":"Alice"}"#.to_string()),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
        };

        let json_str = serde_json::to_string(&log).expect("serialize");
        assert!(json_str.contains("\"model_type\":\"User\""));
        assert!(json_str.contains("\"event\":\"created\""));

        let deserialized: AuditLog = serde_json::from_str(&json_str).expect("deserialize");
        assert_eq!(deserialized.id, 1);
        assert_eq!(deserialized.model_id, 42);
        assert_eq!(deserialized.event, "created");
        assert!(deserialized.old_values.is_none());
    }

    #[test]
    fn test_audit_log_clone_debug() {
        let log = AuditLog {
            id: 5,
            model_type: "Post".to_string(),
            model_id: 99,
            event: "updated".to_string(),
            old_values: Some(r#"{"title":"Old"}"#.to_string()),
            new_values: Some(r#"{"title":"New"}"#.to_string()),
            created_at: None,
        };
        let cloned = log.clone();
        assert_eq!(cloned.model_type, "Post");
        // Debug must not panic
        let _ = format!("{:?}", cloned);
    }

    #[test]
    fn test_compute_diff_changes() {
        let old_json = r#"{"name":"Alice","age":30}"#;
        let new_json = r#"{"name":"Alice","age":31}"#;
        let (old_diff, new_diff) = super::compute_diff(old_json, new_json);
        assert_eq!(old_diff.unwrap(), r#"{"age":30}"#);
        assert_eq!(new_diff.unwrap(), r#"{"age":31}"#);
    }

    #[test]
    fn test_compute_diff_no_changes() {
        let json = r#"{"name":"Alice","age":30}"#;
        let (old_diff, new_diff) = super::compute_diff(json, json);
        assert!(old_diff.is_none());
        assert!(new_diff.is_none());
    }

    #[test]
    fn test_compute_diff_invalid_json() {
        let (old_diff, new_diff) = super::compute_diff("not json", "{invalid}");
        assert!(old_diff.is_none());
        assert!(new_diff.is_none());

        // One valid, one invalid
        let (old_diff2, new_diff2) = super::compute_diff(r#"{"a":1}"#, "invalid");
        assert!(old_diff2.is_none());
        assert!(new_diff2.is_none());

        // Arrays or primitive values instead of JSON Objects
        let (old_diff3, new_diff3) = super::compute_diff("[1, 2]", "[1, 2, 3]");
        assert!(old_diff3.is_none());
        assert!(new_diff3.is_none());

        // Empty objects
        let (old_diff4, new_diff4) = super::compute_diff("{}", "{}");
        assert!(old_diff4.is_none());
        assert!(new_diff4.is_none());
    }

    #[tokio::test]
    async fn test_log_audit_diff_bypass() {
        // Should not panic or hit the database if the old and new JSONs are identical
        let result = super::log_audit_diff(
            "User",
            1,
            "update",
            r#"{"name":"Alice"}"#,
            r#"{"name":"Alice"}"#,
        )
        .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_compute_diff_explicit_null_vs_omitted() {
        let old_json = r#"{"name":"Alice","age":30}"#;
        let new_json = r#"{"name":"Alice"}"#;
        let (old_diff, new_diff) = super::compute_diff(old_json, new_json);
        assert_eq!(old_diff.unwrap(), r#"{"age":30}"#);
        assert_eq!(new_diff.unwrap(), r#"{"age":null}"#);

        let old_json2 = r#"{"name":"Alice"}"#;
        let new_json2 = r#"{"name":"Alice","age":null}"#;
        let (old_diff2, new_diff2) = super::compute_diff(old_json2, new_json2);
        assert_eq!(old_diff2.unwrap(), r#"{"age":null}"#);
        assert_eq!(new_diff2.unwrap(), r#"{"age":null}"#);

        let old_json3 = r#"{"name":"Alice"}"#;
        let new_json3 = r#"{"name":"Alice","age":30}"#;
        let (old_diff3, new_diff3) = super::compute_diff(old_json3, new_json3);
        assert_eq!(old_diff3.unwrap(), r#"{"age":null}"#);
        assert_eq!(new_diff3.unwrap(), r#"{"age":30}"#);
    }

    #[test]
    fn test_validate_and_prepare_payloads() {
        // Normal case
        let res = super::validate_and_prepare_payloads(
            "User",
            "create",
            Some("old".to_string()),
            Some("new".to_string()),
        );
        assert!(res.is_ok());

        // Model type too long
        let long_model = "A".repeat(256);
        let res = super::validate_and_prepare_payloads(&long_model, "create", None, None);
        assert!(res.is_err());

        // Event too long
        let long_event = "A".repeat(51);
        let res = super::validate_and_prepare_payloads("User", &long_event, None, None);
        assert!(res.is_err());

        // Payload too large
        let large_payload = Some("A".repeat(5 * 1024 * 1024 + 1));
        let (old_val, new_val) = super::validate_and_prepare_payloads(
            "User",
            "create",
            large_payload.clone(),
            large_payload,
        )
        .unwrap();
        assert_eq!(old_val.unwrap(), r#"{"error":"payload_too_large"}"#);
        assert_eq!(new_val.unwrap(), r#"{"error":"payload_too_large"}"#);
    }

    #[test]
    fn test_is_sensitive() {
        assert!(super::is_sensitive("password"));
        assert!(super::is_sensitive("PASSWORD"));
        assert!(super::is_sensitive("user_token"));
        assert!(super::is_sensitive("client_secret"));
        assert!(super::is_sensitive("senha"));
        assert!(super::is_sensitive("api_key"));
        assert!(super::is_sensitive("card_cvv"));
        assert!(super::is_sensitive("ssn_number"));
        assert!(super::is_sensitive("credit_card"));
        assert!(super::is_sensitive("auth_code"));
        assert!(!super::is_sensitive("name"));
        assert!(!super::is_sensitive("email"));
        assert!(!super::is_sensitive("age"));
    }

    #[test]
    fn test_mask_if_sensitive() {
        let val = serde_json::Value::String("my-secret-val".to_string());
        let masked = super::mask_if_sensitive("password", val.clone());
        assert_eq!(masked, serde_json::Value::String("***".to_string()));

        let unmasked = super::mask_if_sensitive("username", val);
        assert_eq!(
            unmasked,
            serde_json::Value::String("my-secret-val".to_string())
        );
    }
}

