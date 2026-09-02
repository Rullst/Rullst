use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts")]
struct Account {
    name: String,
}

fn main() {}
