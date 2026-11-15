use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "profiles")]
struct Profile {
    id: i32,
    name: String,
    xp: i32,
}

fn main() {}
