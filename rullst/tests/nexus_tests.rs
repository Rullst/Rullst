use rullst::nexus::RullstNexus;
use rullst::routing::Router;
use rullst::testing::TestApp;
use rullst_orm::sqlx::FromRow;

#[derive(Debug, Clone, FromRow, rullst_orm::Eloquent)]
#[eloquent(table = "dummy_users")]
pub struct DummyUser {
    pub id: i32,
    pub name: String,
}

#[tokio::test]
async fn test_nexus_dashboard_router_compiles_and_renders() {
    let nexus = RullstNexus::new("/admin").register::<DummyUser>();

    // TestApp needs a custom Rullst Router wrapper, but RullstNexus returns axum::Router
    // We can nest it using nest_axum
    let router = Router::new().nest_axum("/admin", nexus.into_router());

    let app = TestApp::new(router.into_axum());

    let res = app.get("/admin").send().await;

    // We expect the Nexus Dashboard to return 200 OK and show the title.
    res.assert_status(200);
    res.assert_see("Rullst Nexus Dashboard");
    res.assert_see("dummy_users"); // Should list the table
}

#[tokio::test]
async fn test_nexus_table_view() {
    let nexus = RullstNexus::new("/admin").register::<DummyUser>();

    let router = Router::new().nest_axum("/admin", nexus.into_router());

    let app = TestApp::new(router.into_axum());

    let res = app.get("/admin/tables/dummy_users").send().await;

    // The table view should return 200 OK and display the table placeholder
    res.assert_status(200);
    res.assert_see("dummy_users");
}
