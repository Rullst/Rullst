use rullst::db::{Orm, FromRow, Nexus};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "profiles")]
pub struct Profile {
    pub id: i32,
    pub name: String,
    pub title: String,
    pub subtitle: String,
    pub email: String,
    pub website: String,
    pub avatar_url: String,
    pub github_url: String,
    pub linkedin_url: String,
}
