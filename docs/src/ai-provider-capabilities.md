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
| OpenAI | yes | yes | yes | yes | native mode | yes | no | no | yes | no | no |
| Anthropic | yes | yes | no | yes | prompt only | no | no | no | yes | no | no |
| Gemini | yes | yes | yes | yes | native mode | yes | no | no | yes | no | no |
| DeepSeek | yes | yes | no | no | native mode | default model only | no | no | yes | no | no |
| Ollama | yes | yes | yes | yes | native mode | yes | no | no | yes | no | no |
| OpenAI-compatible | yes | yes | declared | declared | declared native mode | declared | declared SSE | no | yes | no | declared for SSE |

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
- The OpenAI-compatible adapter defaults to text/chat only. Its exact
  endpoint/model configuration must explicitly declare embeddings, vision,
  native JSON mode, JSON Schema, and SSE streaming. This reports which request shapes Rullst
  will send; it does not discover or certify model behavior.

## Vision input sources

The Vision column describes provider request-shape support, not unrestricted
image acquisition. `AiClient::prompt_with_image` accepts application-admitted
bytes. `prompt_with_image_file` additionally requires an exact canonical
`LocalImagePolicy` root and byte budget. `prompt_with_image_url` accepts only an
explicit `EgressFetcher`, so HTTPS host allowlisting, DNS pinning, peer and
redirect validation, proxy bypass, timeout and streaming limits are mandatory.
Both helpers verify bounded JPEG/PNG/WebP/GIF signatures; a supplied remote
media type must match the bytes. Capability and prompt checks happen before
file or network I/O. Local-directory trust, tenant/owner authorization, image
decoding safety and model behavior remain application/provider boundaries.

## OpenAI-compatible local and cloud endpoints

`OpenAiCompatibleProvider::try_local` accepts an unauthenticated OpenAI-shaped
base URL only when its host is a literal loopback IP such as `127.0.0.1` or
`::1`. It may use HTTP for local development and never probes localhost
implicitly; requiring an IP literal avoids trusting host-name resolution.
`try_local_with_bearer` supports an explicitly authenticated loopback server.
`try_cloud` requires HTTPS and Bearer authentication; empty and `mock_*` keys
select the offline fixture. All constructors reject URL credentials, query
strings, and fragments, disable redirects and environment proxies, cap images
at 10 MiB and JSON responses at 2 MiB, and retain the ordinary 30-second
configurable request deadline.

This adapter covers `/chat/completions`, optional `/embeddings`, OpenAI-shaped
image content, the declared response-format modes, and opt-in strict
`text/event-stream` chat deltas. It does not claim Azure query/header
conventions, arbitrary authentication schemes, provider-native tools, retries,
automatic model discovery, or compatibility with an
unrelated HTTP protocol. Implement `AiProvider` for those explicit semantics.
Local runtimes such as llama.cpp server, LocalAI, LM Studio, and vLLM are
possible consumers only when their installed configuration exposes these exact
shapes; Rullst does not certify a product name or infer capabilities from it.

## Operational boundaries

### Streaming

`StreamingAiClient<P>` applies provider-independent output limits through
static dispatch. For an exact OpenAI-compatible configuration that declares
`with_streaming()`, Rullst parses incremental UTF-8 SSE deltas, requires the
terminal `[DONE]` marker, rejects an incorrect media type, malformed/truncated
events and all configured byte/chunk overflows. The maximums are 4,096 chunks,
64 KiB per chunk and 2 MiB of raw response and delivered text. DeepSeek and
Ollama ordinary payloads still select `stream: false`; other provider-specific
streaming protocols remain unimplemented rather than being treated as OpenAI-compatible.

### Timeouts and cancellation

Every built-in live transport applies a 30-second request deadline by default.
Each provider exposes `with_request_timeout(Duration)` to select a stricter or
longer deadline, and a loopback regression proves timeout classification on the
OpenAI-compatible transport. This bounds the local request future; it is not
proof that an upstream provider stopped work or billing. The adapters still do
not expose cancellation for ordinary `AiProvider` calls. `AiCancellation`
provides an explicit cloneable signal for `StreamingAiClient`; the compatible
transport races it against both the initial request and every streamed body
read. Other provider protocols still use deadline/drop semantics.

### Retries

The adapters make one transport attempt. There is no automatic retry,
backoff, idempotency classification, or retry budget in `rullst-ai`. An
application-level retry must classify operations carefully and must not assume
that an interrupted provider request was never processed.

These provider-call rules are distinct from `AuditDeliveryClient`. The latter
supports one to five bounded attempts only for transport/deadline failures,
HTTP 429 and HTTP 5xx, while preserving a caller-supplied event ID. Its receiver
must deduplicate that ID because a timed-out request may already have been
accepted.

### Authenticated audit export

`AuditDeliveryClient` is an opt-in transport for application-minimized audit
events, not an AI provider adapter. It signs the exact JSON body with
HMAC-SHA256 plus a key ID and timestamp, caps an event at 16 KiB and an
acknowledgement at 8 KiB, disables redirects and ambient proxies, and requires
the acknowledgement to bind the original event ID. Cloud endpoints require
HTTPS; literal-loopback HTTP(S) is reserved for development fixtures. Empty or
`mock_*` keys never use the network.

The receiver owns signature/freshness verification, event-ID deduplication,
authorization, persistence, retention, key distribution/rotation and SIEM
operations. The client does not automatically attach itself to RAG/tools or
provider calls and does not inspect an arbitrary serialized event for secrets.

### Tools

`ToolRegistry` is a separate [guarded local execution
boundary](ai-tool-security.md). Dispatch requires an exact policy allowlist,
principal authorization, closed JSON validation, payload limits, a call budget
and an audit sink. Destructive and financial calls additionally consume a
one-use approval bound to the exact payload. Applications may select either the
bounded process-local in-memory sink or `DurableToolAuditTrail`, whose local
versioned file validates restart integrity and fails closed on quota,
corruption, symlink targets and competing-writer growth. The durable sink is
single-process and does not provide authenticity, rotation, retention, backup,
external delivery or principal/approver authentication.

The registry is not connected to any built-in provider transport. Consequently
the provider tools column remains `no`, and local guarded execution must not be
advertised as provider-native function calling or an autonomous safe agent.

## Portable custom-provider default

A custom `AiProvider` receives a compatibility default of text, chat,
embeddings, and prompt-only JSON because those first three methods are required
by the trait and JSON has a guarded prompt fallback. Custom implementations
that deliberately reject one of those methods, or implement additional native
paths, must override `capabilities()`.

`FallbackProvider` reports the union of its configured providers. The union
means that at least one provider claims a path; it does not guarantee which
provider will satisfy a particular model-specific request.
