use rullst_orm::{Orm, FromRow};

#[derive(Debug, Clone, FromRow, Orm)]
pub struct User {
    pub id: String,
}

fn main() {}
