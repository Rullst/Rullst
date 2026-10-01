// src/blueprints/erp.rs — ERP Pocket blueprint: products, orders and stock with a
// Nexus CMS, in Active Record, Repository or Hybrid mode.
// The templates under `erp/src/` mirror the generated project's layout.
use super::common;

const ERP_STYLES: &str = include_str!("erp/styles.css");

/// Files every ERP variant emits unchanged, in manifest order.
const COMMON_FILES: [(&str, &str); 8] = [
    (
        "src/migrations/mod.rs",
        include_str!("erp/src/migrations/mod.rs.template"),
    ),
    (
        "src/migrations/m20260601000000_create_erp_tables.rs",
        include_str!("erp/src/migrations/m20260601000000_create_erp_tables.rs.template"),
    ),
    (
        "src/models/mod.rs",
        include_str!("erp/src/models/mod.rs.template"),
    ),
    (
        "src/models/product.rs",
        include_str!("erp/src/models/product.rs.template"),
    ),
    (
        "src/models/order.rs",
        include_str!("erp/src/models/order.rs.template"),
    ),
    (
        "src/controllers/mod.rs",
        include_str!("erp/src/controllers/mod.rs.template"),
    ),
    (
        "src/controllers/erp_controller.rs",
        include_str!("erp/src/controllers/erp_controller.rs.template"),
    ),
    (
        "src/pages/mod.rs",
        include_str!("erp/src/pages/mod.rs.template"),
    ),
];

pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let repo_mod_decl = common::repo_mod_decl(orm_pattern);
    let mut manifest = Vec::new();

    if hot_reload {
        manifest.push((
            "src/lib.rs",
            include_str!("erp/src/lib.rs.template").replace("__REPO_MOD_DECL__", repo_mod_decl),
        ));
        // The caller-supplied name is substituted last so it is never re-expanded.
        manifest.push((
            "src/main.rs",
            include_str!("erp/src/main.hot.rs.template")
                .replace("__REPO_MOD_DECL__", repo_mod_decl)
                .replace("__PROJECT_NAME_SAFE__", project_name_safe),
        ));
    } else {
        manifest.push((
            "src/main.rs",
            include_str!("erp/src/main.rs.template").replace("__REPO_MOD_DECL__", repo_mod_decl),
        ));
    }

    manifest.extend(
        COMMON_FILES
            .iter()
            .map(|(path, template)| (*path, (*template).to_string())),
    );
    manifest.push((
        "src/pages/erp.rs",
        include_str!("erp/src/pages/erp.rs.template").replace(
            "__FE_IMPORTS__",
            &common::frontend_page_imports(frontend_engine),
        ),
    ));
    manifest.push(("static/rullst.css", ERP_STYLES.to_string()));

    if common::is_repo_mode(orm_pattern) {
        manifest.push((
            "src/repositories/product_repository.rs",
            common::generate_repository("Product", "products"),
        ));
        manifest.push((
            "src/repositories/order_repository.rs",
            common::generate_repository("Order", "orders"),
        ));
        manifest.push((
            "src/repositories/mod.rs",
            common::generate_repositories_mod(&["Product", "Order"]),
        ));
    }

    manifest
}

#[cfg(test)]
mod tests {
    use super::file_manifest;

    #[test]
    fn every_erp_route_shares_the_admin_policy() {
        for hot_reload in [false, true] {
            let manifest =
                file_manifest("erp_app", hot_reload, "Active Record", "Zero-Bundle HTMX");
            let entry = if hot_reload {
                "src/lib.rs"
            } else {
                "src/main.rs"
            };
            let source = manifest
                .iter()
                .find_map(|(path, source)| (*path == entry).then_some(source.as_str()))
                .unwrap_or_default();
            let protected = source
                .split_once("let admin_routes = routes![")
                .and_then(|(_, rest)| rest.split_once("];"))
                .map(|(routes, _)| routes)
                .unwrap_or_default();
            // The dashboard lists every order's customer and the revenue totals.
            assert!(protected.contains("get(\"/\" => controllers::erp_controller::index)"));
            assert_eq!(source.matches("routes![").count(), 1, "{entry}");
            assert!(source.contains("admin_access.protect_router(admin_routes.into_axum())?"));
        }
        let library = file_manifest("erp_app", true, "Active Record", "Zero-Bundle HTMX");
        assert!(library.iter().any(|(path, source)| *path == "src/lib.rs"
            && source.contains("fn back_office_dashboard_is_not_public()")));
    }

    #[test]
    fn orders_reserve_stock_atomically_and_report_failures() {
        let manifest = file_manifest("erp_app", false, "Active Record", "Zero-Bundle HTMX");
        let controller = manifest
            .iter()
            .find_map(|(path, source)| {
                (*path == "src/controllers/erp_controller.rs").then_some(source.as_str())
            })
            .unwrap_or_default();
        // A read/compare/write let concurrent orders oversell, a negative
        // quantity added stock, and discarded save errors still redirected.
        assert!(!controller.contains("let _ = "));
        assert!(!controller.contains("stock -= payload.quantity"));
        assert!(!controller.contains("stock += 1"));
        assert!(controller.contains("WHERE id = ? AND stock >= ?\""));
        assert!(controller.contains("WHERE id = $2 AND stock >= $3\""));
        assert!(controller.contains("WHERE id = ? AND stock < ?\""));
        assert!(controller.contains(".begin().await?"));
        assert!(controller.contains("transaction.commit().await?"));
        assert!(controller.contains("!(1..=MAX_ORDER_QUANTITY).contains(&payload.quantity)"));
        assert!(
            controller.contains(
                "Ok(StockChange::UnknownProduct) => return rejected(StatusCode::NOT_FOUND"
            )
        );
        assert!(controller.contains("Err(error) => unavailable(error)"));
    }

    #[test]
    fn dashboard_totals_cover_every_row_and_lists_are_bounded() {
        let manifest = file_manifest("erp_app", false, "Active Record", "Zero-Bundle HTMX");
        let source = |name: &str| {
            manifest
                .iter()
                .find_map(|(path, source)| (*path == name).then_some(source.as_str()))
                .unwrap_or_default()
        };
        let controller = source("src/controllers/erp_controller.rs");
        let page = source("src/pages/erp.rs");
        // `all()` stops at the ORM's 1000-row cap with no ORDER BY, so totals
        // froze and recent orders showed the oldest rows; errors became zeros.
        assert!(!controller.contains("::all()"));
        assert!(!controller.contains("unwrap_or_default()"));
        assert!(controller.contains("SELECT SUM(total_price) FROM orders WHERE status = 'Paid'"));
        assert!(controller.contains("orders: Order::query().count().await?"));
        assert!(controller.contains("Product::query().where_lt(\"stock\", 6).count().await?"));
        assert!(controller.contains(".order_by(\"id\").paginate(page, PRODUCTS_PER_PAGE)"));
        assert!(controller.contains(".order_by_desc(\"id\").limit(RECENT_ORDERS)"));
        assert!(!page.contains("orders.len()"));
        assert!(!page.contains(".sum()"));
        assert!(page.contains("summary.revenue"));
    }
}
