# Schema-first Rust and TypeScript APIs

This is an **unpublished v13 candidate**. `generate:api` consumes one explicit
OpenAPI 3.1 JSON document and generates Rust DTOs/operation codecs, a TypeScript
client and a canonical OpenAPI copy. It does not infer request or response types
from a route handler, install dependencies or mount routes. The earlier
`generate:openapi` and `generate:ts` scanning commands remain separate discovery
helpers with placeholder/unchecked types.

## Generate and check

Use the CLI built from the reviewed v13 source as described in the
[adoption guide](migration-v13.md):

```bash
cargo rullst generate:api --schema api/profile.json --output src/profile_api
cargo rullst generate:api --schema api/profile.json --output src/profile_api --check
```

The output directory contains `contract.rs`, `client.ts` and `openapi.json`.
`--check` compares all three without writing anything. A missing, changed or stale
output fails. Regeneration preflights all files, preserves unrelated files and
refuses linked, special or unrecognized existing outputs. Keep application
handlers outside these generated files. The schema digest records input identity;
it does not authenticate the schema or prove deployment compatibility. Generation
assumes a trusted project directory without concurrent adversarial filesystem edits.

This minimal input uses the explicit `rullst.api.v1` profile:

```json
{
  "openapi": "3.1.1",
  "x-rullst-profile": "rullst.api.v1",
  "info": {"title": "Profile API", "version": "1"},
  "security": [{"bearerAuth": []}],
  "components": {
    "securitySchemes": {"bearerAuth": {"type": "http", "scheme": "bearer"}},
    "schemas": {
      "Profile": {
        "type": "object", "additionalProperties": false,
        "required": ["display_name", "note"],
        "properties": {
          "display_name": {"type": "string", "minLength": 1, "maxLength": 80},
          "note": {"type": ["string", "null"], "maxLength": 200}
        }
      },
      "Failure": {
        "type": "object", "additionalProperties": false,
        "required": ["code"],
        "properties": {"code": {"type": "string", "maxLength": 80}}
      }
    }
  },
  "paths": {
    "/profiles/{owner}": {
      "get": {
        "operationId": "read_profile",
        "parameters": [{
          "name": "owner", "in": "path", "required": true,
          "schema": {"type": "string", "minLength": 1, "maxLength": 80}
        }],
        "responses": {
          "200": {"description": "Own profile", "content": {
            "application/json": {"schema": {"$ref": "#/components/schemas/Profile"}}
          }},
          "401": {"description": "Unauthenticated", "content": {
            "application/json": {"schema": {"$ref": "#/components/schemas/Failure"}}
          }},
          "403": {"description": "Forbidden", "content": {
            "application/json": {"schema": {"$ref": "#/components/schemas/Failure"}}
          }}
        }
      }
    }
  }
}
```

## Server wiring and authorization

Add direct `serde` (with `derive`), `serde_json` and `rullst-security` dependencies
matching the reviewed framework source. Import `contract.rs` as an application
module and construct `Contract::new()` once during startup; handle configuration
errors before serving traffic. Each `op_<operationId>` module provides `METHOD`,
`PATH`, a typed `Params`, `decode_params` and typed response variants plus
`encode_response`. Body operations also expose `Request` and `decode_request`.

Authenticate the bearer token and check tenant/resource ownership in the handler.
Pass decoded path/query pairs to `decode_params`, retaining duplicate parameters
so the codec can reject them. Path fields use `p_` and query fields `q_` prefixes.
For a body operation, enforce `application/json` and the 64 KiB transport limit
before reading, then pass the bytes to `decode_request`. Encode every response
through the operation codec, set its returned HTTP status and JSON content type,
and retain the application's secure headers, WAF, CSRF policy and rate limits.
Never derive the authenticated user from a submitted path/body owner value.

Serde derives are useful for application composition, but using them directly
bypasses schema, presence and size checks. The generated codecs are the wire
boundary. An optional field is omitted when its `Option` is `None`; a required
nullable field serializes `None` as JSON `null`. Missing required nullable fields
and null optional non-nullable fields are rejected before deserialization.

## Client use

Compile `client.ts` with TypeScript's `--strict --exactOptionalPropertyTypes`,
`--target ES2022` and `--lib ES2022,DOM`. The client uses Fetch, Web Streams,
TextEncoder/TextDecoder and AbortSignal.any; this increment is exercised on
Node 24, not a claim of acceptance on every browser or native WebView.

```typescript
const client = new ContractClient('https://api.example.com', () => sessionToken);
const response = await client.call_read_profile({p_owner: authenticatedUserId});
if (response.status === 200) {
    console.log(response.body.display_name);
} else {
    console.log(response.status, response.body.code);
}
```

The base URL must be an HTTP(S) origin without credentials, path, query or
fragment. Supply the current application-owned token through the callback; it
is not persisted by the client. Pass an optional AbortSignal to each operation.
The client encodes path/query values, refuses redirects, omits cookies, requests
no-store/no-referrer behavior and bounds each operation to ten seconds including
credential lookup. Callbacks still own cancellation of their own background work.
Declared error statuses return their typed body union. Unknown statuses, malformed
JSON, duplicate keys, invalid UTF-8, oversized responses and schema mismatches
raise a fixed `ContractError` without tokens, upstream bodies or server diagnostics.

## Supported profile and limits

The profile follows the [OpenAPI 3.1 schema model](https://spec.openapis.org/oas/v3.1.1.html)
and preserves [required versus optional object properties](https://json-schema.org/understanding-json-schema/reference/object).
Enable [strict null checking](https://www.typescriptlang.org/tsconfig/strictNullChecks.html)
so those distinctions remain visible to the consumer compiler.

| Area | Initial supported contract |
| --- | --- |
| Document | JSON; OpenAPI 3.1.0/3.1.1/3.1.2; explicit `rullst.api.v1`; global HTTP bearer scheme. No external files/network resolution. |
| Shapes | Closed named objects, local acyclic object references, bounded strings/arrays, booleans and safe integers. Unknown keywords fail. |
| Presence | Required nullable scalar/array fields and optional non-nullable fields. Optional nullable fields and nullable references are unsupported. |
| Strings/arrays | Required `maxLength` ≤ 4096 code points or `maxItems` ≤ 128; optional valid minimum. Invalid Unicode surrogates fail. |
| Integers | Explicit `minimum`/`maximum` within ±9,007,199,254,740,991. Numeric tokens ≤128 characters and exponent within ±128; exact decimal/exponent forms must represent an integer, without fractional rounding. |
| Parameters | Explicit path strings with `minLength` ≥1 and query strings/integers/booleans; explicit `required`. Empty/dot path segments, unknown/duplicate parameters, noncanonical query integers and null query values fail. |
| Operations | GET/DELETE without body; POST/PUT/PATCH with required JSON object body; explicit JSON success, 401 and 403 responses. No wildcard statuses, redirects, empty-body 204/205 or streaming. |
| Names | Components: uppercase-first ASCII alphanumeric, ≤64 characters. Fields/operation IDs: lowercase-first ASCII letters, digits and underscores, ≤40; reserved collision names fail. |
| Generation | 128 KiB source, depth24/8192 source nodes, 32 schemas, 32 paths/operations, 32 properties/object, 16 parameters/responses, reference depth8; 512 KiB per output. |
| Wire | 64 KiB per received/sent JSON body, depth24 and 8192 value nodes. Exact integer normalization has a separate bounded scratch buffer. |

Formats, patterns, defaults, enums, open maps, inline objects, recursive schemas,
polymorphism, arbitrary numeric wire values, file uploads, headers/cookie parameters
and authentication schemes beyond this profile are rejected. Extend the source
profile and its compiled consumer tests before adding another supported shape.

Local acceptance compiles generated Rust with strict Clippy and TypeScript 5.9.3
from a pinned lockfile, then executes the client against a disposable Rust HTTP
server. It covers Unicode, optional/null semantics, input/output validation,
query duplicates, cross-owner/unauthenticated denial and transport faults.
The test's fixed identities are fixtures, not a production authentication service.
The combined source passed hosted and installed-archive acceptance in
[PR #221](https://github.com/Rullst/Rullst/pull/221). The final release campaign
and deployment-specific browser acceptance remain separate requirements.
