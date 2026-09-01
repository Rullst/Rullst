use rullst_orm_macros::Orm;

struct Post;

#[derive(Orm)]
#[orm(table = "accounts", backend = "turso")]
struct Account {
    id: i64,
    #[orm(has_many = "Post")]
    posts: Vec<Post>,
}

fn main() {}
