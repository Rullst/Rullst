use rullst::db::{Orm, FromRow, Nexus};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "experiences")]
pub struct Experience {
    pub id: i32,
    pub role: String,
    pub company: String,
    pub period: String,
    pub description: String,
}
