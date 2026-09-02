use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "posts")]
struct Post {
    id: i32,
    user_id: i32,
    #[orm(belongs_to = "User", foreign_key = "user_id", cascade_soft_delete)]
    user: Option<User>,
}

struct User;

fn main() {}
