//! Explicitly authorized local tool dispatch for AI-assisted workflows.
//!
//! Provider-native tool calling is not implemented by the built-in transports.
//! This module exposes a local registry whose execution path requires an exact
//! allowlist, caller authorization, bounded JSON, a call budget, and an audit
//! sink. Destructive and financial tools additionally require a one-use approval
//! record. The application remains responsible for authenticating the principal
//! and the human approver.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

mod audit;
mod policy;
mod validation;
pub use audit::{
    InMemoryToolAuditTrail, RecordedToolAuditEvent, ToolAuditEvent, ToolAuditOutcome, ToolAuditSink,
};
pub use policy::{HumanApproval, ToolExecutionContext, ToolExecutionPolicy};
use validation::{serialized_size, validate_payload, validate_tool};

/// Representation of a parameter in an AI tool's bounded JSON schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolParam {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// Operational impact assigned by the tool implementation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolRisk {
    ReadOnly,
    Mutating,
    Destructive,
    Financial,
}

impl ToolRisk {
    const fn requires_human_approval(self) -> bool {
        matches!(self, Self::Destructive | Self::Financial)
    }
}

/// Trait implemented by a Rust function or struct exposed as a local AI tool.
pub trait AiTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Vec<ToolParam>;
    /// Declares the tool's operational impact. This has no permissive default.
    fn risk(&self) -> ToolRisk;
    fn execute(&self, payload: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
}

/// Typed failures from registry configuration or guarded tool execution.
#[non_exhaustive]
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ToolExecutionError {
    #[error("invalid tool policy: {0}")]
    InvalidPolicy(String),
    #[error("tool '{0}' is already registered")]
    DuplicateTool(String),
    #[error("tool '{0}' is not registered")]
    ToolNotFound(String),
    #[error("principal is not authorized to execute tool '{tool}'")]
    Unauthorized { tool: String },
    #[error("tool '{tool}' requires a one-use human approval")]
    HumanApprovalRequired { tool: String },
    #[error("tool call budget is exhausted")]
    CallBudgetExhausted,
    #[error("tool input is {actual} bytes; limit is {limit} bytes")]
    InputTooLarge { actual: usize, limit: usize },
    #[error("tool output is {actual} bytes; limit is {limit} bytes")]
    OutputTooLarge { actual: usize, limit: usize },
    #[error("invalid payload for tool '{tool}': {reason}")]
    InvalidPayload { tool: String, reason: String },
    #[error("tool '{tool}' execution failed")]
    ExecutionFailed { tool: String },
    #[error("tool audit trail is unavailable: {0}")]
    AuditUnavailable(String),
}

/// Registry of locally implemented tools. Every execution requires a policy,
/// authorization context, and audit sink.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Box<dyn AiTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }

    pub fn register<T: AiTool + 'static>(&mut self, tool: T) -> Result<(), ToolExecutionError> {
        validate_tool(&tool)?;
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            return Err(ToolExecutionError::DuplicateTool(name));
        }
        self.tools.insert(name, Box::new(tool));
        Ok(())
    }

    /// Exports schemas only for tools selected by the exact execution policy.
    pub fn export_openai_schema(&self, policy: &ToolExecutionPolicy) -> Value {
        let tools_json = self
            .tools
            .values()
            .filter(|tool| policy.allows(tool.name()))
            .map(|tool| {
                let mut properties = serde_json::Map::new();
                let mut required_fields = Vec::new();
                for param in tool.parameters() {
                    properties.insert(
                        param.name.clone(),
                        serde_json::json!({
                            "type": param.param_type,
                            "description": param.description
                        }),
                    );
                    if param.required {
                        required_fields.push(param.name);
                    }
                }
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": {
                            "type": "object",
                            "properties": properties,
                            "required": required_fields,
                            "additionalProperties": false
                        }
                    }
                })
            })
            .collect::<Vec<_>>();
        Value::Array(tools_json)
    }

    /// Executes a tool only after every authorization and validation gate passes.
    pub fn execute(
        &self,
        name: &str,
        payload: Value,
        context: &mut ToolExecutionContext,
        policy: &ToolExecutionPolicy,
        audit: &dyn ToolAuditSink,
    ) -> Result<Value, ToolExecutionError> {
        let tool = match self.tools.get(name) {
            Some(tool) => tool,
            None => {
                return Err(deny(
                    audit,
                    context,
                    name,
                    None,
                    ToolExecutionError::ToolNotFound(name.to_string()),
                ));
            }
        };
        let risk = tool.risk();
        if !policy.allows(name) || !context.is_authorized(name) {
            return Err(deny(
                audit,
                context,
                name,
                Some(risk),
                ToolExecutionError::Unauthorized {
                    tool: name.to_string(),
                },
            ));
        }
        if context.remaining_calls == 0 {
            return Err(deny(
                audit,
                context,
                name,
                Some(risk),
                ToolExecutionError::CallBudgetExhausted,
            ));
        }

        let input_size = serialized_size(&payload, name)?;
        if input_size > policy.max_input_bytes {
            return Err(deny(
                audit,
                context,
                name,
                Some(risk),
                ToolExecutionError::InputTooLarge {
                    actual: input_size,
                    limit: policy.max_input_bytes,
                },
            ));
        }
        validate_payload(name, &payload, &tool.parameters())
            .map_err(|error| deny(audit, context, name, Some(risk), error))?;

        let approval = if risk.requires_human_approval() {
            match context.consume_approval(name) {
                Some(approval) => Some(approval),
                None => {
                    return Err(deny(
                        audit,
                        context,
                        name,
                        Some(risk),
                        ToolExecutionError::HumanApprovalRequired {
                            tool: name.to_string(),
                        },
                    ));
                }
            }
        } else {
            None
        };

        audit.record(audit_event(
            context,
            name,
            Some(risk),
            approval.as_ref(),
            ToolAuditOutcome::Authorized,
        ))?;
        context.remaining_calls -= 1;

        let output = match tool.execute(payload) {
            Ok(output) => output,
            Err(_) => {
                audit.record(audit_event(
                    context,
                    name,
                    Some(risk),
                    approval.as_ref(),
                    ToolAuditOutcome::Failed,
                ))?;
                return Err(ToolExecutionError::ExecutionFailed {
                    tool: name.to_string(),
                });
            }
        };
        let output_size = serialized_size(&output, name)?;
        if output_size > policy.max_output_bytes {
            audit.record(audit_event(
                context,
                name,
                Some(risk),
                approval.as_ref(),
                ToolAuditOutcome::Failed,
            ))?;
            return Err(ToolExecutionError::OutputTooLarge {
                actual: output_size,
                limit: policy.max_output_bytes,
            });
        }
        audit.record(audit_event(
            context,
            name,
            Some(risk),
            approval.as_ref(),
            ToolAuditOutcome::Succeeded,
        ))?;
        Ok(output)
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

fn audit_event(
    context: &ToolExecutionContext,
    tool: &str,
    risk: Option<ToolRisk>,
    approval: Option<&HumanApproval>,
    outcome: ToolAuditOutcome,
) -> ToolAuditEvent {
    ToolAuditEvent {
        principal: context.principal.clone(),
        tool: tool.to_string(),
        risk,
        approved_by: approval.map(|approval| approval.approver().to_string()),
        approval_reason: approval.map(|approval| approval.reason().to_string()),
        outcome,
    }
}

fn deny(
    audit: &dyn ToolAuditSink,
    context: &ToolExecutionContext,
    tool: &str,
    risk: Option<ToolRisk>,
    error: ToolExecutionError,
) -> ToolExecutionError {
    match audit.record(audit_event(
        context,
        tool,
        risk,
        None,
        ToolAuditOutcome::Denied,
    )) {
        Ok(()) => error,
        Err(audit_error) => audit_error,
    }
}

#[cfg(test)]
mod tests;

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_tool_param_instantiation() {
        let required: bool = kani::any();
        let param = ToolParam {
            name: "param".to_string(),
            param_type: "string".to_string(),
            description: "description".to_string(),
            required,
        };
        assert_eq!(param.required, required);
    }
}
