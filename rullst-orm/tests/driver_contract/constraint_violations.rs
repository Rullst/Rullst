//! Unique and foreign-key violations reach the caller as typed errors from
//! generated writes and raw SQL, without the rejected values.

use rullst_orm::schema::Schema;
use rullst_orm::{Error, FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_accounts")]
struct ContractAccount {
    id: i32,
    email: String,
}

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_memberships")]
struct ContractMembership {
    id: i32,
    account_id: i32,
}

/// Only PostgreSQL reports the constraint name.
fn expected_name(driver: &str, name: &str) -> Option<String> {
    (driver == "postgres").then(|| name.to_string())
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    let id = match driver {
        "postgres" => "SERIAL PRIMARY KEY",
        "mysql" => "INTEGER PRIMARY KEY AUTO_INCREMENT",
        _ => "INTEGER PRIMARY KEY AUTOINCREMENT",
    };
    // `Schema` has no UNIQUE or FOREIGN KEY yet, so the DDL is raw.
    Orm::raw(format!(
        "CREATE TABLE contract_accounts (id {id}, email VARCHAR(191) NOT NULL, \
         CONSTRAINT contract_accounts_email_key UNIQUE (email))"
    ))
    .execute()
    .await
    .expect("create unique contract table");
    Orm::raw(format!(
        "CREATE TABLE contract_memberships (id {id}, account_id INTEGER NOT NULL, \
         CONSTRAINT contract_memberships_account_fkey FOREIGN KEY (account_id) \
         REFERENCES contract_accounts (id))"
    ))
    .execute()
    .await
    .expect("create foreign-key contract table");

    let mut account = ContractAccount {
        id: 0,
        email: "alice@example.test".to_string(),
    };
    account.save().await.expect("insert account");

    let mut duplicate = ContractAccount {
        id: 0,
        email: "alice@example.test".to_string(),
    };
    let saved = duplicate.save().await;
    let unique = expected_name(driver, "contract_accounts_email_key");
    assert!(
        matches!(&saved, Err(Error::UniqueViolation { constraint, .. }) if *constraint == unique),
        "{driver} duplicate save(): {saved:?}"
    );
    assert_eq!(duplicate.id, 0, "{driver} the failed insert keeps the id");
    let shown = format!("{saved:?} {}", saved.as_ref().unwrap_err());
    assert!(
        !shown.contains("alice@example.test"),
        "{driver} the error must not quote the value: {shown}"
    );

    let insert = if driver == "postgres" {
        "INSERT INTO contract_accounts (email) VALUES ($1)"
    } else {
        "INSERT INTO contract_accounts (email) VALUES (?)"
    };
    let raw = Orm::raw(insert).bind("alice@example.test").execute().await;
    assert!(
        matches!(&raw, Err(Error::UniqueViolation { constraint, .. }) if *constraint == unique),
        "{driver} duplicate raw insert: {raw:?}"
    );

    let foreign = expected_name(driver, "contract_memberships_account_fkey");
    let mut orphan = ContractMembership {
        id: 0,
        account_id: account.id + 1000,
    };
    let saved = orphan.save().await;
    assert!(
        matches!(&saved, Err(Error::ForeignKeyViolation { constraint, .. }) if *constraint == foreign),
        "{driver} membership of a missing account: {saved:?}"
    );

    let mut membership = ContractMembership {
        id: 0,
        account_id: account.id,
    };
    membership.save().await.expect("insert membership");
    let deleted = account.delete().await;
    assert!(
        matches!(&deleted, Err(Error::ForeignKeyViolation { constraint, .. }) if *constraint == foreign),
        "{driver} delete of a referenced account: {deleted:?}"
    );

    Schema::drop_if_exists("contract_memberships")
        .await
        .expect("drop foreign-key contract table");
    Schema::drop_if_exists("contract_accounts")
        .await
        .expect("drop unique contract table");
}
