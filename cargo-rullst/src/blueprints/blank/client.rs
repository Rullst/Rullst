pub(super) fn rpc_source() -> String {
    r#"use rullst::server_function;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct CounterResponse {
    pub new_value: i32,
    pub message: String,
}

#[server_function]
pub async fn increment_counter() -> rullst::rpc::RpcResult<CounterResponse> {
    Ok(CounterResponse {
        new_value: 1,
        message: "Successfully incremented on the server!".to_owned(),
    })
}
"#
    .to_owned()
}
