use rullst::db::{FromRow, Nexus, Orm, RowSnapshot};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "projects")]
pub struct Project {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub url: String,
    pub tags: String,
    pub is_featured: i32,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}
