#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{DatabaseEnum, Enum, Orm};

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
#[rullst_enum(type_name = "account_status", rename_all = "snake_case")]
enum AccountStatus {
    AwaitingReview,
    #[rullst_enum(rename = "live")]
    Active,
}

#[tokio::test]
async fn derived_enum_round_trips_and_enforces_sqlite_constraint() {
    const DB_FILE: &str = "native_enum_test.db";
    let _ = std::fs::remove_file(DB_FILE);
    Orm::init(&format!("sqlite:{DB_FILE}?mode=rwc"))
        .await
        .expect("SQLite ORM should initialize");

    Schema::create("native_enum_accounts", |table: &mut Blueprint| {
        table.id();
        table.native_enum::<AccountStatus>("status").not_null();
    })
    .await
    .expect("native enum schema should be created");

    sqlx::query("INSERT INTO native_enum_accounts (status) VALUES (?)")
        .bind(AccountStatus::AwaitingReview)
        .execute(Orm::pool().expect("ORM pool should exist"))
        .await
        .expect("derived enum should encode through SQLx");

    let stored = sqlx::query_scalar::<_, AccountStatus>(
        "SELECT status FROM native_enum_accounts WHERE id = 1",
    )
    .fetch_one(Orm::pool().expect("ORM pool should exist"))
    .await
    .expect("derived enum should decode through SQLx");
    assert_eq!(stored, AccountStatus::AwaitingReview);

    let invalid = sqlx::query("INSERT INTO native_enum_accounts (status) VALUES (?)")
        .bind("retired")
        .execute(Orm::pool().expect("ORM pool should exist"))
        .await;
    assert!(invalid.is_err(), "SQLite CHECK must reject unknown labels");

    let invalid_decode = sqlx::query_scalar::<_, AccountStatus>("SELECT 'retired'")
        .fetch_one(Orm::pool().expect("ORM pool should exist"))
        .await;
    assert!(
        invalid_decode.is_err(),
        "unknown database labels must fail closed while decoding"
    );

    assert_eq!(AccountStatus::TYPE_NAME, "account_status");
    assert_eq!(AccountStatus::VARIANTS, &["awaiting_review", "live"]);
    assert_eq!(AccountStatus::AwaitingReview.to_string(), "awaiting_review");
    assert_eq!(
        "live".parse::<AccountStatus>().expect("parse enum label"),
        AccountStatus::Active
    );
    let orm_value: rullst_orm::RullstValue = AccountStatus::Active.into();
    assert!(
        matches!(&orm_value, rullst_orm::RullstValue::String(value) if value == "live"),
        "RullstValue must use the derived database label"
    );
    assert_eq!(
        AccountStatus::try_from(orm_value).expect("convert ORM enum value"),
        AccountStatus::Active
    );
    assert_eq!(
        serde_json::to_string(&AccountStatus::Active).expect("serialize enum"),
        "\"live\""
    );
    assert_eq!(
        serde_json::from_str::<AccountStatus>("\"awaiting_review\"").expect("deserialize enum"),
        AccountStatus::AwaitingReview
    );

    Schema::drop_if_exists("native_enum_accounts")
        .await
        .expect("native enum table should be removed");
    Schema::drop_native_enum::<AccountStatus>()
        .await
        .expect("SQLite standalone enum cleanup should be a no-op");
    Orm::pool().expect("ORM pool should exist").close().await;
    let _ = std::fs::remove_file(DB_FILE);
}
