use rullst_macros::server_function;

#[server_function]
async fn sum((left, right): (u32, u32)) -> RpcResult<u32> {
    Ok(left + right)
}

fn main() {}
