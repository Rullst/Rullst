//! Typed Turso model codecs and repository shape checks on the offline store.
#![cfg(feature = "turso")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use rullst_orm::polyglot::{
    PolyglotError, TursoCodec, TursoConfig, TursoModel, TursoPrimaryKey, TursoQueryLimit, TursoRow,
    TursoStatement, TursoStore, TursoValue,
};

/// The primary key is not the first column.
#[derive(Debug, Clone, PartialEq, rullst_orm::Orm)]
#[orm(table = "pets", backend = "turso")]
struct Pet {
    name: String,
    id: i64,
}

/// A text primary key without a DEFAULT: the application assigns it.
#[derive(Debug, Clone, PartialEq, rullst_orm::Orm)]
#[orm(table = "tags", backend = "turso")]
struct Tag {
    id: String,
    label: String,
}

/// A text primary key filled by the column's DEFAULT.
#[derive(Debug, Clone, PartialEq, rullst_orm::Orm)]
#[orm(table = "notes", backend = "turso")]
struct Note {
    id: String,
    body: String,
}

/// A hand-written model whose encoder omits a declared column.
struct Broken {
    id: i64,
}

impl TursoModel for Broken {
    fn table_name() -> &'static str {
        "broken"
    }

    fn columns() -> &'static [&'static str] {
        &["id", "value"]
    }

    fn primary_key_column() -> &'static str {
        "id"
    }

    fn encode_turso(&self) -> Result<Vec<TursoValue>, PolyglotError> {
        Ok(vec![TursoValue::Integer(self.id)])
    }

    fn decode_turso(_: &TursoRow) -> Result<Self, PolyglotError> {
        Ok(Self { id: 0 })
    }

    fn primary_key_value(&self) -> Result<TursoValue, PolyglotError> {
        Ok(TursoValue::Integer(self.id))
    }

    fn primary_key_is_unset(&self) -> bool {
        self.id == 0
    }

    fn assign_primary_key(&mut self, _: &TursoValue) -> Result<(), PolyglotError> {
        Ok(())
    }
}

async fn store() -> TursoStore {
    let store = TursoStore::connect(TursoConfig::new("mock_local", ""))
        .await
        .unwrap();
    for sql in [
        "CREATE TABLE pets (name TEXT NOT NULL, id INTEGER PRIMARY KEY AUTOINCREMENT)",
        "CREATE TABLE broken (id INTEGER PRIMARY KEY, value TEXT)",
        // Nullable on purpose: SQLite accepts NULL in a non-INTEGER key.
        "CREATE TABLE tags (id TEXT PRIMARY KEY, label TEXT NOT NULL UNIQUE)",
        "CREATE TABLE notes (id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))), body TEXT NOT NULL UNIQUE)",
    ] {
        store
            .execute(TursoStatement::new(sql, vec![]).unwrap())
            .await
            .unwrap();
    }
    store
}

#[tokio::test]
async fn a_primary_key_after_other_columns_round_trips() {
    let store = store().await;
    let mut rex = Pet {
        name: "Rex".to_owned(),
        id: 0,
    };
    store.models::<Pet>().save(&mut rex).await.expect("insert");
    assert!(rex.id > 0);
    rex.name = "Rex II".to_owned();
    store.models::<Pet>().save(&mut rex).await.expect("update");
    assert_eq!(
        store.models::<Pet>().find(rex.id).await.unwrap(),
        Some(rex.clone())
    );
    assert_eq!(store.models::<Pet>().query().count().await.unwrap(), 1);
}

async fn count(store: &TursoStore, sql: &str) -> i64 {
    let rows = store
        .query(
            TursoStatement::new(sql, vec![]).unwrap(),
            TursoQueryLimit::new(1).unwrap(),
        )
        .await
        .unwrap();
    match rows[0].get("c") {
        Some(TursoValue::Integer(count)) => *count,
        other => panic!("unexpected count {other:?}"),
    }
}

async fn tag_rows(store: &TursoStore) -> i64 {
    count(store, "SELECT COUNT(*) AS c FROM tags").await
}

#[tokio::test]
async fn an_empty_text_key_without_a_default_writes_nothing() {
    let store = store().await;
    let tags = store.models::<Tag>();
    for create in [false, true] {
        let mut unset = Tag {
            id: String::new(),
            label: "no key".to_owned(),
        };
        let refused = if create {
            tags.create(&mut unset).await
        } else {
            tags.save(&mut unset).await
        };
        match refused {
            Err(PolyglotError::InvalidIdentifier { kind, reason }) => {
                assert_eq!(kind, "Turso model primary key");
                assert_eq!(
                    reason,
                    "the database generated no value for this key; assign it before saving or give its column a DEFAULT"
                );
            }
            other => panic!("create={create}: {other:?}"),
        }
        assert_eq!(unset.id, "");
    }
    assert_eq!(tag_rows(&store).await, 0, "no row, NULL key or otherwise");

    let mut rust = Tag {
        id: "rust".to_owned(),
        label: "Rust".to_owned(),
    };
    tags.create(&mut rust).await.unwrap();
    assert_eq!(rust.id, "rust");
    assert_eq!(tags.find("rust".to_owned()).await.unwrap(), Some(rust));
    assert_eq!(tag_rows(&store).await, 1);
}

#[tokio::test]
async fn an_empty_text_key_takes_the_column_default() {
    let store = store().await;
    let notes = store.models::<Note>();
    let mut first = Note {
        id: String::new(),
        body: "first".to_owned(),
    };
    notes.save(&mut first).await.unwrap();
    assert_eq!(first.id.len(), 32, "{}", first.id);
    assert!(first.id.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(
        notes.find(first.id.clone()).await.unwrap(),
        Some(first.clone())
    );
    let mut second = Note {
        id: String::new(),
        body: "second".to_owned(),
    };
    notes.create(&mut second).await.unwrap();
    assert_ne!(second.id, first.id);
    // A conflict on the streamed insert is still a conflict and keeps nothing.
    let mut duplicate = Note {
        id: String::new(),
        body: "first".to_owned(),
    };
    assert!(matches!(
        notes.save(&mut duplicate).await,
        Err(PolyglotError::Conflict)
    ));
    assert_eq!(duplicate.id, "");
    assert_eq!(count(&store, "SELECT COUNT(*) AS c FROM notes").await, 2);
}

#[tokio::test]
async fn unique_and_primary_key_violations_are_conflicts() {
    let store = store().await;
    let tags = store.models::<Tag>();
    let mut first = Tag {
        id: "a".to_owned(),
        label: "A".to_owned(),
    };
    tags.create(&mut first).await.unwrap();
    // Same primary key, then same UNIQUE label.
    for (id, label) in [("a", "other"), ("b", "A")] {
        let mut duplicate = Tag {
            id: id.to_owned(),
            label: label.to_owned(),
        };
        assert!(
            matches!(
                tags.create(&mut duplicate).await,
                Err(PolyglotError::Conflict)
            ),
            "{id} {label}"
        );
    }
    let insert = |id: &str, label: Option<&str>| {
        TursoStatement::new(
            "INSERT INTO tags (id, label) VALUES (?1, ?2)",
            vec![
                TursoValue::Text(id.to_owned()),
                label.map_or(TursoValue::Null, |label| TursoValue::Text(label.to_owned())),
            ],
        )
        .unwrap()
    };
    assert!(matches!(
        store.execute(insert("a", Some("again"))).await,
        Err(PolyglotError::Conflict)
    ));
    assert!(matches!(
        store
            .transaction(vec![insert("c", Some("C")), insert("a", Some("again"))])
            .await,
        Err(PolyglotError::Conflict)
    ));
    // The failed transaction kept nothing.
    assert_eq!(tag_rows(&store).await, 1);
    // Other constraints remain driver errors.
    assert!(matches!(
        store.execute(insert("d", None)).await,
        Err(PolyglotError::Driver {
            backend: "Turso offline",
            ..
        })
    ));
}

#[tokio::test]
async fn encoders_must_produce_one_value_per_column() {
    let store = store().await;
    assert!(Broken::opaque_columns().is_empty());
    let mut broken = Broken { id: 0 };
    assert!(matches!(
        store.models::<Broken>().save(&mut broken).await,
        Err(PolyglotError::InvalidIdentifier {
            kind: "Turso model",
            ..
        })
    ));
    let mut existing = Broken { id: 4 };
    assert!(matches!(
        store.models::<Broken>().save(&mut existing).await,
        Err(PolyglotError::InvalidIdentifier {
            kind: "Turso model",
            ..
        })
    ));
}

#[test]
fn scalar_codecs_reject_values_outside_their_type() {
    assert_eq!(f32::decode_turso(&TursoValue::Real(1.5)).unwrap(), 1.5);
    for invalid in [f64::NAN, f64::INFINITY, 1e39, -1e39] {
        assert!(
            f32::decode_turso(&TursoValue::Real(invalid)).is_err(),
            "{invalid}"
        );
    }
    assert_eq!(f64::decode_turso(&TursoValue::Real(2.5)).unwrap(), 2.5);
    assert!(f64::decode_turso(&TursoValue::Real(f64::NAN)).is_err());
    assert_eq!(
        Vec::<u8>::decode_turso(&TursoValue::Blob(vec![1, 2])).unwrap(),
        [1, 2]
    );
    let error = String::decode_turso(&TursoValue::Integer(1)).unwrap_err();
    assert_eq!(
        error.to_string(),
        PolyglotError::Serialization("expected TEXT, received INTEGER".to_owned()).to_string()
    );

    let mut key = String::new();
    assert!(key.is_unset());
    key.assign_turso(&TursoValue::Text("pet-1".to_owned()))
        .unwrap();
    assert_eq!(key, "pet-1");
    assert!(!key.is_unset());
}
