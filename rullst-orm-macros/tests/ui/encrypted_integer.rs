use rullst_orm_macros::Orm;

#[derive(Orm)]
struct Account {
    id: i32,
    #[orm(encrypted)]
    secret: i64,
}

fn main() {}
