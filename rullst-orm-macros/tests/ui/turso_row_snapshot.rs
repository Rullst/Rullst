use rullst_orm_macros::Orm;

#[derive(Default)]
struct RowSnapshot;

#[derive(Orm)]
#[orm(table = "notes", backend = "turso", full_row_save)]
struct Note {
    id: i64,
    title: String,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

fn main() {}
