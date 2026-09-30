use super::*;
use std::sync::Mutex;

struct RefundTool;

impl AiTool for RefundTool {
    fn name(&self) -> &str {
        "issue_refund"
    }

    fn description(&self) -> &str {
        "Issue a refund"
    }

    fn parameters(&self) -> Vec<ToolParam> {
        vec![ToolParam {
            name: "amount".to_string(),
            param_type: "number".to_string(),
            description: "Refund amount".to_string(),
            required: true,
        }]
    }

    fn risk(&self) -> ToolRisk {
        ToolRisk::Financial
    }

    fn execute(&self, payload: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(payload)
    }
}

/// Rejects the first record and accepts every later one.
#[derive(Default)]
struct AuditRejectingFirst {
    rejected: Mutex<bool>,
}

impl ToolAuditSink for AuditRejectingFirst {
    fn record(&self, _event: ToolAuditEvent) -> Result<(), ToolExecutionError> {
        let mut rejected = self
            .rejected
            .lock()
            .map_err(|_| ToolExecutionError::AuditUnavailable("lock poisoned".to_string()))?;
        if *rejected {
            return Ok(());
        }
        *rejected = true;
        Err(ToolExecutionError::AuditUnavailable(
            "audit file is at its byte quota".to_string(),
        ))
    }
}

#[test]
fn unaudited_authorization_keeps_the_approval_for_a_retry() {
    let mut registry = ToolRegistry::new();
    registry.register(RefundTool).expect("valid financial tool");
    let policy = ToolExecutionPolicy::new(["issue_refund"]).expect("valid policy");
    let mut context = ToolExecutionContext::new("finance-user", ["issue_refund"], 2)
        .expect("valid authorization");
    let payload = serde_json::json!({"amount": 15.0});
    context.approve(
        HumanApproval::for_payload("issue_refund", &payload, "reviewer-9", "ticket FIN-42")
            .expect("valid approval"),
    );
    let audit = AuditRejectingFirst::default();

    assert!(matches!(
        registry.execute(
            "issue_refund",
            payload.clone(),
            &mut context,
            &policy,
            &audit
        ),
        Err(ToolExecutionError::AuditUnavailable(_))
    ));
    assert_eq!(context.remaining_calls(), 2);
    registry
        .execute(
            "issue_refund",
            payload.clone(),
            &mut context,
            &policy,
            &audit,
        )
        .expect("the retained approval authorizes the retry");
    assert!(matches!(
        registry.execute("issue_refund", payload, &mut context, &policy, &audit),
        Err(ToolExecutionError::HumanApprovalRequired { .. })
    ));
}
