//! Typed `Blueprint` helpers round-trip their Rust field types.

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_measurements")]
struct ContractMeasurement {
    id: i32,
    price: f64,
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_measurements", |table: &mut Blueprint| {
        table.id();
        table.float("price").not_null();
    })
    .await
    .expect("create float contract table");

    // Not representable in single precision (it would read back as
    // 1234567.875), so a 4-byte column cannot hide behind a lucky value.
    let price = 1_234_567.89_f64;
    let mut row = ContractMeasurement { id: 0, price };
    row.save().await.expect("save float row");
    let stored = ContractMeasurement::find(row.id)
        .await
        .unwrap_or_else(|error| panic!("{driver} float row must decode as f64: {error}"))
        .expect("stored float row");
    assert_eq!(
        stored.price.to_bits(),
        price.to_bits(),
        "{driver} float() must keep f64 precision"
    );

    Schema::drop_if_exists("contract_measurements")
        .await
        .expect("drop float contract table");
}
