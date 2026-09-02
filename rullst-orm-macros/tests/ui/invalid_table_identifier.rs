use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts; DROP TABLE users")]
struct Account {
    id: i32,
}

fn main() {}
