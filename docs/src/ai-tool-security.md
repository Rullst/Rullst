# Guarded local AI tools

`rullst-ai` includes a local tool registry, not provider-native function calling
or an autonomous agent runtime. Model-produced tool names and arguments are
untrusted input. The only execution entry point requires all of these controls:

- an exact registry-level allowlist (`ToolExecutionPolicy`);
- a principal-specific authorization set (`ToolExecutionContext`);
- a closed, bounded JSON object validated from `ToolParam` declarations;
- non-zero serialized input/output limits and a per-context call budget;
- a mandatory `ToolAuditSink` that fails execution closed when unavailable; and
- a one-use, exact-payload approval for `Destructive` and `Financial` tools.

The application authenticates the principal and approver. Rullst does not infer
authorization from model output, a prompt, a role string, or tool registration.

## Minimal read-only dispatch

```rust
use rullst_ai::ai::{
    AiTool, InMemoryToolAuditTrail, ToolExecutionContext, ToolExecutionPolicy,
    ToolParam, ToolRegistry, ToolRisk,
};
use serde_json::{Value, json};

struct AccountStatus;

impl AiTool for AccountStatus {
    fn name(&self) -> &str { "account_status" }
    fn description(&self) -> &str { "Read an account status" }
    fn parameters(&self) -> Vec<ToolParam> {
        vec![ToolParam {
            name: "account_id".into(),
            param_type: "string".into(),
            description: "Authorized account identifier".into(),
            required: true,
        }]
    }
    fn risk(&self) -> ToolRisk { ToolRisk::ReadOnly }
    fn execute(
        &self,
        payload: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(json!({ "account_id": payload["account_id"], "status": "active" }))
    }
}

let mut registry = ToolRegistry::new();
registry.register(AccountStatus)?;

let policy = ToolExecutionPolicy::new(["account_status"])?
    .with_payload_limits(4 * 1024, 16 * 1024)?;
let mut context = ToolExecutionContext::new(
    "authenticated-user-17",
    ["account_status"],
    3,
)?;
let audit = InMemoryToolAuditTrail::new(128)?;

let output = registry.execute(
    "account_status",
    json!({ "account_id": "acct-17" }),
    &mut context,
    &policy,
    &audit,
)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`additionalProperties: false` is emitted in the exported schema and enforced at
dispatch. Supported parameter types are the bounded JSON primitives `string`,
`number`, `integer`, `boolean`, `object`, `array`, and `null`. Applications must
perform domain validation inside the tool as well: a JSON string is not proof
that an account identifier belongs to the principal.

## Destructive and financial approval

High-risk approval is bound to the exact serialized payload. Changing an amount,
destination, identifier, or any other field produces
`ToolExecutionError::ApprovalPayloadMismatch`.

```rust
use rullst_ai::ai::HumanApproval;
use serde_json::json;

let payload = json!({ "invoice_id": "inv-42", "amount_cents": 1500 });
let approval = HumanApproval::for_payload(
    "issue_refund",
    &payload,
    "authenticated-reviewer-9",
    "support ticket FIN-42",
)?;
context.approve(approval);

let result = registry.execute(
    "issue_refund",
    payload,
    &mut context,
    &policy,
    &audit,
)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The approval is consumed once. Its approver and bounded reason are recorded in
authorized/success/failure audit events, while the payload itself is omitted to
avoid duplicating secrets or personal data in the audit trail.

## Production boundary

`InMemoryToolAuditTrail` is bounded and process-local; it is intended for local
development and single-process tests. A production or multi-instance
application should implement `ToolAuditSink` using a durable append-only store
with retention, access control, integrity monitoring, and incident export.

The registry deliberately does not provide network fetchers, shell execution,
database ownership policy, distributed budgets, provider tool-call parsing, or
human identity verification. Tools providing those capabilities must enforce
their own domain authorization and egress policies. The [provider capability
matrix](ai-provider-capabilities.md) therefore continues to report provider
tools as unsupported.
