use rullst_macros::server_function;

struct Counter;

impl Counter {
    #[server_function]
    async fn increment(&self) -> RpcResult<u32> {
        Ok(1)
    }
}

fn main() {}
