use rullst_orm_macros::Orm;

#[derive(Default)]
struct RowSnapshot;

#[derive(Orm)]
#[orm(table = "profiles", full_row_save)]
struct Profile {
    id: i32,
    name: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

fn main() {}
