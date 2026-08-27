# AI provider capability matrix

This matrix describes the transport paths implemented by `rullst-ai` v12. It
is not a claim that every model sold by a provider accepts every request. Model
availability, account entitlements, regions, quotas, and upstream API behavior
remain provider concerns.

Applications can inspect the same contract in code through
`AiProvider::capabilities()` or `AiClient::capabilities()`. Built-in provider
tests assert every row so unsupported paths remain explicit instead of silently
falling back to another operation.

| Provider transport | Text | Chat | Embeddings | Vision | JSON | JSON Schema | Streaming | Provider tools | Rullst timeout | Automatic retry | Explicit cancellation |
| --- | :---: | :---: | :---: | :---: | --- | :---: | :---: | :---: | :---: | :---: | :---: |
| OpenAI | yes | yes | yes | yes | native mode | yes | no | no | no | no | no |
| Anthropic | yes | yes | no | yes | prompt only | no | no | no | no | no | no |
| Gemini | yes | yes | yes | yes | native mode | yes | no | no | no | no | no |
| DeepSeek | yes | yes | no | no | native mode | default model only | no | no | no | no | no |
| Ollama | yes | yes | yes | yes | native mode | yes | no | no | no | no | no |

`yes` means Rullst constructs and parses that provider request. A configured
model can still reject vision, embeddings, or schema output. In particular:

- DeepSeek JSON Schema is enabled only for the default `deepseek-v4-flash`
  transport contract. Selecting another model makes the capability false and
  returns `UnsupportedCapability`, including in deterministic offline mode.
- Ollama vision, embeddings, JSON, and schema support depend on the installed
  local models. The transport can send those request shapes; it cannot prove
  that an arbitrary model implements them.
- Anthropic JSON uses an instruction requesting one JSON value. It is labelled
  `prompt only` because the current transport does not request native JSON mode
  or schema enforcement.
- Empty and `mock_*` credentials select deterministic offline behavior. They do
  not contact the configured endpoint and do not promote capabilities that the
  transport marks unsupported.

## Operational boundaries

### Streaming

All generation requests currently wait for a complete response. The DeepSeek
and Ollama payloads explicitly select `stream: false`; the other adapters use
their non-streaming response shapes. Rullst exposes no token-stream API in v12.

### Timeouts and cancellation

The built-in clients currently do not impose a Rullst request deadline and do
not expose a cancellation token or abort handle. Dropping an async request
future is the caller's only local cancellation mechanism; it is not proof that
an upstream provider stopped work or billing. Applications should apply a
bounded outer timeout until a provider-neutral deadline API is implemented.

### Retries

The adapters make one transport attempt. There is no automatic retry,
backoff, idempotency classification, or retry budget in `rullst-ai`. An
application-level retry must classify operations carefully and must not assume
that an interrupted provider request was never processed.

### Tools

`ToolRegistry` can register local Rust tools, export an OpenAI-compatible schema,
and dispatch a registered name. It is not connected to any built-in provider
transport and does not yet supply the v12 authorization, argument validation,
resource limits, approval, or durable audit policy. Consequently the provider
tools column remains `no`, and tool execution must not be advertised as an
autonomous safe agent boundary.

## Portable custom-provider default

A custom `AiProvider` receives a compatibility default of text, chat,
embeddings, and prompt-only JSON because those first three methods are required
by the trait and JSON has a guarded prompt fallback. Custom implementations
that deliberately reject one of those methods, or implement additional native
paths, must override `capabilities()`.

`FallbackProvider` reports the union of its configured providers. The union
means that at least one provider claims a path; it does not guarantee which
provider will satisfy a particular model-specific request.
