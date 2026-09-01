#![cfg(feature = "turso")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use rullst_orm::polyglot::{TursoConfig, TursoMigration, TursoOrder, TursoOrm, TursoStatement};

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
    assert!(TursoOrm::store().is_err());
    assert!(TursoOrm::repository::<User>().is_err());
    assert!(User::paginate(0, 10).await.is_err());
    assert!(User::paginate(1, 0).await.is_err());
    assert!(User::paginate(1, 501).await.is_err());
    assert!(User::paginate(usize::MAX, 500).await.is_err());

    TursoOrm::init(TursoConfig::new("mock_local", ""))
        .await
        .unwrap();
    assert!(
        TursoOrm::init(TursoConfig::new("mock_second", ""))
            .await
            .is_err()
    );
    assert!(TursoOrm::init_from_env().await.is_err());
    let store = TursoOrm::store().unwrap();
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
    let repository = TursoOrm::repository::<User>().unwrap();
    let mut ada = User {
        id: 0,
        name: "Ada".to_owned(),
        active: true,
        score: Some(9.5),
    };
    ada.save().await.unwrap();
    assert!(ada.id > 0);

    let found = User::find(ada.id).await.unwrap().unwrap();
    assert_eq!(found, ada);
    let active = User::query()
        .unwrap()
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
    assert_eq!(User::count().await.unwrap(), 1);
    assert_eq!(User::all().await.unwrap(), vec![ada.clone()]);
    let first_page = User::paginate(1, 1).await.unwrap();
    assert_eq!(first_page.data, vec![ada.clone()]);
    assert_eq!(first_page.total, 1);
    assert_eq!(first_page.current_page, 1);
    assert_eq!(first_page.last_page, 1);

    ada.name = "Ada Lovelace".to_owned();
    ada.save().await.unwrap();
    assert_eq!(
        repository.find(ada.id).await.unwrap().unwrap().name,
        ada.name
    );
    let mut grace = User {
        id: 99,
        name: "Grace".to_owned(),
        active: true,
        score: None,
    };
    grace.create().await.unwrap();
    assert_eq!(User::find(99).await.unwrap(), Some(grace.clone()));

    ada.delete().await.unwrap();
    grace.delete().await.unwrap();
    assert!(User::find(ada.id).await.unwrap().is_none());

    let migration = TursoMigration::new(
        "m20260901_primary_notes",
        vec![
            TursoStatement::new(
                "CREATE TABLE primary_notes (id INTEGER PRIMARY KEY)",
                vec![],
            )
            .unwrap(),
        ],
    )
    .unwrap()
    .with_down(vec![
        TursoStatement::new("DROP TABLE primary_notes", vec![]).unwrap(),
    ])
    .unwrap();
    let report = TursoOrm::migrate(vec![migration.clone()]).await.unwrap();
    assert_eq!(report.applied, vec!["m20260901_primary_notes"]);
    assert_eq!(
        TursoOrm::migration_status().await.unwrap(),
        vec!["m20260901_primary_notes"]
    );
    let rollback = TursoOrm::rollback_last(vec![migration]).await.unwrap();
    assert_eq!(
        rollback.rolled_back.as_deref(),
        Some("m20260901_primary_notes")
    );
}
