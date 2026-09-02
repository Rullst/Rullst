use super::*;
use std::io;

struct EchoTool;

impl AiTool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echo a message"
    }

    fn parameters(&self) -> Vec<ToolParam> {
        vec![ToolParam {
            name: "message".to_string(),
            param_type: "string".to_string(),
            description: "Message to echo".to_string(),
            required: true,
        }]
    }

    fn risk(&self) -> ToolRisk {
        ToolRisk::ReadOnly
    }

    fn execute(&self, payload: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(payload)
    }
}

struct FinancialTool;

impl AiTool for FinancialTool {
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

struct FailingTool;

impl AiTool for FailingTool {
    fn name(&self) -> &str {
        "fail"
    }

    fn description(&self) -> &str {
        "Always fail"
    }

    fn parameters(&self) -> Vec<ToolParam> {
        Vec::new()
    }

    fn risk(&self) -> ToolRisk {
        ToolRisk::Mutating
    }

    fn execute(&self, _: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err(Box::new(io::Error::other("secret provider error")))
    }
}

fn guarded_echo() -> (
    ToolRegistry,
    ToolExecutionPolicy,
    ToolExecutionContext,
    InMemoryToolAuditTrail,
) {
    let mut registry = ToolRegistry::new();
    registry.register(EchoTool).expect("valid echo tool");
    let policy = ToolExecutionPolicy::new(["echo"]).expect("valid policy");
    let context = ToolExecutionContext::new("user-17", ["echo"], 2).expect("valid authorization");
    let audit = InMemoryToolAuditTrail::new(16).expect("valid audit trail");
    (registry, policy, context, audit)
}

#[test]
fn guarded_execution_validates_schema_budget_and_records_audit() {
    let (registry, policy, mut context, audit) = guarded_echo();
    let schema = registry.export_openai_schema(&policy);
    assert_eq!(schema.as_array().map(Vec::len), Some(1));
    assert_eq!(
        schema[0]["function"]["parameters"]["additionalProperties"],
        false
    );

    let result = registry
        .execute(
            "echo",
            serde_json::json!({"message": "hello"}),
            &mut context,
            &policy,
            &audit,
        )
        .expect("authorized echo");
    assert_eq!(result, serde_json::json!({"message": "hello"}));
    assert_eq!(context.remaining_calls(), 1);

    let events = audit.entries().expect("audit entries");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].sequence, 1);
    assert_eq!(events[0].event.outcome, ToolAuditOutcome::Authorized);
    assert_eq!(events[1].event.outcome, ToolAuditOutcome::Succeeded);
}

#[test]
// TM-AI-03: unknown, invalid and unauthorized tool calls fail before execution.
fn policy_context_and_schema_all_fail_closed_and_are_audited() {
    let (registry, policy, mut context, audit) = guarded_echo();
    let unknown = registry.execute(
        "missing",
        serde_json::json!({}),
        &mut context,
        &policy,
        &audit,
    );
    assert!(matches!(unknown, Err(ToolExecutionError::ToolNotFound(_))));

    let invalid = registry.execute(
        "echo",
        serde_json::json!({"message": 7, "extra": true}),
        &mut context,
        &policy,
        &audit,
    );
    assert!(matches!(
        invalid,
        Err(ToolExecutionError::InvalidPayload { .. })
    ));

    let denied_policy = ToolExecutionPolicy::new(Vec::<String>::new()).expect("deny all");
    let denied = registry.execute(
        "echo",
        serde_json::json!({"message": "hello"}),
        &mut context,
        &denied_policy,
        &audit,
    );
    assert!(matches!(
        denied,
        Err(ToolExecutionError::Unauthorized { .. })
    ));
    assert_eq!(context.remaining_calls(), 2);
    assert!(
        audit
            .entries()
            .expect("audit entries")
            .iter()
            .all(|entry| entry.event.outcome == ToolAuditOutcome::Denied)
    );
}

#[test]
fn financial_approval_is_exact_and_consumed_once() {
    let mut registry = ToolRegistry::new();
    registry
        .register(FinancialTool)
        .expect("valid financial tool");
    let policy = ToolExecutionPolicy::new(["issue_refund"]).expect("valid policy");
    let mut context = ToolExecutionContext::new("finance-user", ["issue_refund"], 2)
        .expect("valid authorization");
    let audit = InMemoryToolAuditTrail::new(16).expect("valid audit trail");

    let missing = registry.execute(
        "issue_refund",
        serde_json::json!({"amount": 10.0}),
        &mut context,
        &policy,
        &audit,
    );
    assert!(matches!(
        missing,
        Err(ToolExecutionError::HumanApprovalRequired { .. })
    ));

    let approved_payload = serde_json::json!({"amount": 10.0});
    let approval = HumanApproval::for_payload(
        "issue_refund",
        &approved_payload,
        "reviewer-9",
        "ticket FIN-42",
    )
    .expect("valid approval");
    assert_eq!(approval.approver(), "reviewer-9");
    assert_eq!(approval.reason(), "ticket FIN-42");
    context.approve(approval);
    registry
        .execute(
            "issue_refund",
            serde_json::json!({"amount": 10.0}),
            &mut context,
            &policy,
            &audit,
        )
        .expect("approved refund");
    let approved = audit
        .entries()
        .expect("audit entries")
        .into_iter()
        .find(|entry| entry.event.outcome == ToolAuditOutcome::Authorized)
        .expect("authorized event");
    assert_eq!(approved.event.approved_by.as_deref(), Some("reviewer-9"));
    assert_eq!(
        approved.event.approval_reason.as_deref(),
        Some("ticket FIN-42")
    );

    let replay = registry.execute(
        "issue_refund",
        serde_json::json!({"amount": 10.0}),
        &mut context,
        &policy,
        &audit,
    );
    assert!(matches!(
        replay,
        Err(ToolExecutionError::HumanApprovalRequired { .. })
    ));
}

#[test]
fn financial_approval_cannot_be_reused_with_different_arguments() {
    let mut registry = ToolRegistry::new();
    registry
        .register(FinancialTool)
        .expect("valid financial tool");
    let policy = ToolExecutionPolicy::new(["issue_refund"]).expect("valid policy");
    let mut context = ToolExecutionContext::new("finance-user", ["issue_refund"], 1)
        .expect("valid authorization");
    let audit = InMemoryToolAuditTrail::new(16).expect("valid audit trail");
    context.approve(
        HumanApproval::for_payload(
            "issue_refund",
            &serde_json::json!({"amount": 10.0}),
            "reviewer-9",
            "ticket FIN-42",
        )
        .expect("valid approval"),
    );

    let changed = registry.execute(
        "issue_refund",
        serde_json::json!({"amount": 1000.0}),
        &mut context,
        &policy,
        &audit,
    );
    assert!(matches!(
        changed,
        Err(ToolExecutionError::ApprovalPayloadMismatch { .. })
    ));
    assert_eq!(context.remaining_calls(), 1);
}

#[test]
fn payload_failure_and_budget_limits_are_enforced() {
    let (registry, policy, mut context, audit) = guarded_echo();
    let tiny = policy
        .clone()
        .with_payload_limits(8, 8)
        .expect("valid tiny limits");
    let oversized = registry.execute(
        "echo",
        serde_json::json!({"message": "hello"}),
        &mut context,
        &tiny,
        &audit,
    );
    assert!(matches!(
        oversized,
        Err(ToolExecutionError::InputTooLarge { .. })
    ));

    let mut one_call = ToolExecutionContext::new("user-17", ["echo"], 1).expect("one call");
    registry
        .execute(
            "echo",
            serde_json::json!({"message": "ok"}),
            &mut one_call,
            &policy,
            &audit,
        )
        .expect("first call");
    let exhausted = registry.execute(
        "echo",
        serde_json::json!({"message": "again"}),
        &mut one_call,
        &policy,
        &audit,
    );
    assert_eq!(exhausted, Err(ToolExecutionError::CallBudgetExhausted));

    let mut failing_registry = ToolRegistry::new();
    failing_registry
        .register(FailingTool)
        .expect("valid failing tool");
    let failing_policy = ToolExecutionPolicy::new(["fail"]).expect("failing policy");
    let mut failing_context =
        ToolExecutionContext::new("user-17", ["fail"], 1).expect("failing context");
    let failed = failing_registry.execute(
        "fail",
        serde_json::json!({}),
        &mut failing_context,
        &failing_policy,
        &audit,
    );
    assert!(matches!(
        failed,
        Err(ToolExecutionError::ExecutionFailed { .. })
    ));
}

#[test]
fn invalid_registration_policy_context_and_audit_capacity_are_rejected() {
    let mut registry = ToolRegistry::new();
    registry.register(EchoTool).expect("first registration");
    assert!(matches!(
        registry.register(EchoTool),
        Err(ToolExecutionError::DuplicateTool(_))
    ));
    assert!(ToolExecutionPolicy::new(["../escape"]).is_err());
    assert!(ToolExecutionContext::new("", ["echo"], 1).is_err());
    assert!(ToolExecutionContext::new("user", ["echo"], 0).is_err());
    assert!(InMemoryToolAuditTrail::new(0).is_err());
    assert!(InMemoryToolAuditTrail::new(1_000_001).is_err());
}
