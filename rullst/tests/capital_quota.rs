#![cfg(feature = "capital-quota-sql")]

use rullst::{
    capital::{BillingSubject, InMemoryQuotaStore, QuotaRequest, QuotaStore as _, SqlQuotaStore},
    security::TenantContext,
};

#[tokio::test]
async fn umbrella_exposes_shared_quota_and_sql_store() {
    let tenant = TenantContext::try_new("facade-workspace").expect("tenant");
    let subject = BillingSubject::from_tenant(&tenant).expect("billing subject");
    let request = QuotaRequest::try_new(subject.clone(), "projects", "create-1", 1, 2)
        .expect("quota request");
    let local = InMemoryQuotaStore::default();
    let grant = local.reserve(&request).await.expect("facade reservation");
    assert_eq!(grant.used_after(), 1);

    let sql = SqlQuotaStore::connect("sqlite::memory:")
        .await
        .expect("facade SQL store");
    sql.prepare_schema().await.expect("facade quota schema");
    assert_eq!(sql.reserve(&request).await.unwrap().used_after(), 1);
}
