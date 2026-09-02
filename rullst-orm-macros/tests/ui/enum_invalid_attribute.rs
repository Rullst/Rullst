use rullst_orm_macros::Enum;

#[derive(Enum)]
#[rullst_enum(rename_all = "SCREAMING_SNAKE_CASE")]
enum Status {
    Active,
}

fn main() {}
