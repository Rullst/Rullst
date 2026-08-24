#![allow(clippy::unwrap_used, clippy::expect_used)]

use rullst_orm::collection::RullstCollection;
use rullst_orm::error::RullstError;
use rullst_orm::resource::ApiResource;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
struct Product {
    id: i32,
    name: String,
    price: f64,
    category: String,
}

impl ApiResource for Product {
    fn to_array(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "price": self.price,
            "category": self.category,
        })
    }
}

#[test]
fn test_rullst_collection_methods() {
    let products = vec![
        Product {
            id: 1,
            name: "Laptop".into(),
            price: 1200.0,
            category: "Electronics".into(),
        },
        Product {
            id: 2,
            name: "Mouse".into(),
            price: 25.0,
            category: "Electronics".into(),
        },
        Product {
            id: 3,
            name: "Chair".into(),
            price: 150.0,
            category: "Furniture".into(),
        },
    ];

    // 1. key_by
    let keyed: HashMap<i32, Product> = products.clone().key_by(|p| p.id);
    assert_eq!(keyed.len(), 3);
    assert_eq!(keyed.get(&1).unwrap().name, "Laptop");

    // 2. map
    let names: Vec<String> = products.clone().map(|p| p.name);
    assert_eq!(names, vec!["Laptop", "Mouse", "Chair"]);

    // 3. filter
    let electronics: Vec<Product> = products.clone().filter(|p| p.category == "Electronics");
    assert_eq!(electronics.len(), 2);

    // 4. chunk
    let chunks = products.clone().chunk(2);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].len(), 2);
    assert_eq!(chunks[1].len(), 1);

    // 5. implode
    let names_joined = products.implode(", ", |p| p.name.clone());
    assert_eq!(names_joined, "Laptop, Mouse, Chair");

    // 6. sum_by
    let total_price: f64 = products.sum_by(|p| p.price);
    assert_eq!(total_price, 1375.0);

    // 7. max_by_key & min_by_key
    let most_expensive = products.max_by_key(|p| (p.price * 100.0) as i64);
    assert_eq!(most_expensive.unwrap().name, "Laptop");

    let cheapest = products.min_by_key(|p| (p.price * 100.0) as i64);
    assert_eq!(cheapest.unwrap().name, "Mouse");

    // 8. collection_resource
    let resource_json = products.collection_resource();
    assert!(resource_json.is_array());
    assert_eq!(resource_json.as_array().unwrap().len(), 3);
}

#[test]
fn test_orm_error_conversions_and_display() {
    let not_found = RullstError::RecordNotFound;
    assert_eq!(format!("{}", not_found), "Record not found");

    let db_err = RullstError::DatabaseError("Connection timeout".into());
    assert_eq!(format!("{}", db_err), "Database error: Connection timeout");

    let val_err = RullstError::Validation("Invalid SQL identifier".into());
    assert_eq!(
        format!("{}", val_err),
        "Validation error: Invalid SQL identifier"
    );

    let ser_err = RullstError::SerializationError("JSON decode failure".into());
    assert_eq!(
        format!("{}", ser_err),
        "Serialization error: JSON decode failure"
    );
}
