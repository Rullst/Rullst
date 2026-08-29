//! Database schema guidance for Studio.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationStatusItem {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

/// Renders schema inspection and explicit CLI guidance. Studio cannot discover
/// an application's compiled migration/seeder registry, so it must not report a
/// successful empty execution.
pub fn render_migration_manager_html(schema_tables_html: &str) -> String {
    format!(
        r#"
<div class="max-w-6xl mx-auto p-6 space-y-6">
  <div class="bg-slate-800/80 p-6 rounded-2xl border border-slate-700/60 shadow-xl backdrop-blur-md space-y-4">
    <h2 class="text-2xl font-bold text-slate-100">Database schema tools</h2>
    <p class="text-sm text-slate-300">
      This Studio instance can inspect the configured schema, but no application
      migration or seeder registry was supplied to it. Run the explicit CLI
      commands from the project root and review their terminal output:
    </p>
    <pre class="bg-slate-950 border border-slate-800 rounded-xl p-4 text-sm text-slate-200"><code>cargo rullst db:migrate
cargo rullst db:rollback
cargo rullst db:seed</code></pre>
    <p class="text-xs text-amber-300">
      The compatibility mutation handlers return 501 instead of pretending that
      an empty registry changed the database.
    </p>
  </div>
  {schema_tables_html}
</div>
"#
    )
}

fn unavailable(operation: &str) -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse {
            success: false,
            message: format!(
                "Studio cannot {operation} without an explicitly supplied application registry; use the project CLI"
            ),
        }),
    )
        .into_response()
}

pub async fn handle_run_migrations() -> Response {
    unavailable("run migrations")
}

pub async fn handle_rollback_migrations() -> Response {
    unavailable("roll back migrations")
}

pub async fn handle_run_seeders() -> Response {
    unavailable("run seeders")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migration_surface_never_reports_empty_registry_success() {
        let html = render_migration_manager_html("<div id=\"tables\">Table List</div>");
        assert!(html.contains("Database schema tools"));
        assert!(html.contains("cargo rullst db:migrate"));
        assert!(html.contains("Table List"));

        for response in [
            handle_run_migrations().await,
            handle_rollback_migrations().await,
            handle_run_seeders().await,
        ] {
            assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        }
    }
}
