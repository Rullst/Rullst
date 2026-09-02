use rullst_macros::server_function;

#[server_function(path = "https://example.test/api/rpc/sum")]
async fn sum(left: u32, right: u32) -> RpcResult<u32> {
    Ok(left + right)
}

fn main() {}
