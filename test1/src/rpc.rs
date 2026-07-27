use rullst::server_function;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CounterResponse {
    pub new_value: i32,
    pub message: String,
}

#[server_function]
pub async fn increment_counter(current: i32) -> CounterResponse {
    CounterResponse {
        new_value: current + 1,
        message: format!("Successfully incremented on the server!"),
    }
}
