use rullst_orm::{Orm, RagContext};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Orm, rullst_orm::FromRow)]
#[orm(table = "documents")]
pub struct Document {
    pub id: i32,

    #[orm(rag_context)]
    pub title: String,

    #[orm(rag_context)]
    pub body: String,
    // We can't easily test pgvector + rullst_ai in a pure unit test without a DB and API key,
    // so we just test the parsing and trait generation for RagContext here.
}

#[test]
fn test_rag_context_generation() {
    let doc = Document {
        id: 1,
        title: "Hello World".to_string(),
        body: "This is a test document for RAG.".to_string(),
    };

    let context = doc.get_context();
    assert!(context.contains("title: Hello World"));
    assert!(context.contains("body: This is a test document for RAG."));
}
