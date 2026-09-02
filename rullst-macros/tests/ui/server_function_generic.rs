use rullst_macros::server_function;

#[server_function]
async fn identity<T>(value: T) -> RpcResult<T> {
    Ok(value)
}

fn main() {}
