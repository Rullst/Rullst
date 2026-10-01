//! `#[sqlx(json)]` fields round-trip through the generated INSERT, UPDATE and
//! partial update on a strict driver, whose `FromRow` decodes them through
//! SQLx `Json`. `Preferences` implements only Serde, not `Encode`.

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preferences {
    pub theme: String,
    pub beta: bool,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "json_profiles")]
pub struct JsonProfile {
    pub id: i32,
    #[sqlx(json)]
    pub tags: Vec<String>,
    #[sqlx(json)]
    pub preferences: Preferences,
    #[sqlx(json(nullable))]
    pub extra: Option<Preferences>,
}

async fn stored(id: i32) -> JsonProfile {
    JsonProfile::find(id)
        .await
        .expect("read JSON profile")
        .expect("stored JSON profile")
}

/// Runs on the strict driver's native JSON column type (`json_type`).
#[allow(dead_code)]
pub async fn exercise_sqlx_json(json_type: &str) {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("json_profiles", |table: &mut Blueprint| {
        table.id();
    })
    .await
    .expect("create JSON profile table");
    let pool = Orm::pool().expect("ORM pool");
    for column in ["tags", "preferences", "extra"] {
        let alter = format!("ALTER TABLE json_profiles ADD COLUMN {column} {json_type}");
        rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(alter.as_str()))
            .execute(pool)
            .await
            .expect("add JSON column");
    }

    let mut profile = JsonProfile {
        id: 0,
        tags: vec!["rust".to_string(), "orm".to_string()],
        preferences: Preferences {
            theme: "dark".to_string(),
            beta: true,
        },
        extra: None,
    };
    profile
        .save()
        .await
        .unwrap_or_else(|error| panic!("{driver} insert of JSON fields: {error}"));
    let inserted = stored(profile.id).await;
    assert_eq!(inserted.tags, profile.tags);
    assert_eq!(inserted.preferences, profile.preferences);
    assert_eq!(inserted.extra, None, "{driver} json(nullable) None is NULL");

    profile.tags.push("json".to_string());
    profile.extra = Some(Preferences {
        theme: "light".to_string(),
        beta: false,
    });
    profile
        .save()
        .await
        .unwrap_or_else(|error| panic!("{driver} update of JSON fields: {error}"));
    let updated = stored(profile.id).await;
    assert_eq!(updated.tags, profile.tags);
    assert_eq!(updated.extra, profile.extra);

    profile
        .update_partial()
        .tags(vec!["partial".to_string()])
        .save()
        .await
        .unwrap_or_else(|error| panic!("{driver} partial update of JSON fields: {error}"));
    assert_eq!(stored(profile.id).await.tags, vec!["partial".to_string()]);

    Schema::drop_if_exists("json_profiles")
        .await
        .expect("drop JSON profile table");
}
