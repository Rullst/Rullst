use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts", before_save = "Self::validate")]
struct Account {
    id: i32,
}

fn main() {}
