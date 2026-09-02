use rullst_orm_macros::Enum;

#[derive(Enum)]
enum Status {
    Active(String),
}

fn main() {}
