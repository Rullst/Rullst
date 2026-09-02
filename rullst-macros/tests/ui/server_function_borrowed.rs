use rullst_macros::server_function;

#[server_function]
async fn greet(name: &str) -> RpcResult<String> {
    Ok(name.to_owned())
}

fn main() {}
