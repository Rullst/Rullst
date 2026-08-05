# Tutorial 22: RAG Systems & Vector Search 🧠

Build a Retrieval-Augmented Generation (RAG) system with semantic vector embeddings using `rullst-ai`.

---

## 🛠️ Step 1: Generate Vector Embeddings

```rust
use rullst_ai::rag::{EmbeddingModel, VectorStore};

pub async fn index_documentation(docs: Vec<String>) -> Result<(), rullst_core::AppError> {
    let mut store = VectorStore::memory();
    let embedder = EmbeddingModel::default();

    for doc in docs {
        let vector = embedder.embed(&doc).await?;
        store.insert(vector, doc);
    }
    
    Ok(())
}
```

---

## 💻 Step 2: Query Semantic Vector Search

```rust
pub async fn answer_user_query(query: &str) -> Result<String, rullst_core::AppError> {
    let store = VectorStore::global();
    let matches = store.similarity_search(query, 3).await?;
    
    let context = matches.join("\n");
    let prompt = format!("Context:\n{}\n\nQuestion: {}", context, query);
    
    rullst_ai::generate_completion(&prompt).await
}
```

---

## 💡 Key Takeaways
- RAG allows AI agents to answer domain-specific questions accurately without hallucinations.
- Supports in-memory vector indexing and pgvector storage.
