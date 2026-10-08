use rullst::db::{Orm, FromRow};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "notes")]
pub struct Note {
    pub id: i32,
    /// The id of the user who wrote the note; set from the session, never from the form.
    pub user_id: i32,
    pub title: String,
    pub body: String,
}
