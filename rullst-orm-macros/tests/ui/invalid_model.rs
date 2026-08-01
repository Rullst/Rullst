use rullst_orm_macros::Model;

#[derive(Model)]
struct InvalidUser {
    id: i32,
    // Error: The struct must have either #[derive(Model)] with #[table(...)] or some other expected configuration.
    // We'll just write an invalid macro usage and let trybuild capture the exact compiler error.
}

fn main() {}
