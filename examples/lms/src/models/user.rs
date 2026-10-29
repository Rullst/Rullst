use rullst::db::{FromRow, Orm, RowSnapshot};
use rullst::nexus::{FieldKind, FieldMeta, NexusModel};
use rullst::orm::chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default, FromRow, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[orm(hidden)]
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Lets this account into Nexus (`/nexus`) under the application's
    // sign-in. The database manages this column: ORM inserts and saves never
    // write it, so neither a form nor `save()` grants it, while queries can
    // still filter on it (`where_eq("is_admin", true)`). Set it to true with
    // your database client for the first administrator, who can then grant
    // others in Nexus.
    #[orm(db_default)]
    pub is_admin: bool,
    // Login cookies carry the epoch they were issued with. ORM inserts and
    // saves never write this column, so `save()` never undoes
    // `revoke_sessions`.
    #[orm(db_default)]
    pub session_epoch: i64,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}

impl User {
    pub async fn find_by_email(email: &str) -> Result<Option<Self>, rullst_orm::Error> {
        Self::query()
            .where_eq("email", email.to_owned())
            .first()
            .await
    }

    /// The epoch to put in this account's login cookies, or `None` when the
    /// column holds a negative value, which no generated code writes.
    pub fn current_session_epoch(&self) -> Option<u64> {
        u64::try_from(self.session_epoch).ok()
    }

    /// Whether a decoded login cookie was issued for this account at its
    /// current session epoch.
    pub fn accepts_session(&self, claims: &rullst::auth::SessionClaims) -> bool {
        claims.user_id == self.id
            && self
                .current_session_epoch()
                .is_some_and(|epoch| claims.verify_epoch(epoch).is_ok())
    }

    /// Signs out every device of this account: one UPDATE increments
    /// `session_epoch`, so each login cookie issued before stops matching.
    /// "Sign out everywhere" calls it; call it too when you deactivate or
    /// delete the account. Password changes and resets bump the epoch in the
    /// same UPDATE that stores the new hash.
    pub async fn revoke_sessions(id: i32) -> Result<(), rullst::orm::Error> {
        let increment = if rullst::db::Orm::driver()? == "postgres" {
            "UPDATE users SET session_epoch = session_epoch + 1 WHERE id = $1"
        } else {
            "UPDATE users SET session_epoch = session_epoch + 1 WHERE id = ?"
        };
        rullst::db::sqlx::query(increment)
            .bind(id)
            .execute(rullst::db::Orm::pool()?)
            .await?;
        Ok(())
    }

    /// Stores a new password hash and signs out every device in one UPDATE,
    /// but only while the account is still at `epoch`, the epoch of the
    /// cookie that asked. Returns the new epoch for that browser's next
    /// cookie, or `None` when its sessions were revoked meanwhile and nothing
    /// changed.
    pub async fn change_password(
        id: i32,
        epoch: u64,
        password_hash: &str,
        updated_at: rullst::orm::chrono::DateTime<rullst::orm::chrono::Utc>,
    ) -> Result<Option<u64>, rullst::orm::Error> {
        let (Ok(current), Some(next)) = (i64::try_from(epoch), epoch.checked_add(1)) else {
            return Ok(None);
        };
        let update = if rullst::db::Orm::driver()? == "postgres" {
            "UPDATE users SET password_hash = $1, updated_at = $2, session_epoch = session_epoch + 1 WHERE id = $3 AND session_epoch = $4"
        } else {
            "UPDATE users SET password_hash = ?, updated_at = ?, session_epoch = session_epoch + 1 WHERE id = ? AND session_epoch = ?"
        };
        let changed = rullst::db::sqlx::query(update)
            .bind(password_hash)
            .bind(updated_at)
            .bind(id)
            .bind(current)
            .execute(rullst::db::Orm::pool()?)
            .await?
            .rows_affected();
        Ok((changed == 1).then_some(next))
    }
}

impl NexusModel for User {
    fn nexus_table() -> &'static str {
        "users"
    }
    fn nexus_label() -> &'static str {
        "Learners"
    }
    fn nexus_icon() -> &'static str {
        "👥"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta {
                name: "id",
                label: "ID",
                kind: FieldKind::Number,
                hidden: true,
                readonly: true,
            },
            FieldMeta {
                name: "name",
                label: "Name",
                kind: FieldKind::Text,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "email",
                label: "Email",
                kind: FieldKind::Text,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "password_hash",
                label: "Password Hash",
                kind: FieldKind::Text,
                hidden: true,
                readonly: true,
            },
            FieldMeta {
                name: "oauth_provider",
                label: "OAuth Provider",
                kind: FieldKind::Text,
                hidden: false,
                readonly: true,
            },
            FieldMeta {
                name: "oauth_id",
                label: "OAuth ID",
                kind: FieldKind::Text,
                hidden: true,
                readonly: true,
            },
            FieldMeta {
                name: "is_admin",
                label: "Nexus administrator",
                kind: FieldKind::Boolean,
                hidden: false,
                readonly: false,
            },
            FieldMeta {
                name: "created_at",
                label: "Created At",
                kind: FieldKind::DateTimeUtc,
                hidden: false,
                readonly: true,
            },
            FieldMeta {
                name: "updated_at",
                label: "Updated At",
                kind: FieldKind::DateTimeUtc,
                hidden: false,
                readonly: true,
            },
        ]
    }
}
