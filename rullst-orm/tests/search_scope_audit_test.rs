#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rullst_orm::{Error, Orm, SearchEngine, set_search_engine, with_tenant};

#[derive(Clone, Debug, rullst_orm::Orm, rullst_orm::FromRow)]
#[orm(
    table = "search_scope_records",
    searchable,
    tenant_column = "tenant_id",
    global_scope = "visible"
)]
struct SearchRecord {
    id: i32,
    tenant_id: String,
    name: String,
    visible: i32,
}

impl SearchRecordQueryBuilder {
    fn visible(self) -> Self {
        self.where_eq("visible", 1)
    }
}

struct BroadSearchEngine(Arc<AtomicUsize>);

#[rullst_orm::async_trait]
impl SearchEngine for BroadSearchEngine {
    async fn update(&self, _: &str, _: i32, _: serde_json::Value) -> Result<(), Error> {
        Ok(())
    }

    async fn delete(&self, _: &str, _: i32) -> Result<(), Error> {
        Ok(())
    }

    async fn search(&self, _: &str, query: &str) -> Result<Vec<i32>, Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(if query == "empty" {
            Vec::new()
        } else {
            vec![1, 2, 3]
        })
    }
}

#[tokio::test]
async fn local_and_external_search_enforce_model_scopes_and_empty_results() {
    Orm::init_with_options("sqlite::memory:", 1, 5)
        .await
        .expect("initialize isolated search fixture");
    let pool = Orm::pool().expect("pool");
    rullst_orm::_sqlx::query(
        "CREATE TABLE search_scope_records (id INTEGER PRIMARY KEY, tenant_id TEXT NOT NULL, name TEXT NOT NULL, visible INTEGER NOT NULL)",
    )
    .execute(pool)
    .await
    .expect("create search fixture");
    rullst_orm::_sqlx::query(
        "INSERT INTO search_scope_records VALUES (0, 'tenant-a', 'zero', 1), (1, 'tenant-a', 'needle', 1), (2, 'tenant-b', 'needle', 1), (3, 'tenant-a', 'needle', 0)",
    )
    .execute(pool)
    .await
    .expect("seed search fixture including a legitimate explicit zero ID");

    let mut violations = Vec::new();
    let missing_context = SearchRecord::search("needle").await.get().await;
    if !matches!(missing_context, Err(Error::Validation(_))) {
        violations.push("local search accepted missing tenant context");
    }
    let local_ids = with_tenant("tenant-a", async {
        SearchRecord::search("needle")
            .await
            .order_by("id")
            .pluck_i32("id")
            .await
            .expect("local search")
    })
    .await;
    if local_ids != [1] {
        violations.push("local search bypassed tenant or model-wide visibility scope");
    }

    let searches = Arc::new(AtomicUsize::new(0));
    set_search_engine(BroadSearchEngine(searches.clone())).expect("configure offline search");
    let missing_context = SearchRecord::search("needle").await.get().await;
    if !matches!(missing_context, Err(Error::Validation(_))) {
        violations.push("external search accepted missing tenant context");
    }
    if searches.load(Ordering::SeqCst) != 0 {
        violations.push("missing tenant context reached the external search provider");
    }
    let (external_ids, empty_ids) = with_tenant("tenant-a", async {
        let external_ids = SearchRecord::search("needle")
            .await
            .order_by("id")
            .pluck_i32("id")
            .await
            .expect("filter external IDs using database model scopes");
        let empty_ids = SearchRecord::search("empty")
            .await
            .pluck_i32("id")
            .await
            .expect("empty provider match must not select explicit ID zero");
        (external_ids, empty_ids)
    })
    .await;
    if external_ids != [1] {
        violations.push("external search bypassed tenant or model-wide visibility scope");
    }
    if !empty_ids.is_empty() {
        violations.push("empty provider results matched the explicit zero ID");
    }
    assert!(violations.is_empty(), "{}", violations.join("; "));
}
