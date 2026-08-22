use axum::{Router, response::Html, routing::get};

pub fn router() -> Router {
    Router::new().route("/", get(render_er_diagram))
}

async fn get_sqlite_schema() -> String {
    let mut diagram = String::from("erDiagram\n");
    if let Some(pool) = rullst_core::db::safe_pool() {
        use sqlx::Row;
        if let Ok(tables) = rullst_orm::_sqlx::query("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations'")
            .fetch_all(pool)
            .await
        {
            for t_row in tables {
                let table_name = t_row.try_get::<String, _>("name").unwrap_or_default();
                diagram.push_str(&format!("    {} {{\n", table_name));

                let q_col = format!("PRAGMA table_info(\"{}\")", table_name);
                if let Ok(columns) = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(q_col.as_str()))
                    .fetch_all(pool)
                    .await
                {
                    for c_row in columns {
                        let col_name = c_row.try_get::<String, _>("name").unwrap_or_default();
                        let col_type = c_row.try_get::<String, _>("type").unwrap_or_else(|_| "text".to_string()).to_lowercase();
                        let pk = c_row.try_get::<i32, _>("pk").unwrap_or(0);

                        let pk_str = if pk > 0 { " PK" } else { "" };

                        diagram.push_str(&format!("        {} {}{}\n", col_type, col_name, pk_str));
                    }
                }
                diagram.push_str("    }\n");

                let q_fk = format!("PRAGMA foreign_key_list(\"{}\")", table_name);
                if let Ok(fks) = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(q_fk.as_str()))
                    .fetch_all(pool)
                    .await
                {
                    for fk_row in fks {
                        let to_table = fk_row.try_get::<String, _>("table").unwrap_or_default();
                        let from_col = fk_row.try_get::<String, _>("from").unwrap_or_default();
                        let to_col = fk_row.try_get::<String, _>("to").unwrap_or_default();
                        diagram.push_str(&format!("    {} }}|--|| {} : \"{}.{} -> {}.{}\"\n", table_name, to_table, table_name, from_col, to_table, to_col));
                    }
                }
            }
        }
    }
    diagram
}

async fn get_postgres_schema() -> String {
    let mut diagram = String::from("erDiagram\n");
    if let Some(pool) = rullst_core::db::safe_pool() {
        use sqlx::Row;

        let table_query = "
            SELECT table_name 
            FROM information_schema.tables 
            WHERE table_schema = 'public' AND table_name != '_sqlx_migrations'
        ";

        if let Ok(tables) = rullst_orm::_sqlx::query(table_query).fetch_all(pool).await {
            for t_row in tables {
                let table_name = t_row.try_get::<String, _>("table_name").unwrap_or_default();
                diagram.push_str(&format!("    {} {{\n", table_name));

                let col_query = format!(
                    "
                    SELECT column_name, data_type 
                    FROM information_schema.columns 
                    WHERE table_schema = 'public' AND table_name = '{}'
                ",
                    table_name
                );

                if let Ok(columns) =
                    rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(col_query.as_str()))
                        .fetch_all(pool)
                        .await
                {
                    for c_row in columns {
                        let col_name = c_row
                            .try_get::<String, _>("column_name")
                            .unwrap_or_default();
                        let col_type = c_row
                            .try_get::<String, _>("data_type")
                            .unwrap_or_else(|_| "text".to_string());

                        diagram.push_str(&format!(
                            "        {} {}\n",
                            col_type.replace(" ", "_"),
                            col_name
                        ));
                    }
                }
                diagram.push_str("    }\n");

                let fk_query = format!(
                    "
                    SELECT
                        kcu.column_name,
                        ccu.table_name AS foreign_table_name,
                        ccu.column_name AS foreign_column_name 
                    FROM 
                        information_schema.table_constraints AS tc 
                        JOIN information_schema.key_column_usage AS kcu
                          ON tc.constraint_name = kcu.constraint_name
                          AND tc.table_schema = kcu.table_schema
                        JOIN information_schema.constraint_column_usage AS ccu
                          ON ccu.constraint_name = tc.constraint_name
                          AND ccu.table_schema = tc.table_schema
                    WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_name='{}'
                ",
                    table_name
                );

                if let Ok(fks) =
                    rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(fk_query.as_str()))
                        .fetch_all(pool)
                        .await
                {
                    for fk_row in fks {
                        let from_col = fk_row
                            .try_get::<String, _>("column_name")
                            .unwrap_or_default();
                        let to_table = fk_row
                            .try_get::<String, _>("foreign_table_name")
                            .unwrap_or_default();
                        let to_col = fk_row
                            .try_get::<String, _>("foreign_column_name")
                            .unwrap_or_default();

                        diagram.push_str(&format!(
                            "    {} }}|--|| {} : \"{}.{} -> {}.{}\"\n",
                            table_name, to_table, table_name, from_col, to_table, to_col
                        ));
                    }
                }
            }
        }
    }
    diagram
}

async fn render_er_diagram() -> Html<String> {
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");

    let diagram = if driver == "postgres" {
        get_postgres_schema().await
    } else {
        get_sqlite_schema().await
    };

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>ER Diagram - Rullst Studio</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://cdn.tailwindcss.com"></script>
    <script type="module">
        import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
        mermaid.initialize({{ 
            startOnLoad: true, 
            theme: 'dark',
            securityLevel: 'loose'
        }});
    </script>
</head>
<body class="h-full flex flex-col font-mono p-8">
    <div class="max-w-7xl mx-auto w-full h-full flex flex-col">
        <div class="flex items-center justify-between mb-8 flex-shrink-0">
            <h1 class="text-3xl font-bold text-emerald-400 flex items-center gap-3">
                <a href="/studio" class="text-slate-500 hover:text-emerald-400 transition-colors">←</a>
                Visual ER Diagram
            </h1>
        </div>
        
        <div class="bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl flex-1 p-8 flex items-center justify-center">
            <pre class="mermaid">
{}
            </pre>
        </div>
    </div>
</body>
</html>"#,
        diagram
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_er_diagram_endpoints() {
        let app = router();

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let html = render_er_diagram().await.0;
        assert!(html.contains("Visual ER Diagram"));
        assert!(html.contains("class=\"mermaid\""));
        assert!(html.contains("erDiagram"));

        let sqlite_schema = get_sqlite_schema().await;
        assert!(sqlite_schema.starts_with("erDiagram"));

        let pg_schema = get_postgres_schema().await;
        assert!(pg_schema.starts_with("erDiagram"));
    }
}
