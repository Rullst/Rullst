use rullst_orm_macros::Enum;

#[derive(Enum)]
enum Status {
    #[rullst_enum(rename = "owner's")]
    Active,
}

fn main() {}
