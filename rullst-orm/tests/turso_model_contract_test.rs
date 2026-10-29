//! Typed Turso model codecs and repository shape checks on the offline store.
#![cfg(feature = "turso")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::polyglot::{
    PolyglotError, TursoCodec, TursoConfig, TursoModel, TursoPrimaryKey, TursoRow, TursoStatement,
    TursoStore, TursoValue,
};

/// The primary key is not the first column.
#[derive(Debug, Clone, PartialEq, rullst_orm::Orm)]
#[orm(table = "pets", backend = "turso")]
struct Pet {
    name: String,
    id: i64,
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
