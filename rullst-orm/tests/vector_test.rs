use rullst_orm::Orm;
use rullst_orm::schema::Blueprint;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Clone, Debug, Serialize, Deserialize, Orm, FromRow)]
pub struct Document {
    pub id: i32,
    pub content: String,
}

#[tokio::test]
async fn test_vector_schema_and_query() {
    // Initialize ORM
    Orm::init_with_options("sqlite:file:vector_test_db?mode=memory&cache=shared", 5, 30)
        .await
        .unwrap();

    // 1. Test Schema Builder generates correct VECTOR(1536) type
    let mut schema = Blueprint::new();
    schema.id();
    schema.string("content");
    schema.vector("embedding", 1536);

    let sql = schema.build().unwrap();
    assert!(sql.contains("embedding VECTOR(1536)"));

    // 2. Test QueryBuilder generates correct pgvector Order clauses
    let query = Document::query()
        .order_by_l2_distance("embedding", vec![0.1, 0.2, 0.3])
        .to_sql();
    assert!(query.contains("ORDER BY embedding <-> '[0.1,0.2,0.3]'"));

    let query_cosine = Document::query()
        .order_by_cosine_distance("embedding", vec![0.1, 0.2, 0.3])
        .to_sql();
    assert!(query_cosine.contains("ORDER BY embedding <=> '[0.1,0.2,0.3]'"));

    let query_inner = Document::query()
        .order_by_inner_product("embedding", vec![0.1, 0.2, 0.3])
        .to_sql();
    assert!(query_inner.contains("ORDER BY embedding <#> '[0.1,0.2,0.3]'"));
}
