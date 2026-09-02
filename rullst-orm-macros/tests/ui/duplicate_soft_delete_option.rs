use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(
    table = "accounts",
    soft_delete(field = "deleted_at", column = "removed_at")
)]
struct Account {
    id: i32,
    deleted_at: Option<String>,
    removed_at: Option<String>,
}

fn main() {}
