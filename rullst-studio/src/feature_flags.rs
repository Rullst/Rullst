use axum::{
    Router,
    extract::Path,
    response::{Html, IntoResponse},
    routing::{get, post},
};

pub fn router() -> Router {
    Router::new()
        .route("/", get(render_feature_flags))
        .route("/toggle/{name}", post(toggle_feature_flag))
}

async fn ensure_table_exists() {
    if let Some(pool) = rullst_core::db::safe_pool() {
        let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
        let sql = if driver == "postgres" {
            "CREATE TABLE IF NOT EXISTS rullst_feature_flags (
                name VARCHAR(255) PRIMARY KEY,
                enabled BOOLEAN NOT NULL DEFAULT false,
                rollout_percentage INTEGER,
                variants TEXT
            )"
        } else {
            "CREATE TABLE IF NOT EXISTS rullst_feature_flags (
                name TEXT PRIMARY KEY,
                enabled INTEGER NOT NULL DEFAULT 0,
                rollout_percentage INTEGER,
                variants TEXT
            )"
        };
        let _ = rullst_orm::_sqlx::query(sql).execute(pool).await;
    }
}

async fn render_feature_flags() -> Html<String> {
    ensure_table_exists().await;

    let mut rows_html = String::new();

    if let Some(pool) = rullst_core::db::safe_pool() {
        let _driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
        if let Ok(rows) = rullst_orm::_sqlx::query("SELECT name, enabled, rollout_percentage, variants FROM rullst_feature_flags ORDER BY name ASC").fetch_all(pool).await {
            use sqlx::Row;
            for row in rows {
                let name = row.try_get::<String, _>("name").unwrap_or_default();
                let enabled = row.try_get::<i32, _>("enabled").map(|v| v != 0).or_else(|_| row.try_get::<bool, _>("enabled")).unwrap_or(false);
                let rollout = row.try_get::<i32, _>("rollout_percentage").map(|v| v.to_string()).unwrap_or_else(|_| "-".to_string());
                let variants = row.try_get::<String, _>("variants").unwrap_or_else(|_| "-".to_string());
                let encoded_name = urlencoding::encode(&name);

                let toggle_btn = if enabled {
                    format!("<button hx-post=\"/studio/features/toggle/{encoded_name}\" hx-target=\"body\" class=\"bg-emerald-500 hover:bg-emerald-600 text-white px-3 py-1 rounded-full text-xs font-bold transition-colors\">ENABLED</button>")
                } else {
                    format!("<button hx-post=\"/studio/features/toggle/{encoded_name}\" hx-target=\"body\" class=\"bg-slate-700 hover:bg-slate-600 text-slate-300 px-3 py-1 rounded-full text-xs font-bold transition-colors\">DISABLED</button>")
                };

                rows_html.push_str(&format!(
                    "<tr class=\"border-b border-slate-800 hover:bg-slate-800/50 transition-colors\">\
                     <td class=\"py-4 px-4 font-semibold text-slate-200\">{}</td>\
                     <td class=\"py-4 px-4\">{}</td>\
                     <td class=\"py-4 px-4 text-slate-400\">{}</td>\
                     <td class=\"py-4 px-4 text-slate-400\">{}</td>\
                     </tr>",
                    rullst_core::html::escape_str(&name),
                    toggle_btn,
                    rollout,
                    rullst_core::html::escape_str(&variants)
                ));
            }
        }
    }

    if rows_html.is_empty() {
        rows_html = "<tr><td colspan=\"4\" class=\"py-8 text-center text-slate-500\">No feature flags found. (Table <code>rullst_feature_flags</code> is empty)</td></tr>".to_string();
    }

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Feature Flags - Rullst Studio</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="https://unpkg.com/htmx.org@2.0.4" defer></script>
</head>
<body class="h-full flex flex-col font-mono p-8">
    <div class="max-w-6xl mx-auto w-full">
        <div class="flex items-center justify-between mb-8">
            <h1 class="text-3xl font-bold text-emerald-400 flex items-center gap-3">
                <a href="/studio" class="text-slate-500 hover:text-emerald-400 transition-colors">←</a>
                Feature Flags Manager
            </h1>
        </div>
        
        <div class="bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl">
            <table class="w-full text-left border-collapse">
                <thead>
                    <tr class="bg-slate-950/50 border-b border-slate-800">
                        <th class="py-3 px-4 text-slate-400 font-medium w-1/3">Flag Name</th>
                        <th class="py-3 px-4 text-slate-400 font-medium">Status</th>
                        <th class="py-3 px-4 text-slate-400 font-medium">Rollout %</th>
                        <th class="py-3 px-4 text-slate-400 font-medium">Variants</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    </div>
</body>
</html>"#,
        rows_html
    ))
}

async fn toggle_feature_flag(Path(name): Path<String>) -> axum::response::Response {
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    toggle_feature_flag_with_pool(&name, rullst_core::db::safe_pool(), driver).await
}

async fn toggle_feature_flag_with_pool(
    name: &str,
    pool: Option<&rullst_core::db::RullstPool>,
    driver: &str,
) -> axum::response::Response {
    let Some(pool) = pool else {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "Database is not configured",
        )
            .into_response();
    };

    let select_sql = if driver == "postgres" {
        "SELECT enabled FROM rullst_feature_flags WHERE name = $1"
    } else {
        "SELECT enabled FROM rullst_feature_flags WHERE name = ?"
    };
    let row = match rullst_orm::_sqlx::query(select_sql)
        .bind(name)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            return (axum::http::StatusCode::NOT_FOUND, "Feature flag not found").into_response();
        }
        Err(error) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to load feature flag: {error}"),
            )
                .into_response();
        }
    };

    use sqlx::Row;
    let current_enabled = row
        .try_get::<i32, _>("enabled")
        .map(|v| v != 0)
        .or_else(|_| row.try_get::<bool, _>("enabled"))
        .unwrap_or(false);

    let sql = if driver == "postgres" {
        "UPDATE rullst_feature_flags SET enabled = $1 WHERE name = $2"
    } else {
        "UPDATE rullst_feature_flags SET enabled = ? WHERE name = ?"
    };

    let update = if driver == "sqlite" {
        rullst_orm::_sqlx::query(sql)
            .bind(if current_enabled { 0 } else { 1 })
            .bind(name)
            .execute(pool)
            .await
    } else {
        rullst_orm::_sqlx::query(sql)
            .bind(!current_enabled)
            .bind(name)
            .execute(pool)
            .await
    };
    if let Err(error) = update {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update feature flag: {error}"),
        )
            .into_response();
    }

    axum::response::Redirect::to("/studio/features").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_feature_flags_endpoints() {
        let app = router();

        // 1. GET /
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 2. Mutations fail closed when Studio has no configured database. Invoke the extracted
        // dependency boundary directly so this assertion cannot race another test's global pool.
        let toggle_resp = toggle_feature_flag_with_pool("new_ai_copilot", None, "sqlite").await;
        assert_eq!(toggle_resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            !toggle_resp
                .headers()
                .contains_key(axum::http::header::LOCATION)
        );
        let body = axum::body::to_bytes(toggle_resp.into_body(), 1_024)
            .await
            .expect("bounded response body");
        assert_eq!(&body[..], b"Database is not configured");

        // 3. Directly check render_feature_flags HTML
        let html = render_feature_flags().await.0;
        assert!(html.contains("Feature Flags Manager"));
    }
}
