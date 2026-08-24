use rullst_macros::server_function;

#[server_function(path = "/sum")]
async fn sum(left: u32, right: u32) -> u32 {
    left + right
}

fn main() {}
