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

    // A combination the guardrail still blocks is context, not generation.
    let (result, outcome, prompts) = answer_with([
        "The logo is ![logo][site-logo] in the header.",
        "See https://docs.rs for details.",
    ])
    .await;
    assert!(matches!(
        result,
        Err(RagError::Generation(AiError::BlockedByFirewall(code))) if code == "data_exfiltration"
    ));
    assert_eq!(outcome, RagAuditOutcome::ContextRejected);
    assert_eq!(prompts, 0);
}
