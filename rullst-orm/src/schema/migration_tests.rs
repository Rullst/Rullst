#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{Migration, pending_before_seed, run_artisan_with_args};
use crate::Error;
use std::collections::HashSet;

struct Named(&'static str);

#[async_trait::async_trait]
impl Migration for Named {
    fn name(&self) -> &'static str {
        self.0
    }

    async fn up(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn down(&self) -> Result<(), Error> {
        Ok(())
    }
}

fn registry(names: &[&'static str]) -> Vec<Box<dyn Migration>> {
    names
        .iter()
        .map(|name| Box::new(Named(name)) as Box<dyn Migration>)
        .collect()
}

fn executed(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[test]
fn seeding_waits_for_every_registered_migration() {
    let migrations = registry(&["m1_users", "m2_posts", "m3_tags", "m4_likes"]);
    let error = pending_before_seed(&migrations, &executed(&["m1_users"])).unwrap_err();
    assert!(
        matches!(&error, Error::Validation(message) if message
            == "3 migration(s) pending, starting with `m2_posts`: run `db:migrate` before `db:seed`"),
        "{error:?}"
    );
    let error = pending_before_seed(&migrations, &executed(&[])).unwrap_err();
    assert_eq!(
        error.to_string(),
        "Validation error: 4 migration(s) pending, starting with `m1_users`: run `db:migrate` before `db:seed`"
    );
}

#[test]
fn seeding_proceeds_once_everything_is_migrated() {
    let migrations = registry(&["m1_users", "m2_posts"]);
    assert!(pending_before_seed(&migrations, &executed(&["m1_users", "m2_posts"])).is_ok());
    // A recorded migration the binary no longer registers does not block.
    assert!(
        pending_before_seed(&migrations, &executed(&["m0_old", "m1_users", "m2_posts"])).is_ok()
    );
    assert!(pending_before_seed(&[], &executed(&[])).is_ok());
}

// The library's tests never initialize the ORM.
#[tokio::test]
async fn seeding_with_registered_migrations_needs_the_database() {
    let seeded = run_artisan_with_args(
        &["artisan".to_string(), "db:seed".to_string()],
        registry(&["m1_users"]),
        Vec::new(),
    )
    .await;
    assert!(matches!(seeded, Err(Error::NotInitialized)), "{seeded:?}");
}
