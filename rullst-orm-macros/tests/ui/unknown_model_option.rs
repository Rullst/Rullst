use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts", mystery)]
struct Account {
    id: i32,
}

fn main() {}
