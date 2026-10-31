//! Constraint violations and missing rows reach the caller as the same
//! `rullst_orm::Error` kind on every backend.

use rullst_orm::Orm;
use rullst_orm::schema::Schema;

use super::create_table;
use crate::transcript::Scenario;

#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_accounts")]
struct Account {
    id: i32,
    email: String,
    nickname: String,
    code: String,
}

/// The same table seen by a model whose `nickname` may be `None`, so the
/// generated INSERT can send a NULL to the NOT NULL column.
#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_accounts")]
struct AccountDraft {
    id: i32,
    email: String,
    nickname: Option<String>,
    code: String,
}

#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_memberships")]
struct Membership {
    id: i32,
    account_id: i32,
}

fn account(email: &str, code: &str) -> Account {
    Account {
        id: 0,
        email: email.to_string(),
        nickname: "nick".to_string(),
        code: code.to_string(),
    }
}

async fn create_tables(s: &mut Scenario<'_>) -> bool {
    let accounts = create_table(s, "diff_accounts", |table| {
        table.id();
        table.string("email").not_null().col_type = "VARCHAR(191)".to_string();
        table.string("nickname").not_null();
        table.string("code").not_null().col_type = "VARCHAR(8)".to_string();
        table.unique(&["email"]).name("diff_accounts_email_key");
    })
    .await;
    accounts
        && create_table(s, "diff_memberships", |table| {
            table.id();
            table.integer("account_id").not_null();
            table
                .foreign("account_id")
                .references("diff_accounts", "id")
                .name("diff_memberships_account_fkey");
        })
        .await
}

pub async fn run(s: &mut Scenario<'_>) {
    if !create_tables(s).await {
        return;
    }
    let mut owner = account("owner@example.test", "OWNER");
    let saved = owner.save().await;
    let ok = saved.is_ok();
    s.outcome("insert", saved);
    if !ok {
        return;
    }

    let mut duplicate = account("owner@example.test", "DUP");
    s.outcome("unique_violation", duplicate.save().await);
    s.record("unique_violation_keeps_id_zero", duplicate.id == 0);

    let mut draft = AccountDraft {
        id: 0,
        email: "draft@example.test".to_string(),
        nickname: None,
        code: "DRAFT".to_string(),
    };
    s.outcome("not_null_violation", draft.save().await);

    let mut long_code = account("long@example.test", "NINECHARS");
    s.outcome("varchar_overflow", long_code.save().await);

    let mut orphan = Membership {
        id: 0,
        account_id: owner.id + 1_000,
    };
    s.outcome("foreign_key_violation", orphan.save().await);
    let mut membership = Membership {
        id: 0,
        account_id: owner.id,
    };
    s.outcome("insert_membership", membership.save().await);
    s.outcome("delete_referenced", owner.delete().await);
    // SQLite keeps the over-long code (see `varchar_overflow`); leave it out.
    s.outcome(
        "accounts_after_failures",
        Account::query()
            .where_not_eq("email", "long@example.test")
            .count()
            .await,
    );

    s.outcome("delete_membership", membership.delete().await);
    let stale = owner.clone();
    s.outcome("delete_owner", owner.delete().await);
    let mut edited = stale.clone();
    edited.nickname = "edited".to_string();
    s.outcome("save_deleted_row", edited.save().await);
    s.outcome("delete_deleted_row", stale.delete().await);
    s.outcome(
        "find_deleted_row",
        Account::find(stale.id).await.map(|row| row.is_some()),
    );
    s.outcome(
        "update_partial_deleted_row",
        edited
            .update_partial()
            .nickname("partial".to_string())
            .save()
            .await,
    );

    // Generated SQL leaves identifiers unquoted (docs/src/spec.md), so a
    // column named after a reserved word works only where it is not reserved.
    let reserved = Schema::create("diff_reserved_words", |table| {
        table.id();
        table.big_integer("rank");
    })
    .await;
    s.outcome("reserved_word_column", reserved);
}
