use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts")]
struct Account {
    id: i32,
    #[orm(foreign_key = "account_id")]
    note: String,
}

fn main() {}
