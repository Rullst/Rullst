//! A model whose only persisted column is `id` must insert and update on
//! every driver (`DEFAULT VALUES`, or `() VALUES ()` on MySQL/MariaDB).

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_markers")]
struct ContractMarker {
    id: i32,
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_markers", |table: &mut Blueprint| {
        table.id();
    })
    .await
    .expect("create id-only contract table");

    let mut first = ContractMarker { id: 0 };
    first
        .save()
        .await
        .unwrap_or_else(|error| panic!("{driver} insert of an id-only row: {error}"));
    assert!(first.id > 0, "{driver} returns the generated id");
    first
        .save()
        .await
        .unwrap_or_else(|error| panic!("{driver} update of an id-only row: {error}"));
    let mut second = ContractMarker { id: 0 };
    second.save().await.expect("insert a second id-only row");
    assert_ne!(first.id, second.id);
    assert_eq!(
        ContractMarker::all()
            .await
            .expect("list id-only rows")
            .len(),
        2,
        "{driver}"
    );
    first.delete().await.expect("delete id-only row");

    Schema::drop_if_exists("contract_markers")
        .await
        .expect("drop id-only contract table");
}
