use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts", table_name = "other_accounts")]
struct Account {
    id: i32,
}

fn main() {}
