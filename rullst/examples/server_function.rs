use rullst::server_function;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Greeting {
    pub message: String,
}

#[server_function(path = "/api/rpc/examples/greet")]
pub async fn greet(name: String) -> rullst::rpc::RpcResult<Greeting> {
    Ok(Greeting {
        message: format!("Hello, {name}!"),
    })
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let _router = greet_rpc_router();
}
