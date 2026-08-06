use criterion::{Criterion, criterion_group, criterion_main};
use rullst_ai::ai::{AiTool, Message, ToolParam, ToolRegistry};
use serde_json::Value;
use std::hint::black_box;

struct SampleDbTool;

impl AiTool for SampleDbTool {
    fn name(&self) -> &str {
        "db_query"
    }
    fn description(&self) -> &str {
        "Executes a parameterized SQL query against the Rullst database schema"
    }
    fn parameters(&self) -> Vec<ToolParam> {
        vec![
            ToolParam {
                name: "sql".to_string(),
                param_type: "string".to_string(),
                description: "The SQL statement to execute".to_string(),
                required: true,
            },
            ToolParam {
                name: "limit".to_string(),
                param_type: "integer".to_string(),
                description: "Max rows to return".to_string(),
                required: false,
            },
        ]
    }
    fn execute(&self, _payload: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Ok(serde_json::json!({ "status": "ok" }))
    }
}

fn bench_tool_schema_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ai_tool_registry");

    let mut registry = ToolRegistry::new();
    registry.register(SampleDbTool);

    group.bench_function("export_openai_schema", |b| {
        b.iter(|| registry.export_openai_schema())
    });

    group.finish();
}

fn bench_message_context_tokens(c: &mut Criterion) {
    let mut group = c.benchmark_group("ai_message_context");

    let messages = vec![
        Message {
            role: "system".to_string(),
            content: "You are Rullst AI Assistant. Help the user optimize Rust code.".to_string(),
        },
        Message {
            role: "user".to_string(),
            content: "How do I implement static dispatch in Rullst?".to_string(),
        },
        Message {
            role: "assistant".to_string(),
            content: "Use generics or impl Trait instead of dyn Trait for zero cost abstractions."
                .to_string(),
        },
    ];

    group.bench_function("message_json_serialization", |b| {
        b.iter(|| serde_json::to_string(black_box(&messages)))
    });

    group.bench_function("estimate_context_tokens", |b| {
        b.iter(|| {
            let total_len: usize = messages.iter().map(|m| m.content.len()).sum();
            black_box(total_len / 4)
        })
    });

    group.finish();
}

fn bench_pii_masking(c: &mut Criterion) {
    let mut group = c.benchmark_group("ai_pii_masking");

    let pii_input = "User John Doe registered with email john@example.com and phone 555-0199.";

    group.bench_function("mask_pii", |b| {
        b.iter(|| rullst_core::security::mask_pii(black_box(pii_input)))
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets = bench_tool_schema_generation, bench_message_context_tokens, bench_pii_masking
);
criterion_main!(benches);
