use rullst_orm::{Error, Orm};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "audit_records")]
pub(super) struct AuditRecord {
    pub(super) id: i32,
    pub(super) name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "audit_tenant_records", tenant_column = "tenant_id")]
pub(super) struct AuditTenantRecord {
    pub(super) id: i32,
    pub(super) tenant_id: String,
    pub(super) name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "audit_soft_records", soft_delete)]
pub(super) struct AuditSoftRecord {
    pub(super) id: i32,
    pub(super) name: String,
    pub(super) deleted_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(
    table = "audit_tenant_records",
    tenant_column = "tenant_id",
    global_scope = "visible"
)]
pub(super) struct AuditScopedRecord {
    pub(super) id: i32,
    pub(super) tenant_id: String,
    pub(super) name: String,
}

impl AuditScopedRecordQueryBuilder {
    fn visible(self) -> Self {
        self.where_eq("name", "alpha").or_where("name", "beta")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "audit_records", policy = "DenyDeletion")]
pub(super) struct AuditProtectedRecord {
    pub(super) id: i32,
    pub(super) name: String,
}

struct DenyDeletion;

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "audit_records")]
pub(super) struct AuditParent {
    pub(super) id: i32,
    pub(super) name: String,
    #[sqlx(skip)]
    #[orm(has_many = "AuditChild", foreign_key = "audit_record_id")]
    pub(super) children: Option<Vec<AuditChild>>,
    #[sqlx(skip)]
    #[orm(
        belongs_to_many = "AuditRole",
        pivot_table = "audit_parent_roles",
        foreign_key = "parent_id",
        related_key = "role_id"
    )]
    pub(super) roles: Option<Vec<AuditRole>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "audit_roles")]
pub(super) struct AuditRole {
    pub(super) id: i32,
    pub(super) name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "audit_children")]
pub(super) struct AuditChild {
    pub(super) id: i32,
    pub(super) audit_record_id: i32,
    pub(super) name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(table = "audit_records", after_fetch = "verify_exists")]
pub(super) struct AuditHookRecord {
    pub(super) id: i32,
    pub(super) name: String,
}

impl AuditHookRecord {
    async fn verify_exists(&mut self) -> Result<(), Error> {
        assert_eq!(
            AuditRecord::query().where_eq("id", self.id).count().await?,
            1
        );
        Ok(())
    }
}

#[async_trait::async_trait]
impl rullst_orm::Policy<AuditProtectedRecord> for DenyDeletion {
    async fn can_delete(_: &AuditProtectedRecord) -> Result<bool, Error> {
        Ok(false)
    }
}

static AUDIT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static INITIALIZED: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

pub(super) async fn database() -> tokio::sync::MutexGuard<'static, ()> {
    let guard = AUDIT_LOCK.lock().await;
    INITIALIZED
        .get_or_init(|| async {
            Orm::init_with_options("sqlite::memory:", 1, 5)
                .await
                .expect("initialize isolated audit database");
            let pool = Orm::pool().expect("audit pool");
            for sql in [
                "CREATE TABLE audit_records (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                "INSERT INTO audit_records VALUES (1, 'alpha'), (2, 'beta')",
                "CREATE TABLE audit_secrets (secret TEXT NOT NULL)",
                "INSERT INTO audit_secrets VALUES ('private fixture data')",
                "CREATE TABLE audit_children (id INTEGER PRIMARY KEY, audit_record_id INTEGER NOT NULL, name TEXT NOT NULL)",
                "INSERT INTO audit_children VALUES (1, 1, 'child alpha'), (2, 2, 'child beta')",
                "CREATE TABLE audit_roles (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                "CREATE TABLE audit_parent_roles (parent_id INTEGER NOT NULL, role_id INTEGER NOT NULL)",
                "CREATE TABLE audit_tenant_records (id INTEGER PRIMARY KEY, tenant_id TEXT NOT NULL, name TEXT NOT NULL)",
                "INSERT INTO audit_tenant_records VALUES (1, 'tenant-a', 'alpha'), (2, 'tenant-b', 'beta')",
                "CREATE TABLE audit_soft_records (id INTEGER PRIMARY KEY, name TEXT NOT NULL, deleted_at TEXT)",
                "INSERT INTO audit_soft_records VALUES (1, 'active', NULL), (2, 'deleted', '2026-01-01')",
            ] {
                rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql))
                    .execute(pool)
                    .await
                    .expect("create isolated audit fixture");
            }
        })
        .await;
    guard
}

pub(super) fn validation_error<T: std::fmt::Debug>(result: Result<T, Error>) {
    assert!(matches!(result, Err(Error::Validation(_))), "{result:?}");
}
