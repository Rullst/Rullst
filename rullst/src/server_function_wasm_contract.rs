//! Compile-only proof that the facade's public server-function expansion and
//! browser transport stay compatible on `wasm32-unknown-unknown`.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct WasmContractOutput {
    value: u32,
}

#[allow(dead_code)]
#[crate::server_function(path = "/api/rpc/contract/wasm")]
async fn wasm_contract(value: u32) -> crate::rpc::RpcResult<WasmContractOutput> {
    Ok(WasmContractOutput { value })
}
