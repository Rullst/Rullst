use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[tokio::main]
async fn main() -> Result<(), rullst_orm::Error> {
    // Para testar corretamente com o Pool do SQLX, precisamos de um arquivo físico
    let _ = std::fs::File::create("test.db");
    Orm::init("sqlite://test.db").await?;
    let pool = Orm::pool();

    rullst_orm::_sqlx::query(
        "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, email TEXT)",
    )
    .execute(pool)
    .await?;

    rullst_orm::_sqlx::query(
        "INSERT INTO users (name, email) VALUES ('Vene Louis', 'vene@cosmos.com')",
    )
    .execute(pool)
    .await?;
    rullst_orm::_sqlx::query(
        "INSERT INTO users (name, email) VALUES ('John Doe', 'john@example.com')",
    )
    .execute(pool)
    .await?;
    rullst_orm::_sqlx::query(
        "INSERT INTO users (name, email) VALUES ('Maria Doe', 'maria@example.com')",
    )
    .execute(pool)
    .await?;

    println!("\n🚀 Testando o Query Builder Encadeável:");

    let users = User::query()
        .where_like("email", "%@example.com")
        .order_by_name_desc()
        .limit(1)
        .get()
        .await?;

    println!("=> Último usuário da example.com: {:?}", users);

    let count = User::query().count().await?;
    println!("=> Total de usuários na tabela: {}", count);

    // Filter using dynamic generic inputs (i32) and magic methods
    let filtered_user = User::query().where_id(1).first().await?;
    println!("=> User via builder mágico onde id=1: {:?}", filtered_user);

    // Limpeza
    let _ = std::fs::remove_file("test.db");

    Ok(())
}
