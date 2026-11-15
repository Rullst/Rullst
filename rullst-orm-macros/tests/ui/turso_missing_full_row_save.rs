use rullst_orm_macros::TursoModel;

#[derive(TursoModel)]
#[orm(table = "notes")]
struct Note {
    id: i64,
    title: String,
}

fn main() {}
