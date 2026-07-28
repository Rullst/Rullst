use rullst_orm::schema::Schema;
use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserPreferences {
    pub theme: String,
    pub notifications_enabled: bool,
}

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users_with_json")]
pub struct User {
    pub id: i32,
    pub name: String,

    // The magic JSON column!
    #[orm(json)]
    pub preferences: rullst_orm::Json<UserPreferences>,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    let _ = std::fs::remove_file("json_casting.db");
    std::fs::File::create("json_casting.db").unwrap();
    Orm::init("sqlite://json_casting.db").await?;

    // In SQLite, JSON is just a TEXT column
    Schema::create("users_with_json", |table| {
        table.id();
        table.string("name").not_null();
        table.string("preferences").not_null(); // A TEXT column to hold the JSON string
    })
    .await?;

    let mut user = User {
        id: 0,
        name: "Louis".to_string(),
        preferences: rullst_orm::Json(UserPreferences {
            theme: "dark".to_string(),
            notifications_enabled: true,
        }),
    };

    // It should automatically serialize the preferences struct to JSON string
    user.save().await?;
    println!("Saved User with ID: {}", user.id);

    // It should automatically deserialize the JSON string back into the struct
    let fetched_user = User::query()
        .where_eq("id", user.id)
        .first()
        .await?
        .unwrap();
    println!(
        "Fetched User Preferences Theme: {}",
        fetched_user.preferences.theme
    );
    println!(
        "Fetched User Notifications Enabled: {}",
        fetched_user.preferences.notifications_enabled
    );

    // Update test
    let mut to_update = fetched_user;
    to_update.preferences.theme = "light".to_string();
    to_update.save().await?;

    let updated_user = User::query()
        .where_eq("id", to_update.id)
        .first()
        .await?
        .unwrap();
    println!(
        "Updated User Preferences Theme: {}",
        updated_user.preferences.theme
    );

    Ok(())
}
