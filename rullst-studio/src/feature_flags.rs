use crate::access::{VerifiedLocalStudioAccess, verified_local_access_required};
use axum::{
    Router,
    extract::{Extension, Path},
    response::{Html, IntoResponse},
    routing::{get, post},
};

/// Raw feature-flag routes, without an access boundary.
///
/// [`crate::Studio::into_router`] mounts them behind the verified local
/// boundary. Toggling additionally requires the crate-private marker installed
/// by that boundary, so mounting this router elsewhere returns `403` for it.
pub fn router() -> Router {
    Router::new()
        .route("/", get(render_feature_flags))
        // rullst-access: admin — composed behind LocalStudioAccess::protect_router.
        .route("/toggle/{name}", post(toggle_feature_flag))
}

async fn ensure_table_exists() {
    if let Some(pool) = rullst_core::db::safe_pool() {
        let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
        let sql = match driver {
            "postgres" => {
                "CREATE TABLE IF NOT EXISTS rullst_feature_flags (
                    name VARCHAR(255) PRIMARY KEY,
                    enabled BOOLEAN NOT NULL DEFAULT false,
                    rollout_percentage INTEGER,
                    variants TEXT
                )"
            }
            "mysql" | "mariadb" => {
                "CREATE TABLE IF NOT EXISTS rullst_feature_flags (
                    name VARCHAR(255) PRIMARY KEY,
                    enabled BOOLEAN NOT NULL DEFAULT false,
                    rollout_percentage INTEGER,
                    variants TEXT
                )"
            }
            _ => {
                "CREATE TABLE IF NOT EXISTS rullst_feature_flags (
                    name TEXT PRIMARY KEY,
                    enabled INTEGER NOT NULL DEFAULT 0,
                    rollout_percentage INTEGER,
                    variants TEXT
                )"
            }
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
                    format!("<form method=\"post\" action=\"/studio/features/toggle/{encoded_name}\"><button type=\"submit\" class=\"bg-emerald-500 hover:bg-emerald-600 text-white px-3 py-1 rounded-full text-xs font-bold transition-colors\">ENABLED</button></form>")
                } else {
                    format!("<form method=\"post\" action=\"/studio/features/toggle/{encoded_name}\"><button type=\"submit\" class=\"bg-slate-700 hover:bg-slate-600 text-slate-300 px-3 py-1 rounded-full text-xs font-bold transition-colors\">DISABLED</button></form>")
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
    <link href="/studio/assets/studio.css" rel="stylesheet">
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

async fn toggle_feature_flag(
    Path(name): Path<String>,
    verified: Option<Extension<VerifiedLocalStudioAccess>>,
) -> axum::response::Response {
    if verified.is_none() {
        return verified_local_access_required();
    }
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

    let sql = if driver == "postgres" {
        "UPDATE rullst_feature_flags SET enabled = NOT enabled WHERE name = $1"
    } else {
        "UPDATE rullst_feature_flags SET enabled = NOT enabled WHERE name = ?"
    };
    let update = rullst_orm::_sqlx::query(sql).bind(name).execute(pool).await;
    match update {
        Ok(result) if result.rows_affected() == 1 => {}
        Ok(_) => {
            return (axum::http::StatusCode::NOT_FOUND, "Feature flag not found").into_response();
        }
        Err(error) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update feature flag: {error}"),
            )
                .into_response();
        }
    }

    rullst_core::DbFeatureDriver::invalidate_process_cache();

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

    fn toggle_request(name: &str, verified: bool) -> Request<Body> {
        let mut request = Request::builder()
            .method("POST")
            .uri(format!("/toggle/{name}"))
            .body(Body::empty())
            .unwrap();
        if verified {
            request
                .extensions_mut()
                .insert(crate::access::VerifiedLocalStudioAccess);
        }
        request
    }

    #[tokio::test]
    // TM-STUDIO-06: the raw router cannot toggle flags without the marker that
    // only the verified local Studio boundary installs.
    async fn raw_router_denies_toggles_without_verified_local_access() {
        let response = router()
            .oneshot(toggle_request("any_flag", false))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    #[cfg(not(miri))]
    #[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
    async fn denied_toggles_leave_the_stored_flag_unchanged() {
        let pool = crate::data_browser::pool::test_sqlite_pool().await;
        rullst_orm::_sqlx::query(
            "CREATE TABLE IF NOT EXISTS rullst_feature_flags (name TEXT PRIMARY KEY, \
             enabled INTEGER NOT NULL DEFAULT 0, rollout_percentage INTEGER, variants TEXT)",
        )
        .execute(pool)
        .await
        .unwrap();
        rullst_orm::_sqlx::query(
            "INSERT OR REPLACE INTO rullst_feature_flags (name, enabled) VALUES ('raw_router_probe', 0)",
        )
        .execute(pool)
        .await
        .unwrap();
        let enabled = || async {
            rullst_orm::_sqlx::query_scalar::<_, i64>(
                "SELECT enabled FROM rullst_feature_flags WHERE name = 'raw_router_probe'",
            )
            .fetch_one(pool)
            .await
            .unwrap()
        };

        let denied = router()
            .oneshot(toggle_request("raw_router_probe", false))
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        assert_eq!(enabled().await, 0);

        let allowed = router()
            .oneshot(toggle_request("raw_router_probe", true))
            .await
            .unwrap();
        assert!(allowed.status().is_redirection());
        assert_eq!(enabled().await, 1);
    }
}
