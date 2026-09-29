use rullst::db::{Orm, FromRow, Nexus};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "skills")]
pub struct Skill {
    pub id: i32,
    pub name: String,
    pub category: String,
}
