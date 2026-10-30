//! `#[derive(Orm)]` implements `FromRow` (since 13.0.0-alpha.4): every
//! `#[sqlx(...)]` option a model accepts keeps SQLx's meaning, relations are
//! never read, and `#[orm(custom_from_row)]` leaves the impl to the model.
#![cfg(all(
    feature = "strict-sqlite",
    not(feature = "strict-postgres"),
    not(feature = "strict-mysql")
))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_orm::sqlx::sqlite::SqliteRow;
use rullst_orm::sqlx::{self, Row};
use rullst_orm::{FromRow, Orm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Settings {
    theme: String,
}

#[derive(Debug, Clone, Default, Orm)]
#[orm(table = "from_row_owners")]
struct Owner {
    id: i32,
    name: String,
}

// `Default`: generated `from_json` fills skipped fields from it.
#[derive(Debug, Clone, Default, Orm)]
#[orm(table = "from_row_profiles")]
struct Profile {
    id: i32,
    owner_id: i32,
    #[sqlx(json)]
    settings: Settings,
    #[sqlx(json(nullable))]
    extra: Option<Settings>,
    #[sqlx(default)]
    views: i64,
    // Generated writes leave it to the database; reads still decode it.
    #[orm(skip)]
    computed: i64,
    #[sqlx(skip)]
    cache: String,
    // No `#[sqlx(skip)]`: a relation is never a column.
    #[orm(belongs_to = "Owner", foreign_key = "owner_id")]
    owner: Option<Owner>,
}

#[derive(Debug, Clone, Orm)]
#[orm(table = "from_row_counters")]
#[sqlx(default)]
struct Counter {
    id: i32,
    label: String,
    hits: i64,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            id: -1,
            label: "unnamed".to_string(),
            hits: 7,
        }
    }
}

#[derive(Debug, Clone, Orm)]
#[orm(table = "from_row_owners", custom_from_row)]
struct ShoutingOwner {
    id: i32,
    name: String,
}

impl<'r> FromRow<'r, SqliteRow> for ShoutingOwner {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get::<String, _>("name")?.to_uppercase(),
        })
    }
}

fn settings(theme: &str) -> Settings {
    Settings {
        theme: theme.to_string(),
    }
}

async fn execute(sql: &str) {
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .execute(Orm::pool().unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn derived_from_row_reads_every_supported_sqlx_option() {
    Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .unwrap();
    execute(
        "CREATE TABLE from_row_owners (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)",
    )
    .await;
    execute(
        "CREATE TABLE from_row_profiles (id INTEGER PRIMARY KEY AUTOINCREMENT, owner_id INTEGER NOT NULL, \
         settings TEXT NOT NULL, extra TEXT, views BIGINT NOT NULL, computed BIGINT NOT NULL DEFAULT 42)",
    )
    .await;
    execute("CREATE TABLE from_row_counters (id INTEGER PRIMARY KEY AUTOINCREMENT, label TEXT NOT NULL, hits BIGINT NOT NULL)").await;

    let mut owner = Owner {
        id: 0,
        name: "Ada".to_string(),
    };
    owner.save().await.unwrap();
    let mut profile = Profile {
        id: 0,
        owner_id: owner.id,
        settings: settings("dark"),
        extra: None,
        views: 3,
        computed: 0,
        cache: "local only".to_string(),
        owner: Some(owner.clone()),
    };
    profile.save().await.unwrap();

    let stored = Profile::find(profile.id)
        .await
        .unwrap()
        .expect("saved profile");
    assert_eq!(stored.settings, settings("dark"));
    assert_eq!(stored.extra, None);
    assert_eq!(stored.views, 3);
    assert_eq!(stored.computed, 42, "an #[orm(skip)] column is still read");
    assert_eq!(stored.cache, "", "#[sqlx(skip)] starts from Default");
    assert!(
        stored.owner.is_none(),
        "a relation is never read from the row"
    );

    let mut updated = stored.clone();
    updated.extra = Some(settings("contrast"));
    updated.save().await.unwrap();
    let stored = Profile::query().get().await.unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].extra, Some(settings("contrast")));

    // A JSON `null` decodes to `None`, like SQL NULL.
    execute("UPDATE from_row_profiles SET extra = 'null'").await;
    let stored = Profile::find(profile.id).await.unwrap().expect("profile");
    assert_eq!(stored.extra, None);

    // `#[sqlx(default)]` fills only a column the row lacks.
    let partial: Profile =
        sqlx::query_as("SELECT id, owner_id, settings, extra, computed FROM from_row_profiles")
            .fetch_one(Orm::pool().unwrap())
            .await
            .unwrap();
    assert_eq!(partial.views, 0);
    assert_eq!(partial.settings, settings("dark"));
    let missing = sqlx::query_as::<_, Profile>(
        "SELECT id, owner_id, settings, extra, views FROM from_row_profiles",
    )
    .fetch_one(Orm::pool().unwrap())
    .await
    .expect_err("a column without a default must be present");
    assert!(
        matches!(&missing, sqlx::Error::ColumnNotFound(column) if column == "computed"),
        "{missing:?}"
    );

    // A struct-level `#[sqlx(default)]` takes missing columns from
    // `Self::default()`.
    let mut counter = Counter {
        id: 0,
        label: "visits".to_string(),
        hits: 12,
    };
    counter.save().await.unwrap();
    let full = Counter::find(counter.id).await.unwrap().expect("counter");
    assert_eq!((full.label.as_str(), full.hits), ("visits", 12));
    let sparse: Counter = sqlx::query_as("SELECT id FROM from_row_counters")
        .fetch_one(Orm::pool().unwrap())
        .await
        .unwrap();
    assert_eq!(sparse.id, counter.id);
    assert_eq!((sparse.label.as_str(), sparse.hits), ("unnamed", 7));

    // `#[orm(custom_from_row)]`: the model's own impl decodes the rows.
    let shouting = ShoutingOwner::find(owner.id).await.unwrap().expect("owner");
    assert_eq!(shouting.name, "ADA");
}
