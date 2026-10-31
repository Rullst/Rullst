//! `Orm::transaction`: commit, rollback on an error or a violation, nested
//! transactions (savepoints), work after a caught failure of a model write or
//! of a raw statement, and what a rolled-back insert does to the next
//! generated id.

use rullst_orm::{Error, Orm};

use super::{create_table, in_order};
use crate::transcript::Scenario;

#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_ledger")]
struct Entry {
    id: i32,
    name: String,
}

impl Entry {
    fn name(&self) -> &str {
        &self.name
    }
}

async fn insert(name: &str) -> Result<i32, Error> {
    let mut entry = Entry {
        id: 0,
        name: name.to_string(),
    };
    entry.save().await?;
    Ok(entry.id)
}

fn failure() -> Error {
    Error::Validation("scenario failure".to_string())
}

/// Which of `candidates` are stored, in insertion order. Each step looks only
/// at its own rows, so one documented difference cannot spread to the next.
async fn stored(candidates: &[&str]) -> Result<Vec<String>, Error> {
    let query = Entry::query().where_in("name", candidates.to_vec());
    in_order(query.order_by("id").get().await, Entry::name)
}

pub async fn run(s: &mut Scenario<'_>) {
    let created = create_table(s, "diff_ledger", |table| {
        table.id();
        table.string("name").not_null().col_type = "VARCHAR(64)".to_string();
        table.unique(&["name"]).name("diff_ledger_name_key");
    })
    .await;
    if !created {
        return;
    }

    let committed = Orm::transaction(|_| {
        Box::pin(async {
            insert("a").await?;
            insert("b").await?;
            Entry::query().count().await
        })
    })
    .await;
    s.outcome("commit_sees_own_rows", committed);
    s.outcome("after_commit", stored(&["a", "b"]).await);

    let failed = Orm::transaction(|_| {
        Box::pin(async {
            insert("c").await?;
            Err::<(), Error>(failure())
        })
    })
    .await;
    s.outcome("closure_error_is_returned", failed);
    s.outcome("after_closure_error", stored(&["c"]).await);

    let violated = Orm::transaction(|_| {
        Box::pin(async {
            insert("d").await?;
            insert("a").await?;
            Ok::<(), Error>(())
        })
    })
    .await;
    s.outcome("violation_is_returned", violated);
    s.outcome("after_violation", stored(&["d"]).await);

    let nested = Orm::transaction(|_| {
        Box::pin(async {
            insert("e").await?;
            let inner = Orm::transaction(|_| {
                Box::pin(async {
                    insert("f").await?;
                    Err::<(), Error>(failure())
                })
            })
            .await;
            insert("g").await?;
            Ok::<bool, Error>(inner.is_err())
        })
    })
    .await;
    s.outcome("savepoint_rollback_keeps_outer", nested);
    s.outcome("after_savepoint_rollback", stored(&["e", "f", "g"]).await);

    let outer_failed = Orm::transaction(|_| {
        Box::pin(async {
            insert("h").await?;
            Orm::transaction(|_| Box::pin(async { insert("i").await })).await?;
            Err::<(), Error>(failure())
        })
    })
    .await;
    s.outcome("outer_rollback_discards_released_savepoint", outer_failed);
    s.outcome("after_outer_rollback", stored(&["h", "i"]).await);

    run_after_caught_failures(s).await;
    run_generated_ids(s).await;
}

/// A failed statement that the closure catches, with and without a nested
/// transaction around it.
async fn run_after_caught_failures(s: &mut Scenario<'_>) {
    let continued = Orm::transaction(|_| {
        Box::pin(async {
            insert("j").await?;
            let duplicate = insert("a").await;
            insert("k").await?;
            Ok::<bool, Error>(duplicate.is_err())
        })
    })
    .await;
    s.outcome("continue_after_caught_violation", continued);
    s.outcome("after_caught_violation", stored(&["j", "k"]).await);

    let recovered = Orm::transaction(|_| {
        Box::pin(async {
            insert("l").await?;
            let duplicate = Orm::transaction(|_| Box::pin(async { insert("a").await })).await;
            insert("m").await?;
            Ok::<bool, Error>(duplicate.is_err())
        })
    })
    .await;
    s.outcome("continue_after_savepoint_violation", recovered);
    s.outcome("after_savepoint_violation", stored(&["l", "m"]).await);

    // `Orm::raw` joins the transaction without a savepoint of its own.
    let raw = Orm::transaction(|_| {
        Box::pin(async {
            insert("q").await?;
            let duplicate = Orm::raw(raw_insert()?).bind("a").execute().await;
            insert("r").await?;
            Ok::<bool, Error>(duplicate.is_err())
        })
    })
    .await;
    s.outcome("continue_after_caught_raw_violation", raw);
    s.outcome("after_caught_raw_violation", stored(&["q", "r"]).await);
}

/// An `INSERT` of one name with the driver's placeholder.
fn raw_insert() -> Result<String, Error> {
    let sql = "INSERT INTO diff_ledger (name) VALUES (?)";
    Ok(if Orm::driver()? == "postgres" {
        rullst_orm::replace_placeholders(sql)
    } else {
        sql.to_string()
    })
}

/// How far the generated id moves past an insert that was rolled back.
async fn run_generated_ids(s: &mut Scenario<'_>) {
    let before = insert("n").await;
    let rolled_back = Orm::transaction(|_| {
        Box::pin(async {
            insert("o").await?;
            Err::<(), Error>(failure())
        })
    })
    .await;
    let after = insert("p").await;
    s.outcome("rolled_back_insert", rolled_back);
    s.outcome(
        "id_step_over_rolled_back_insert",
        before.and_then(|before| after.map(|after| after - before)),
    );
}
