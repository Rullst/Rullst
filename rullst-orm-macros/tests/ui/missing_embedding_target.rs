use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "articles")]
struct Article {
    id: i32,
    #[orm(embedding_for = "body")]
    embedding: Option<Vec<f32>>,
}

fn main() {}
