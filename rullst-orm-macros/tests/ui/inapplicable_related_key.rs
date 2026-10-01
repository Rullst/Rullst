use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "users")]
struct User {
    id: i32,
    #[orm(has_many = "Post", foreign_key = "author_ref", related_key = "author_ref")]
    posts: Option<Vec<Post>>,
}

struct Post;

fn main() {}
