//! `search()` returns engine matches in the engine's ranking on every driver
//! (the ranked IDs are bound into an `ORDER BY CASE`), unless the caller
//! orders explicitly.

use std::sync::Mutex;

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{Error, FromRow, Orm, SearchEngine, set_search_engine};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "contract_articles", searchable)]
struct ContractArticle {
    id: i32,
    title: String,
    category: String,
}

static RANKING: Mutex<Vec<i32>> = Mutex::new(Vec::new());

/// Answers every query on `contract_articles` with `RANKING`, best first.
struct RankedIndex;

#[rullst_orm::async_trait]
impl SearchEngine for RankedIndex {
    async fn update(&self, _: &str, _: i32, _: serde_json::Value) -> Result<(), Error> {
        Ok(())
    }

    async fn delete(&self, _: &str, _: i32) -> Result<(), Error> {
        Ok(())
    }

    async fn search(&self, table: &str, _: &str) -> Result<Vec<i32>, Error> {
        if table != "contract_articles" {
            return Ok(Vec::new());
        }
        Ok(RANKING.lock().expect("ranking lock").clone())
    }
}

fn ids(articles: &[ContractArticle]) -> Vec<i32> {
    articles.iter().map(|article| article.id).collect()
}

pub(super) async fn exercise() {
    let driver = Orm::driver().expect("initialized driver");
    set_search_engine(RankedIndex).expect("configure the ranked contract index");
    Schema::create("contract_articles", |table: &mut Blueprint| {
        table.id();
        table.string("title").not_null();
        table.string("category").not_null();
    })
    .await
    .expect("create search contract table");
    let mut inserted = Vec::new();
    for (title, category) in [("alpha", "news"), ("beta", "news"), ("gamma", "blog")] {
        let mut article = ContractArticle {
            id: 0,
            title: title.to_string(),
            category: category.to_string(),
        };
        article.save().await.expect("insert article");
        inserted.push(article.id);
    }
    let ranking = vec![inserted[2], inserted[0], inserted[1]];
    *RANKING.lock().expect("ranking lock") = ranking.clone();

    let search = ContractArticle::search("ranked").await;
    let ranked = search
        .get()
        .await
        .unwrap_or_else(|error| panic!("{driver} ranked search: {error}"));
    assert_eq!(ids(&ranked), ranking, "{driver} keeps the engine ranking");
    let best = search.first().await.expect("best hit").expect("a hit");
    assert_eq!(best.id, ranking[0], "{driver} first() is the best hit");
    let page = search.paginate(1, 2).await.expect("first page");
    assert_eq!((ids(&page.data), page.total), (ranking[..2].to_vec(), 3));
    assert_eq!(search.pluck_i32("id").await.expect("pluck"), ranking);
    let explicit = search.clone().order_by("id").get().await.expect("ordered");
    assert_eq!(ids(&explicit), inserted, "{driver} order_by() replaces it");
    let distinct = search
        .clone()
        .distinct()
        .get()
        .await
        .unwrap_or_else(|error| panic!("{driver} distinct search: {error}"));
    assert_eq!(distinct.len(), 3);
    let mut categories = search
        .clone()
        .group_by("category")
        .pluck_string("category")
        .await
        .unwrap_or_else(|error| panic!("{driver} grouped search: {error}"));
    categories.sort();
    assert_eq!(categories, vec!["blog".to_string(), "news".to_string()]);
    let deleted = search
        .delete_all()
        .await
        .unwrap_or_else(|error| panic!("{driver} delete_all of search hits: {error}"));
    assert_eq!(deleted, 3);

    Schema::drop_if_exists("contract_articles")
        .await
        .expect("drop search contract table");
}
