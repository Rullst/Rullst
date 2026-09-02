# AI-friendly architecture and local model endpoints

Rullst favors explicit types, conventional file locations, bounded macros and
generated `.llms.txt` context. These choices can make a project easier for a
human or coding assistant to navigate, but the repository has no controlled
evidence for a universal token-saving percentage. Prompt size depends on the
task, tool, repository state and model.

## What “AI-native” means here

- `routes!` and `html!` provide recognizable syntax boundaries.
- Public APIs prefer concrete types and static dispatch where practical.
- `cargo rullst make:*` generates conventional, inspectable source files.
- `cargo rullst generate:ai-context` records a compact structural summary.
- External provider integrations have deterministic offline behavior for empty
  or `mock_*` credentials.

This does not guarantee that an assistant understands the application, chooses
the right edit, uses fewer tokens or produces secure code. Keep source review,
tests and application threat models authoritative.

The [AI maintainability and project-building roadmap](ai-maintainability-roadmap.md)
records the measurable post-v12-RC work needed to improve generated agent
instructions, bounded task-oriented context and reproducible model evaluations.
Those planned evaluation profiles are not current compatibility guarantees.

## Ollama through `AiClient::auto`

The high-level client recognizes `OLLAMA_HOST` and `OLLAMA_MODEL`:

```bash
export OLLAMA_HOST="http://127.0.0.1:11434"
export OLLAMA_MODEL="llama3"
```

```rust
use rullst_ai::ai::AiClient;

# async fn example() -> Result<(), rullst_ai::ai::AiError> {
let client = AiClient::auto()?;
let response = client.prompt("Summarize this bounded input").await?;
println!("{response}");
# Ok(())
# }
```

`AiClient::auto()` also recognizes the built-in cloud-provider API-key
variables. With no configured provider it selects a deterministic offline mock;
that fallback is not a live model.

## OpenAI-compatible endpoints

An OpenAI-compatible server is configured explicitly rather than inferred from
an arbitrary environment variable:

```rust
use rullst_ai::ai::{
    AiClient,
    providers::openai_compatible::{
        OpenAiCompatibleCapabilities, OpenAiCompatibleProvider,
    },
};

let provider = OpenAiCompatibleProvider::try_local(
    "http://127.0.0.1:1234/v1",
    "configured-model-name",
)?
.with_capabilities(OpenAiCompatibleCapabilities::chat_only());
let client = AiClient::new(provider);
# Ok::<(), rullst_ai::ai::AiError>(())
```

The same adapter can be used with local runtimes such as llama.cpp server,
LocalAI, LM Studio, or vLLM **when the exact installed version and
configuration expose the request shapes declared in Rullst**. Those product names
are examples, not compatibility certification; check the runtime's API and
model documentation. A server with a different protocol implements the public
`AiProvider` contract instead.

Compatibility must be tested for the methods the application uses. An endpoint
may implement chat while differing on embeddings, vision, JSON Schema, errors
or streaming. Optional request shapes are disabled until explicitly declared.
Use `try_local_with_bearer` for an authenticated loopback server and
`try_cloud` for HTTPS/Bearer cloud endpoints. Consult the [provider capability
matrix](ai-provider-capabilities.md).

## Privacy boundary

Using a loopback endpoint can avoid sending model requests to a cloud provider,
but it does not prove an air gap or zero leakage. The model runtime, host
network, proxy variables, logs, tracing, crash dumps and application code still
determine the real data path. The compatible adapter ignores ambient proxies
and redirects, but that is only one transport boundary. The built-in prompt and
PII checks are bounded heuristics, not authorization or a complete
data-loss-prevention guarantee.
