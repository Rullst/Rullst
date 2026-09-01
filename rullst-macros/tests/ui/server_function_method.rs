use rullst_macros::server_function;

struct Counter;

impl Counter {
    #[server_function]
    async fn increment(&self) -> u32 {
        1
    }
}

fn main() {}
