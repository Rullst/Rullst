//! `Orm::transaction` returns the closure's error unchanged after rolling
//! back, so constraint violations and application errors keep their type, and
//! a commit rejected by a deferred constraint is typed too.

use std::fmt;

use rullst_orm::schema::Schema;
use rullst_orm::{Error, Orm};

#[derive(Debug, Clone, Orm)]
#[orm(table = "contract_tx_accounts")]
struct ContractTxAccount {
    id: i32,
    email: String,
}

#[derive(Debug, Clone, Orm)]
#[orm(table = "contract_tx_memberships")]
struct ContractTxMembership {
    id: i32,
    account_id: i32,
}

/// An application error that wraps the ORM's, as `E: From<Error>` requires.
#[derive(Debug)]
enum ContractAppError {
    Orm(Error),
    QuotaExceeded { remaining: u32 },
}

impl From<Error> for ContractAppError {
    fn from(error: Error) -> Self {
        Self::Orm(error)
    }
}

impl fmt::Display for ContractAppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Orm(error) => error.fmt(f),
            Self::QuotaExceeded { remaining } => write!(f, "quota exceeded ({remaining} left)"),
        }
    }
}

/// Only PostgreSQL reports the constraint name.
fn expected_name(driver: &str, name: &str) -> Option<String> {
    (driver == "postgres").then(|| name.to_string())
}

async fn account(email: &str) -> Result<ContractTxAccount, Error> {
    let mut account = ContractTxAccount {
        id: 0,
        email: email.to_string(),
    };
    account.save().await?;
    Ok(account)
}

async fn emails() -> Vec<String> {
    ContractTxAccount::query()
        .order_by("id")
        .pluck_string("email")
        .await
        .expect("read transaction contract accounts")
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    Schema::create("contract_tx_accounts", |table| {
        table.id();
        // MySQL/MariaDB cannot make a TEXT column unique.
        table.string("email").not_null().col_type = "VARCHAR(191)".to_string();
        table
            .unique(&["email"])
            .name("contract_tx_accounts_email_key");
    })
    .await
    .expect("create transaction contract accounts");
    Schema::create("contract_tx_memberships", |table| {
        table.id();
        table.integer("account_id").not_null();
        table
            .foreign("account_id")
            .references("contract_tx_accounts", "id")
            .name("contract_tx_memberships_account_fkey");
    })
    .await
    .expect("create transaction contract memberships");
    let alice = account("alice@example.test").await.expect("insert alice");
    let unique = expected_name(driver, "contract_tx_accounts_email_key");
    let foreign = expected_name(driver, "contract_tx_memberships_account_fkey");

    // A unique violation keeps its variant, and the earlier write rolls back.
    let duplicate = Orm::transaction(|_| {
        Box::pin(async {
            account("bob@example.test").await?;
            account("alice@example.test").await?;
            Ok::<(), Error>(())
        })
    })
    .await;
    assert!(
        matches!(&duplicate, Err(Error::UniqueViolation { constraint, .. }) if *constraint == unique),
        "{driver} duplicate inside a transaction: {duplicate:?}"
    );
    assert_eq!(emails().await, vec!["alice@example.test"], "{driver}");

    // A foreign-key violation keeps its variant too.
    let orphan = Orm::transaction(move |_| {
        Box::pin(async move {
            account("carol@example.test").await?;
            let mut membership = ContractTxMembership {
                id: 0,
                account_id: alice.id + 1000,
            };
            membership.save().await?;
            Ok::<(), Error>(())
        })
    })
    .await;
    assert!(
        matches!(&orphan, Err(Error::ForeignKeyViolation { constraint, .. }) if *constraint == foreign),
        "{driver} orphan inside a transaction: {orphan:?}"
    );
    assert_eq!(emails().await, vec!["alice@example.test"], "{driver}");

    // The application's own error type round-trips, wrapped ORM errors too.
    let quota = Orm::transaction(|_| {
        Box::pin(async {
            account("dave@example.test").await?;
            Err::<(), ContractAppError>(ContractAppError::QuotaExceeded { remaining: 0 })
        })
    })
    .await;
    assert!(
        matches!(quota, Err(ContractAppError::QuotaExceeded { remaining: 0 })),
        "{driver} application error: {quota:?}"
    );
    let wrapped = Orm::transaction(|_| {
        Box::pin(async {
            account("alice@example.test").await?;
            Ok::<(), ContractAppError>(())
        })
    })
    .await;
    assert!(
        matches!(&wrapped, Err(ContractAppError::Orm(Error::UniqueViolation { constraint, .. })) if *constraint == unique),
        "{driver} wrapped violation: {wrapped:?}"
    );
    assert_eq!(emails().await, vec!["alice@example.test"], "{driver}");

    // A nested call returns the violation to its caller, which can recover.
    let expected = unique.clone();
    let recovered = Orm::transaction(move |_| {
        Box::pin(async move {
            account("erin@example.test").await?;
            let nested = Orm::transaction(|_| {
                Box::pin(async {
                    account("frank@example.test").await?;
                    account("alice@example.test").await?;
                    Ok::<(), Error>(())
                })
            })
            .await;
            assert!(
                matches!(&nested, Err(Error::UniqueViolation { constraint, .. }) if *constraint == expected),
                "nested duplicate: {nested:?}"
            );
            Ok::<&'static str, Error>("recovered")
        })
    })
    .await;
    assert_eq!(recovered.expect("outer commits"), "recovered", "{driver}");
    assert_eq!(
        emails().await,
        vec!["alice@example.test", "erin@example.test"],
        "{driver}"
    );

    if driver != "mysql" {
        deferred_commit_failure(driver, alice.id).await;
    }

    Schema::drop_if_exists("contract_tx_memberships")
        .await
        .expect("drop transaction contract memberships");
    Schema::drop_if_exists("contract_tx_accounts")
        .await
        .expect("drop transaction contract accounts");
}

/// A deferred foreign key is checked at COMMIT (MySQL/MariaDB have none): the
/// rejected commit reaches the caller as a typed violation through `E::from`.
async fn deferred_commit_failure(driver: &str, account_id: i32) {
    let id_column = if driver == "postgres" {
        "id SERIAL PRIMARY KEY"
    } else {
        "id INTEGER PRIMARY KEY"
    };
    Orm::raw(format!(
        "CREATE TABLE contract_tx_deferred ({id_column}, account_id INTEGER NOT NULL, \
         CONSTRAINT contract_tx_deferred_account_fkey FOREIGN KEY (account_id) \
         REFERENCES contract_tx_accounts (id) DEFERRABLE INITIALLY DEFERRED)"
    ))
    .execute()
    .await
    .expect("create deferred contract table");
    let insert = if driver == "postgres" {
        "INSERT INTO contract_tx_deferred (account_id) VALUES ($1)"
    } else {
        "INSERT INTO contract_tx_deferred (account_id) VALUES (?)"
    };
    let rejected = Orm::transaction(move |_| {
        Box::pin(async move {
            let inserted = Orm::raw(insert).bind(account_id + 1000).execute().await?;
            assert_eq!(inserted, 1, "the deferred check waits for COMMIT");
            Ok::<(), ContractAppError>(())
        })
    })
    .await;
    let foreign = expected_name(driver, "contract_tx_deferred_account_fkey");
    assert!(
        matches!(&rejected, Err(ContractAppError::Orm(Error::ForeignKeyViolation { constraint, .. })) if *constraint == foreign),
        "{driver} deferred commit failure: {rejected:?}"
    );
    let rows: Vec<(i64,)> = Orm::raw("SELECT COUNT(*) FROM contract_tx_deferred")
        .map_to()
        .await
        .expect("count deferred rows");
    assert_eq!(
        rows,
        vec![(0,)],
        "{driver} the rejected commit kept nothing"
    );
    Schema::drop_if_exists("contract_tx_deferred")
        .await
        .expect("drop deferred contract table");
}
