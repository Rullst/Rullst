use super::{ToolExecutionError, validation};
use std::collections::{BTreeMap, BTreeSet};

/// Exact registry-level allowlist and input/output limits.
#[derive(Debug, Clone)]
pub struct ToolExecutionPolicy {
    allowed_tools: BTreeSet<String>,
    pub(super) max_input_bytes: usize,
    pub(super) max_output_bytes: usize,
}

impl ToolExecutionPolicy {
    /// Creates a fail-closed policy. An empty allowlist intentionally permits no tools.
    pub fn new<I, S>(allowed_tools: I) -> Result<Self, ToolExecutionError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut allowed = BTreeSet::new();
        for name in allowed_tools {
            let name = name.into();
            validation::validate_identifier("tool allowlist entry", &name)?;
            allowed.insert(name);
        }
        Ok(Self {
            allowed_tools: allowed,
            max_input_bytes: 64 * 1024,
            max_output_bytes: 256 * 1024,
        })
    }

    /// Sets non-zero serialized JSON input and output limits.
    pub fn with_payload_limits(
        mut self,
        max_input_bytes: usize,
        max_output_bytes: usize,
    ) -> Result<Self, ToolExecutionError> {
        if max_input_bytes == 0 || max_output_bytes == 0 {
            return Err(ToolExecutionError::InvalidPolicy(
                "tool payload limits must be greater than zero".to_string(),
            ));
        }
        self.max_input_bytes = max_input_bytes;
        self.max_output_bytes = max_output_bytes;
        Ok(self)
    }

    pub(super) fn allows(&self, name: &str) -> bool {
        self.allowed_tools.contains(name)
    }
}

/// One-use approval record for a destructive or financial tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanApproval {
    tool: String,
    approver: String,
    reason: String,
}

impl HumanApproval {
    pub fn new(
        tool: impl Into<String>,
        approver: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<Self, ToolExecutionError> {
        let tool = tool.into();
        let approver = approver.into();
        let reason = reason.into();
        validation::validate_identifier("approved tool", &tool)?;
        validation::validate_non_empty_bounded("approver", &approver, 256)?;
        validation::validate_non_empty_bounded("approval reason", &reason, 2 * 1024)?;
        Ok(Self {
            tool,
            approver,
            reason,
        })
    }

    pub fn approver(&self) -> &str {
        &self.approver
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// Authorization and remaining budget for one authenticated principal/workflow.
#[derive(Debug)]
pub struct ToolExecutionContext {
    pub(super) principal: String,
    authorized_tools: BTreeSet<String>,
    pub(super) remaining_calls: usize,
    approvals: BTreeMap<String, HumanApproval>,
}

impl ToolExecutionContext {
    pub fn new<I, S>(
        principal: impl Into<String>,
        authorized_tools: I,
        max_calls: usize,
    ) -> Result<Self, ToolExecutionError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let principal = principal.into();
        validation::validate_non_empty_bounded("principal", &principal, 256)?;
        if max_calls == 0 {
            return Err(ToolExecutionError::InvalidPolicy(
                "tool call budget must be greater than zero".to_string(),
            ));
        }
        let mut authorized = BTreeSet::new();
        for name in authorized_tools {
            let name = name.into();
            validation::validate_identifier("authorized tool", &name)?;
            authorized.insert(name);
        }
        Ok(Self {
            principal,
            authorized_tools: authorized,
            remaining_calls: max_calls,
            approvals: BTreeMap::new(),
        })
    }

    /// Records an approval. It is consumed by the first matching high-risk call.
    pub fn approve(&mut self, approval: HumanApproval) {
        self.approvals.insert(approval.tool.clone(), approval);
    }

    pub fn principal(&self) -> &str {
        &self.principal
    }

    pub const fn remaining_calls(&self) -> usize {
        self.remaining_calls
    }

    pub(super) fn is_authorized(&self, tool: &str) -> bool {
        self.authorized_tools.contains(tool)
    }

    pub(super) fn consume_approval(&mut self, tool: &str) -> Option<HumanApproval> {
        self.approvals.remove(tool)
    }
}
