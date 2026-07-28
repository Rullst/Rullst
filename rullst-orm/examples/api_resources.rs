use rullst_orm::{ApiResource, JsonResource, RullstCollection};

#[derive(Clone, Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub is_admin: bool,
}

impl ApiResource for User {
    fn to_array(&self) -> serde_json::Value {
        serde_json::json!({
            "user_id": self.id,
            "full_name": self.name.to_uppercase(),
            "contact": self.email,
            "role": if self.is_admin { "administrator" } else { "user" }
            // Note: password_hash is intentionally hidden!
        })
    }
}

fn main() {
    let user1 = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        password_hash: "secret123".to_string(),
        is_admin: true,
    };

    let user2 = User {
        id: 2,
        name: "Jane Smith".to_string(),
        email: "jane@example.com".to_string(),
        password_hash: "hidden456".to_string(),
        is_admin: false,
    };

    // Transform a single model
    let json_single = JsonResource::new(&user1).resolve();
    println!("--- Single Resource ---");
    println!("{}", serde_json::to_string_pretty(&json_single).unwrap());

    // Transform a collection
    let users = vec![user1, user2];
    let json_collection = users.collection_resource();
    println!("\n--- Collection Resource ---");
    println!(
        "{}",
        serde_json::to_string_pretty(&json_collection).unwrap()
    );
}
