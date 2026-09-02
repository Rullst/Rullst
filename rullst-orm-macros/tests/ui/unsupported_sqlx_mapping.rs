use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts")]
struct Account {
    id: i32,
    #[sqlx(rename = "display_name")]
    name: String,
}

fn main() {}
