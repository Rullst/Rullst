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
