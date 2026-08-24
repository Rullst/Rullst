use rullst_orm::{Orm, with_tenant};

#[derive(Clone, Debug, Default, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "products", tenant_column = "tenant_id")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub tenant_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = std::fs::remove_file("test_tenant.db");
    std::fs::File::create("test_tenant.db").unwrap();
    Orm::init("sqlite:test_tenant.db").await?;
    let pool = Orm::pool()?;

    sqlx::query(
        "CREATE TABLE products (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, tenant_id TEXT)",
    )
    .execute(pool)
    .await?;

    let t1_id = "tenant_uuid_1".to_string();
    let t2_id = "tenant_uuid_2".to_string();

    println!("--- TENANT 1 CONTEXT ---");
    with_tenant(t1_id.clone(), async {
        let mut p = Product {
            name: "Apple iPhone".to_string(),
            ..Default::default()
        };
        p.save().await.unwrap();

        let products = Product::all().await.unwrap();
        println!(
            "Tenant 1 has {} products. First Product Tenant ID: {}",
            products.len(),
            products[0].tenant_id
        );
    })
    .await;

    println!("--- TENANT 2 CONTEXT ---");
    with_tenant(t2_id.clone(), async {
        let mut p = Product {
            name: "Samsung Galaxy".to_string(),
            ..Default::default()
        };
        p.save().await.unwrap();

        let products = Product::all().await.unwrap();
        println!(
            "Tenant 2 has {} products. First Product Tenant ID: {}",
            products.len(),
            products[0].tenant_id
        );
    })
    .await;

    // Cross-tenant verification:
    println!("--- OUTSIDE TENANT CONTEXT ---");
    let all_products = Product::all().await.unwrap();
    println!("Total products across all tenants: {}", all_products.len());

    Ok(())
}
