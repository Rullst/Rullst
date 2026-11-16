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

fn audit_event(principal: &str, tool: &str, approval: Option<(&str, &str)>) -> ToolAuditEvent {
    ToolAuditEvent {
        principal: principal.to_string(),
        tool: tool.to_string(),
        risk: Some(ToolRisk::Financial),
        approved_by: approval.map(|(approver, _)| approver.to_string()),
        approval_reason: approval.map(|(_, reason)| reason.to_string()),
        outcome: ToolAuditOutcome::Succeeded,
    }
}

#[test]
fn durable_tool_audit_events_bound_principal_tool_and_approval() {
    use crate::ai::durable_audit::DurableAuditRecord;

    let principal_256 = "p".repeat(256);
    let principal_257 = "p".repeat(257);
    assert_eq!(
        audit_event(&principal_256, "issue_refund", None).validate(),
        Ok(())
    );
    for principal in ["", "   ", principal_257.as_str()] {
        assert_eq!(
            audit_event(principal, "issue_refund", None).validate(),
            Err("event has an invalid principal")
        );
    }

    let tool_64 = "t".repeat(64);
    let tool_65 = "t".repeat(65);
    assert_eq!(audit_event("user-1", &tool_64, None).validate(), Ok(()));
    for tool in ["", tool_65.as_str(), "issue refund"] {
        assert_eq!(
            audit_event("user-1", tool, None).validate(),
            Err("event has an invalid tool identifier")
        );
    }

    let approver_256 = "a".repeat(256);
    let approver_257 = "a".repeat(257);
    let reason_2048 = "r".repeat(2_048);
    let reason_2049 = "r".repeat(2_049);
    for approval in [
        ("reviewer-9", "ticket FIN-42"),
        (approver_256.as_str(), reason_2048.as_str()),
    ] {
        assert_eq!(
            audit_event("user-1", "issue_refund", Some(approval)).validate(),
            Ok(())
        );
    }
    for approval in [
        ("   ", "ticket FIN-42"),
        ("reviewer-9", "   "),
        (approver_257.as_str(), "ticket FIN-42"),
        ("reviewer-9", reason_2049.as_str()),
    ] {
        assert_eq!(
            audit_event("user-1", "issue_refund", Some(approval)).validate(),
            Err("event has invalid approval evidence")
        );
    }
    let mut half_approval = audit_event("user-1", "issue_refund", None);
    half_approval.approved_by = Some("reviewer-9".to_string());
    assert_eq!(
        half_approval.validate(),
        Err("event has invalid approval evidence")
    );

    // Only a denial may omit the tool's risk.
    let mut denied = audit_event("user-1", "issue_refund", None);
    denied.risk = None;
    denied.outcome = ToolAuditOutcome::Denied;
    assert_eq!(denied.validate(), Ok(()));
    denied.outcome = ToolAuditOutcome::Failed;
    assert_eq!(
        denied.validate(),
        Err("event omits risk for a non-denied outcome")
    );
}
