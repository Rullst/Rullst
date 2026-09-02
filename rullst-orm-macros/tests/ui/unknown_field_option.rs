use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts")]
struct Account {
    id: i32,
    #[orm(mystery)]
    name: String,
}

fn main() {}
