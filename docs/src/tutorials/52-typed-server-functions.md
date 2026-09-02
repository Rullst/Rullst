# Tutorial 52: Typed Server Functions

`#[server_function]` lets one concrete async Rust signature describe both the
native server implementation and its Wasm caller. The transport is explicit:
the macro also creates a `<function>_rpc_router()` that you mount in the server.
It does not discover routes through runtime reflection.

```rust,no_run
use rullst::{Router, server_function};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct SumResponse {
    pub value: u32,
}

#[server_function(path = "/api/rpc/math/add")]
pub async fn add(left: u32, right: u32) -> rullst::rpc::RpcResult<SumResponse> {
    Ok(SumResponse {
        value: left.saturating_add(right),
    })
}

pub fn rpc_routes() -> Router {
    add_rpc_router()
}

fn main() {
    let _router = rpc_routes();
}
```

When this function is compiled for the native server, its written body runs.
When compiled for `wasm32`, calling `add(20, 22).await` serializes `(20, 22)`
and returns the decoded `RpcResult<SumResponse>`. Transport failures are
machine-readable `RpcFailure` values; they never become a fabricated default
application value.

## Mount the generated route inside server policy

Merge the generated router before applying the standard security baseline and
your domain layers:

```rust,no_run
# use rullst::{Router, server_function};
# use serde::{Deserialize, Serialize};
# #[derive(Deserialize, Serialize)]
# pub struct SumResponse { pub value: u32 }
# #[server_function(path = "/api/rpc/math/add")]
# pub async fn add(left: u32, right: u32) -> rullst::rpc::RpcResult<SumResponse> {
#     Ok(SumResponse { value: left.saturating_add(right) })
# }
fn secured_transport() -> Result<axum::Router, rullst::SecurityBaselineError> {
    let app = Router::new()
        .merge_axum(add_rpc_router().into_axum())
        .into_axum();

    rullst::apply_security_baseline(
        app,
        rullst::SecurityConfig::default(),
        rullst::config::Environment::Production,
    )
}

fn main() -> Result<(), rullst::SecurityBaselineError> {
    let _app = secured_transport()?;
    Ok(())
}
```

The production baseline verifies the double-submit CSRF cookie/header pair.
The Wasm caller reads the bounded `rullst_csrf` cookie and forwards it as
`X-CSRF-Token`. The application must still add session/authentication, trusted
tenant resolution, object/role authorization and rate limiting in the order
documented by `ProductionPreset`. Never accept role, owner or tenant authority
from a function argument.

## Failure codes

Application failures use the same lowercase dotted-code grammar as the shared
client contract:

```rust
fn capacity_failure() -> Result<rullst::rpc::RpcFailure, rullst::client_contract::ClientContractError> {
    rullst::rpc::RpcFailure::application("course.capacity_reached", false)
}

let failure = capacity_failure()?;
assert_eq!(failure.code(), "course.capacity_reached");
# Ok::<(), rullst::client_contract::ClientContractError>(())
```

Do not place provider bodies, database errors, PII, tokens or debug text in a
failure code. Log sensitive diagnostics only through an approved server-side
telemetry policy.

## Exact v12 limits

- zero to 16 simple identifier parameters;
- owned parameter types and one owned output type implementing the needed
  Serde traits;
- a concrete async free function with no generics, receiver, `unsafe`, extern
  ABI or variadic arguments;
- `rullst::rpc::RpcResult<T>` as the return type;
- an optional same-origin path below `/api/rpc/`, using at most 128 ASCII bytes;
- 256 KiB encoded request and response policy;
- JSON POST transport with a versioned envelope and request-ID correlation.

These bounds do not make a mutation exactly once. Put a stable idempotency key
in the domain payload and enforce it transactionally on the server when replay
would be harmful. Actual browser-engine compatibility, network availability
and deployed identity policy must be tested by the application.
