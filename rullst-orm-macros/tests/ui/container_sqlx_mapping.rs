use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts")]
#[sqlx(rename_all = "camelCase")]
struct Account {
    id: i32,
    display_name: String,
}

fn main() {}
