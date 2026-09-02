use rullst_macros::server_function;

#[server_function(path = "/api/rpc/one", path = "/api/rpc/two")]
async fn duplicate() -> RpcResult<()> {
    Ok(())
}

fn main() {}
