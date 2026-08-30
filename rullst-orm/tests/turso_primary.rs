#![cfg(feature = "turso")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use rullst_orm::polyglot::{TursoConfig, TursoOrder, TursoStatement, TursoStore};

#[derive(Debug, Clone, PartialEq, rullst_orm::Orm)]
#[orm(table = "users", backend = "turso")]
struct User {
    id: i64,
    name: String,
    active: bool,
    score: Option<f64>,
}

#[tokio::test]
async fn primary_model_contract_covers_crud_filters_order_and_counts() {
    let store = TursoStore::connect(TursoConfig::new("mock_local", ""))
        .await
        .unwrap();
    store
        .execute(
            TursoStatement::new(
                "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, active INTEGER NOT NULL, score REAL)",
                vec![],
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let repository = store.models::<User>();
    let mut ada = User {
        id: 0,
        name: "Ada".to_owned(),
        active: true,
        score: Some(9.5),
    };
    repository.save(&mut ada).await.unwrap();
    assert!(ada.id > 0);

    let found = repository.find(ada.id).await.unwrap().unwrap();
    assert_eq!(found, ada);
    let active = repository
        .query()
        .where_eq("active", &true)
        .unwrap()
        .order_by("name", TursoOrder::Desc)
        .unwrap()
        .limit(10)
        .unwrap()
        .get()
        .await
        .unwrap();
    assert_eq!(active, vec![ada.clone()]);
    assert_eq!(repository.query().count().await.unwrap(), 1);

    ada.name = "Ada Lovelace".to_owned();
    repository.save(&mut ada).await.unwrap();
    assert_eq!(
        repository.find(ada.id).await.unwrap().unwrap().name,
        ada.name
    );
    repository.delete(&ada).await.unwrap();
    assert!(repository.find(ada.id).await.unwrap().is_none());
}
