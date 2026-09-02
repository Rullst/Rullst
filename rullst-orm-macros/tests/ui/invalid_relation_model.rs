use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "accounts")]
struct Account {
    id: i32,
    #[orm(has_many = "Post<Vec>")]
    posts: Vec<Post>,
}

struct Post;

fn main() {}
