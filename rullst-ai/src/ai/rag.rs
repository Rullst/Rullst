//! Utilities for building Retrieval-Augmented Generation (RAG) prompts.

/// Builds a structured prompt for an LLM containing context documents and a question.
///
/// This is meant to be used alongside `rullst-orm` native vector search and `RagContext` trait.
///
/// # Example
/// ```rust
/// use rullst_ai::ai::rag::build_rag_prompt;
///
/// let contexts = vec![
///     "Rullst is a web framework for Rust.".to_string(),
///     "It includes an ORM and AI integrations natively.".to_string()
/// ];
///
/// let prompt = build_rag_prompt("What is Rullst?", &contexts);
/// // Now send this prompt to `AiClient::prompt()`
/// ```
pub fn build_rag_prompt(question: &str, contexts: &[String]) -> String {
    let mut prompt = String::new();
    prompt.push_str("Use the following pieces of context to answer the user's question.\n");
    prompt.push_str("If you don't know the answer based on the context, just say that you don't know, don't try to make up an answer.\n\n");

    for (i, ctx) in contexts.iter().enumerate() {
        prompt.push_str(&format!("--- Context {} ---\n{}\n\n", i + 1, ctx));
    }

    prompt.push_str(&format!("Question: {}\n", question));
    prompt.push_str("Answer:");
    prompt
}
