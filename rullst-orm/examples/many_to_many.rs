use rullst_orm::schema::Schema;
use rullst_orm::{FromRow, Orm, RullstModel};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "roles")]
pub struct Role {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,

    // The many-to-many relationship!
    #[orm(belongs_to_many = "Role", pivot_table = "role_user")]
    #[sqlx(skip)]
    pub roles: Option<Vec<Role>>,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    let _ = std::fs::remove_file("manytomany.db");
    std::fs::File::create("manytomany.db").unwrap();
    Orm::init("sqlite://manytomany.db").await?;

    Schema::create("users", |table| {
        table.id();
        table.string("name").not_null();
    })
    .await?;

    Schema::create("roles", |table| {
        table.id();
        table.string("name").not_null();
    })
    .await?;

    Schema::create("role_user", |table| {
        table.integer("user_id").not_null();
        table.integer("role_id").not_null();
    })
    .await?;

    // Create roles
    let mut admin_role = Role {
        id: 0,
        name: "Admin".to_string(),
    };
    admin_role.save().await?;

    let mut editor_role = Role {
        id: 0,
        name: "Editor".to_string(),
    };
    editor_role.save().await?;

    let mut viewer_role = Role {
        id: 0,
        name: "Viewer".to_string(),
    };
    viewer_role.save().await?;

    // Create users
    let mut user1 = User {
        id: 0,
        name: "Alice (Admin & Editor)".to_string(),
        roles: None,
    };
    user1.save().await?;

    let mut user2 = User {
        id: 0,
        name: "Bob (Viewer)".to_string(),
        roles: None,
    };
    user2.save().await?;

    // Attach roles to users in the pivot table!
    rullst_orm::_sqlx::query("INSERT INTO role_user (user_id, role_id) VALUES (?, ?)")
        .bind(user1.id)
        .bind(admin_role.id)
        .execute(Orm::pool())
        .await?;

    rullst_orm::_sqlx::query("INSERT INTO role_user (user_id, role_id) VALUES (?, ?)")
        .bind(user1.id)
        .bind(editor_role.id)
        .execute(Orm::pool())
        .await?;

    rullst_orm::_sqlx::query("INSERT INTO role_user (user_id, role_id) VALUES (?, ?)")
        .bind(user2.id)
        .bind(viewer_role.id)
        .execute(Orm::pool())
        .await?;

    // Eager Load test
    println!("Fetching users with their roles...");
    let users = User::query().with_roles().get().await?;

    for user in users {
        println!("User: {}", user.name);
        for role in user.roles.unwrap() {
            println!("  - Role: {}", role.name);
        }
    }

    Ok(())
}
