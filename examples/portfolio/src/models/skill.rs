use rullst::db::{Nexus, Orm, RowSnapshot};

#[derive(Debug, Clone, Orm, Nexus)]
#[orm(table = "skills")]
pub struct Skill {
    pub id: i32,
    pub name: String,
    pub category: String,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}
