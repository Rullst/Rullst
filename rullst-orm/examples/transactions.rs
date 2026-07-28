use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "accounts")]
pub struct Account {
    pub id: i32,
    pub owner: String,
    pub balance: i32,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    // 1. Initialize DB
    let _ = std::fs::remove_file("transactions_test.db");
    std::fs::File::create("transactions_test.db").unwrap();
    Orm::init("sqlite://transactions_test.db").await?;

    // 2. Schema
    Schema::create("accounts", |table: &mut Blueprint| {
        table.id();
        table.string("owner").not_null();
        table.integer("balance").not_null();
    })
    .await?;

    // 3. Create initial accounts
    let mut a = Account {
        id: 0,
        owner: "Alice".to_string(),
        balance: 100,
    };
    a.save().await?;
    let mut b = Account {
        id: 0,
        owner: "Bob".to_string(),
        balance: 100,
    };
    b.save().await?;

    println!("--- Initial Balances ---");
    println!("Alice: ${}", Account::find(1).await?.unwrap().balance);
    println!("Bob: ${}", Account::find(2).await?.unwrap().balance);

    // ---------------------------------------------------------
    // Scenario 1: A Successful Transaction (Transfer $50 from Alice to Bob)
    // ---------------------------------------------------------
    println!("\n--- Attempting Successful Transfer ($50) ---");
    let mut tx1 = Orm::begin_transaction().await?;

    // Using `get_with_tx` and `save_with_tx`
    let mut alice = Account::query()
        .where_eq("owner", "Alice")
        .first_with_tx(&mut tx1)
        .await?
        .unwrap();
    let mut bob = Account::query()
        .where_eq("owner", "Bob")
        .first_with_tx(&mut tx1)
        .await?
        .unwrap();

    alice.balance -= 50;
    bob.balance += 50;

    alice.save_with_tx(&mut tx1).await?;
    bob.save_with_tx(&mut tx1).await?;

    tx1.commit().await?; // COMMITTED!
    println!("Transfer committed successfully.");

    // ---------------------------------------------------------
    // Scenario 2: A Failed Transaction (Transfer $200 from Alice to Bob, but Alice doesn't have enough)
    // ---------------------------------------------------------
    println!("\n--- Attempting Failed Transfer ($200) ---");
    let mut tx2 = Orm::begin_transaction().await?;

    let mut alice2 = Account::query()
        .where_eq("owner", "Alice")
        .first_with_tx(&mut tx2)
        .await?
        .unwrap();
    let mut bob2 = Account::query()
        .where_eq("owner", "Bob")
        .first_with_tx(&mut tx2)
        .await?
        .unwrap();

    alice2.balance -= 200; // Oops, this is invalid logically!
    bob2.balance += 200;

    // We save the corrupted state to the transaction
    alice2.save_with_tx(&mut tx2).await?;
    bob2.save_with_tx(&mut tx2).await?;

    // Wait, let's simulate an error causing a rollback!
    println!("Simulating an error... Rolling back!");
    tx2.rollback().await?; // ROLLED BACK!

    // ---------------------------------------------------------
    // Final Verification
    // ---------------------------------------------------------
    println!("\n--- Final Balances (Expected: Alice $50, Bob $150) ---");
    println!("Alice: ${}", Account::find(1).await?.unwrap().balance);
    println!("Bob: ${}", Account::find(2).await?.unwrap().balance);

    Ok(())
}
