use super::*;
use crate::ai::rag::InMemoryRagAuditTrail;
use crate::ai::{AiProvider, Message};
use std::sync::{Arc, Mutex};

/// Records how many generation requests reached the provider.
struct CountingProvider {
    prompts: Arc<Mutex<usize>>,
}

#[async_trait]
impl AiProvider for CountingProvider {
    async fn prompt(&self, _text: &str) -> Result<String, AiError> {
        *self.prompts.lock().expect("prompt count") += 1;
        Ok("grounded answer".to_string())
    }

    async fn chat(&self, _messages: &[Message]) -> Result<String, AiError> {
        Ok("unused".to_string())
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>, AiError> {
        Ok(vec![1.0, 0.0])
    }
}

struct FixtureRetriever {
    documents: Vec<RagDocument>,
}

#[async_trait]
impl RagRetriever for FixtureRetriever {
    async fn retrieve(
        &self,
        _tenant: &TenantContext,
        _query_embedding: &[f32],
        _limit: usize,
    ) -> Result<Vec<RagDocument>, RagRetrievalError> {
        Ok(self.documents.clone())
    }
}

async fn answer_with(contexts: [&str; 2]) -> (Result<RagAnswer, RagError>, RagAuditOutcome, usize) {
    let tenant = TenantContext::try_new("tenant:docs").expect("tenant");
    let documents = contexts
        .iter()
        .enumerate()
        .map(|(index, content)| {
            RagDocument::try_new(&tenant, format!("doc-{index}"), *content, 1.0).expect("doc")
        })
        .collect();
    let prompts = Arc::new(Mutex::new(0));
    let audit = Arc::new(InMemoryRagAuditTrail::new(4).expect("audit"));
    let pipeline = RagPipeline::new(
        AiClient::new(CountingProvider {
            prompts: Arc::clone(&prompts),
        }),
        FixtureRetriever { documents },
        Arc::clone(&audit),
    );
    let result = pipeline.answer(&tenant, "How do I add a logo?").await;
    let outcome = audit.entries().expect("audit")[0].event.outcome;
    let prompts = *prompts.lock().expect("prompt count");
    (result, outcome, prompts)
}

#[tokio::test]
async fn separately_safe_documents_are_judged_together_without_false_beacons() {
    // A relative image in one document and a link in another are not a beacon.
    let (result, outcome, prompts) = answer_with([
        "![logo](assets/logo.png) sits in the header.",
        "See https://docs.rs for details.",
    ])
    .await;
    assert_eq!(result.expect("answer").answer(), "grounded answer");
    assert_eq!(outcome, RagAuditOutcome::Succeeded);
    assert_eq!(prompts, 1);

    // An undefined reference is literal text next to an unrelated link.
    let (result, _, _) = answer_with([
        "The logo is ![logo][site-logo] in the header.",
        "See https://docs.rs for details.",
    ])
    .await;
    assert_eq!(result.expect("answer").answer(), "grounded answer");

    // A combination the guardrail still blocks is context, not generation:
    // each passage is safe alone, but together they define a remote image.
    let (result, outcome, prompts) = answer_with([
        "The logo is ![logo][site-logo] in the header.",
        "[site-logo]: https://example.invalid/logo.png",
    ])
    .await;
    assert!(matches!(
        result,
        Err(RagError::Generation(AiError::BlockedByFirewall(code))) if code == "data_exfiltration"
    ));
    assert_eq!(outcome, RagAuditOutcome::ContextRejected);
    assert_eq!(prompts, 0);
}

#[tokio::test]
async fn a_foreign_document_after_the_exhausted_budget_is_still_rejected() {
    let tenant = TenantContext::try_new("tenant:docs").expect("tenant");
    let foreign = TenantContext::try_new("tenant:other").expect("foreign tenant");
    // The first passage alone exhausts the five-character context budget, so
    // the loop used to stop at the next document before reaching doc-2.
    let documents = vec![
        RagDocument::try_new(&tenant, "doc-0", "first passage", 1.0).expect("doc"),
        RagDocument::try_new(&tenant, "doc-1", "second passage", 0.8).expect("doc"),
        RagDocument::try_new(&foreign, "doc-2", "foreign passage", 0.5).expect("doc"),
    ];
    let prompts = Arc::new(Mutex::new(0));
    let audit = Arc::new(InMemoryRagAuditTrail::new(4).expect("audit"));
    let pipeline = RagPipeline::new(
        AiClient::new(CountingProvider {
            prompts: Arc::clone(&prompts),
        }),
        FixtureRetriever { documents },
        Arc::clone(&audit),
    )
    .with_config(RagConfig::try_new(3, 64, 5).expect("config"));
    let result = pipeline.answer(&tenant, "How do I add a logo?").await;
    assert!(
        matches!(&result, Err(RagError::InvalidDocument(message)) if message.contains("doc-2")),
        "unexpected result: {result:?}"
    );
    assert_eq!(
        audit.entries().expect("audit")[0].event.outcome,
        RagAuditOutcome::ContextRejected
    );
    assert_eq!(*prompts.lock().expect("prompt count"), 0);
}
