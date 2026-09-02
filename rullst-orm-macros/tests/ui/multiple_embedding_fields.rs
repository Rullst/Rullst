use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "articles")]
struct Article {
    id: i32,
    body: String,
    #[orm(embedding_for = "body")]
    primary_embedding: Option<Vec<f32>>,
    #[orm(embedding_for = "body")]
    secondary_embedding: Option<Vec<f32>>,
}

fn main() {}
