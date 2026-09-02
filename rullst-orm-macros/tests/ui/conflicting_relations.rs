use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts")]
struct Account {
    id: i32,
    #[orm(has_many = "Post", has_one = "Post")]
    posts: Vec<Post>,
}

struct Post;

fn main() {}
