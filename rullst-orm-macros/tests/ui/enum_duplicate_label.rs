use rullst_orm_macros::Enum;

#[derive(Enum)]
enum Status {
    #[rullst_enum(rename = "same")]
    Active,
    #[rullst_enum(rename = "same")]
    Paused,
}

fn main() {}
