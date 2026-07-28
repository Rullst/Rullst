use rullst_orm::{FromRow, Orm};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
}

// Global atomic counters to track observer callback executions
static SAVING_COUNT: AtomicUsize = AtomicUsize::new(0);
static SAVED_COUNT: AtomicUsize = AtomicUsize::new(0);
static CREATING_COUNT: AtomicUsize = AtomicUsize::new(0);
static CREATED_COUNT: AtomicUsize = AtomicUsize::new(0);
static UPDATING_COUNT: AtomicUsize = AtomicUsize::new(0);
static UPDATED_COUNT: AtomicUsize = AtomicUsize::new(0);
static DELETING_COUNT: AtomicUsize = AtomicUsize::new(0);
static DELETED_COUNT: AtomicUsize = AtomicUsize::new(0);

pub struct UserObserverImpl;

#[rullst_orm::async_trait]
impl UserObserver for UserObserverImpl {
    async fn saving(&self, model: &mut User) -> Result<(), rullst_orm::Error> {
        println!("Observer -> saving() called for: {}", model.name);
        SAVING_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn saved(&self, model: &User) -> Result<(), rullst_orm::Error> {
        println!("Observer -> saved() called for: {}", model.name);
        SAVED_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn creating(&self, model: &mut User) -> Result<(), rullst_orm::Error> {
        println!("Observer -> creating() called for: {}", model.name);
        CREATING_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn created(&self, model: &User) -> Result<(), rullst_orm::Error> {
        println!("Observer -> created() called for: {}", model.name);
        CREATED_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn updating(&self, model: &mut User) -> Result<(), rullst_orm::Error> {
        println!("Observer -> updating() called for: {}", model.name);
        UPDATING_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn updated(&self, model: &User) -> Result<(), rullst_orm::Error> {
        println!("Observer -> updated() called for: {}", model.name);
        UPDATED_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn deleting(&self, model: &User) -> Result<(), rullst_orm::Error> {
        println!("Observer -> deleting() called for: {}", model.name);
        DELETING_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn deleted(&self, model: &User) -> Result<(), rullst_orm::Error> {
        println!("Observer -> deleted() called for: {}", model.name);
        DELETED_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    let _ = std::fs::remove_file("test.db");
    std::fs::File::create("test.db").unwrap();
    Orm::init("sqlite://test.db").await?;
    let pool = Orm::pool();

    rullst_orm::_sqlx::query(
        "
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        );
    ",
    )
    .execute(pool)
    .await?;

    // 1. Register the Observer
    println!("Registering UserObserverImpl...");
    User::observe(Arc::new(UserObserverImpl));

    // 2. Perform Insert (Saving + Creating + Created + Saved should trigger)
    println!("\n--- Performing INSERT ---");
    let mut u = User {
        id: 0,
        name: "Observer User".to_string(),
    };
    u.save().await?;

    // Retrieve saved user for update
    let mut saved_user = User::query().first().await?.unwrap();

    // 3. Perform Update (Saving + Updating + Updated + Saved should trigger)
    println!("\n--- Performing UPDATE ---");
    saved_user.name = "Updated Observer User".to_string();
    saved_user.save().await?;

    // 4. Perform Delete (Deleting + Deleted should trigger)
    println!("\n--- Performing DELETE ---");
    saved_user.delete().await?;

    // 5. Verification
    println!("\n--- Verification Report ---");
    println!("Saving:   {}", SAVING_COUNT.load(Ordering::SeqCst));
    println!("Saved:    {}", SAVED_COUNT.load(Ordering::SeqCst));
    println!("Creating: {}", CREATING_COUNT.load(Ordering::SeqCst));
    println!("Created:  {}", CREATED_COUNT.load(Ordering::SeqCst));
    println!("Updating: {}", UPDATING_COUNT.load(Ordering::SeqCst));
    println!("Updated:  {}", UPDATED_COUNT.load(Ordering::SeqCst));
    println!("Deleting: {}", DELETING_COUNT.load(Ordering::SeqCst));
    println!("Deleted:  {}", DELETED_COUNT.load(Ordering::SeqCst));

    assert_eq!(SAVING_COUNT.load(Ordering::SeqCst), 2);
    assert_eq!(SAVED_COUNT.load(Ordering::SeqCst), 2);
    assert_eq!(CREATING_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(CREATED_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(UPDATING_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(UPDATED_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(DELETING_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(DELETED_COUNT.load(Ordering::SeqCst), 1);

    println!("\n🎉 All 8 Observer lifecycle hooks executed successfully!");

    // Clean up
    let _ = std::fs::remove_file("test.db");

    Ok(())
}
