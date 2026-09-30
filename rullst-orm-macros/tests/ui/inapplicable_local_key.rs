use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "posts")]
struct Post {
    id: i32,
    author_ref: i32,
    #[orm(belongs_to = "User", foreign_key = "author_ref", local_key = "legacy_id")]
    author: Option<User>,
}

struct User;

fn main() {}
