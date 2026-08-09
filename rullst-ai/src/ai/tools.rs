use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Representation of a parameter for an AI Tool schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParam {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

/// Trait implemented by any Rust function or struct exposed as an AI Tool
pub trait AiTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Vec<ToolParam>;
    fn execute(&self, payload: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>;
}

/// Registry storing all available AI Function Calling tools
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn AiTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a new tool into the AI registry
    pub fn register<T: AiTool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
    }

    /// Export OpenAI / Ollama compatible JSON Function Calling Tool Schema
    pub fn export_openai_schema(&self) -> Value {
        let mut tools_json = Vec::new();

        for tool in self.tools.values() {
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

            tools_json.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name(),
                    "description": tool.description(),
                    "parameters": {
                        "type": "object",
                        "properties": properties,
                        "required": required_fields
                    }
                }
            }));
        }

        Value::Array(tools_json)
    }

    /// Execute a registered tool by name with arguments
    pub fn execute(&self, name: &str, payload: Value) -> Result<Value, String> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| format!("Tool '{}' not found in registry", name))?;

        tool.execute(payload)
            .map_err(|e| format!("Execution error in tool '{}': {}", name, e))
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;
    impl AiTool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echo back input"
        }
        fn parameters(&self) -> Vec<ToolParam> {
            vec![ToolParam {
                name: "message".to_string(),
                param_type: "string".to_string(),
                description: "Message to echo".to_string(),
                required: true,
            }]
        }
        fn execute(
            &self,
            payload: Value,
        ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            Ok(payload)
        }
    }

    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(EchoTool);
        assert_eq!(registry.len(), 1);

        let schema = registry.export_openai_schema();
        assert!(schema.is_array());

        let res = registry.execute("echo", serde_json::json!({"message": "hello"}));
        assert!(res.is_ok());
    }
}

#[cfg(kani)]
#[cfg_attr(mutants, mutants::skip)]
mod kani_proofs {
    use super::*;

    #[kani::proof]
    fn proof_tool_param_instantiation() {
        let req: bool = kani::any();
        let param = ToolParam {
            name: "param".to_string(),
            param_type: "string".to_string(),
            description: "desc".to_string(),
            required: req,
        };
        assert_eq!(param.required, req);
    }
}
