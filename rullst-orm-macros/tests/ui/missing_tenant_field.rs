use rullst_orm_macros::Orm;

#[derive(Orm)]
#[orm(table = "records", tenant_column = "tenant_id")]
struct Record {
    id: i32,
}

fn main() {}
