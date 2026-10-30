use rullst::db::{Nexus, Orm, RowSnapshot};

#[derive(Debug, Clone, Orm, Nexus)]
#[orm(table = "experiences")]
pub struct Experience {
    pub id: i32,
    pub role: String,
    pub company: String,
    pub period: String,
    pub description: String,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}
