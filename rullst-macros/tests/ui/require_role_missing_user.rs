use rullst_macros::require_role;

#[require_role("Admin")]
async fn dashboard(value: String) {
    drop(value);
}

fn main() {}
