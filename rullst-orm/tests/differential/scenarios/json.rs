//! `#[sqlx(json)]` fields. `Blueprint` has no JSON helper, so the column
//! type is the driver's JSON type: `TEXT` on SQLite, `JSONB` on PostgreSQL
//! and `JSON` on MySQL/MariaDB (where MariaDB's `JSON` is a checked
//! `LONGTEXT`). Everything else is the same table definition.

use rullst_orm::{Error, Orm};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::create_table;
use crate::transcript::Scenario;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Preferences {
    theme: String,
    beta: bool,
    limits: Vec<i64>,
}

#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_documents")]
struct Document {
    id: i32,
    #[sqlx(json)]
    tags: Vec<String>,
    #[sqlx(json)]
    preferences: Preferences,
    #[sqlx(json(nullable))]
    extra: Option<Preferences>,
}

impl Document {
    fn columns(&self) -> Value {
        json!([self.tags, self.preferences, self.extra])
    }
}

fn json_type(driver: &str) -> &'static str {
    match driver {
        "postgres" => "JSONB",
        "mysql" => "JSON",
        _ => "TEXT",
    }
}

async fn stored(id: i32) -> Result<Value, Error> {
    let document = Document::find(id).await?.ok_or(Error::RecordNotFound)?;
    Ok(document.columns())
}

pub async fn run(s: &mut Scenario<'_>) {
    let column_type = json_type(Orm::driver().unwrap_or_default());
    let created = create_table(s, "diff_documents", |table| {
        table.id();
        table.string("tags").not_null().col_type = column_type.to_string();
        table.string("preferences").not_null().col_type = column_type.to_string();
        table.string("extra").col_type = column_type.to_string();
    })
    .await;
    if !created {
        return;
    }
    let mut document = Document {
        id: 0,
        tags: vec!["rust".to_string(), "ünïcødé 👋".to_string(), String::new()],
        preferences: Preferences {
            theme: "dark \"quoted\"".to_string(),
            beta: true,
            limits: vec![i64::MIN, 0, i64::MAX],
        },
        extra: None,
    };
    let inserted = document.save().await;
    let ok = inserted.is_ok();
    s.outcome("insert", inserted);
    if !ok {
        return;
    }
    s.outcome("round_trip", stored(document.id).await);

    document.tags.push("added".to_string());
    document.extra = Some(Preferences {
        theme: "light".to_string(),
        beta: false,
        limits: Vec::new(),
    });
    s.outcome("update", document.save().await);
    s.outcome("after_update", stored(document.id).await);
    s.outcome(
        "update_partial",
        document.update_partial().extra(None).save().await,
    );
    s.outcome("after_update_partial", stored(document.id).await);
}
