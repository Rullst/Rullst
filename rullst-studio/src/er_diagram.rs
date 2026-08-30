//! Live, read-only ER diagram generated from the configured relational schema.

use axum::{Router, response::Html, routing::get};
use sqlx::{QueryBuilder, Row};

type SchemaRow = <rullst_orm::RullstDatabase as sqlx::Database>::Row;

pub fn router() -> Router {
    Router::new().route("/", get(render_er_diagram))
}

fn mermaid_identifier(value: &str) -> String {
    let mut result = String::with_capacity(value.len().min(64));
    for character in value.chars() {
        if result.len() == 64 {
            break;
        }
        result.push(if character.is_ascii_alphanumeric() || character == '_' {
            character
        } else {
            '_'
        });
    }
    if result.is_empty() {
        result.push_str("unnamed");
    } else if result.as_bytes()[0].is_ascii_digit() {
        result.insert_str(0, "entity_");
    }
    result
}

fn row_is_primary_key(row: &SchemaRow, column: &str) -> bool {
    row.try_get::<i32, _>(column)
        .map(|value| value != 0)
        .or_else(|_| row.try_get::<i64, _>(column).map(|value| value != 0))
        .or_else(|_| row.try_get::<bool, _>(column))
        .unwrap_or(false)
}

fn append_entity(diagram: &mut String, table: &str, columns: &[SchemaRow]) {
    let table = mermaid_identifier(table);
    diagram.push_str("    ");
    diagram.push_str(&table);
    diagram.push_str(" {\n");
    for row in columns {
        let name = row.try_get::<String, _>("name").map_or_else(
            |_| "unnamed".to_string(),
            |value| mermaid_identifier(&value),
        );
        let kind = row.try_get::<String, _>("kind").map_or_else(
            |_| "unknown".to_string(),
            |value| mermaid_identifier(&value),
        );
        let primary_key = if row_is_primary_key(row, "pk") {
            " PK"
        } else {
            ""
        };
        diagram.push_str("        ");
        diagram.push_str(&kind);
        diagram.push(' ');
        diagram.push_str(&name);
        diagram.push_str(primary_key);
        diagram.push('\n');
    }
    diagram.push_str("    }\n");
}

fn append_relation(diagram: &mut String, from_table: &str, row: &SchemaRow) {
    let to_table = row.try_get::<String, _>("to_table").map_or_else(
        |_| "unnamed".to_string(),
        |value| mermaid_identifier(&value),
    );
    let from_column = row.try_get::<String, _>("from_column").map_or_else(
        |_| "unnamed".to_string(),
        |value| mermaid_identifier(&value),
    );
    let to_column = row.try_get::<String, _>("to_column").map_or_else(
        |_| "unnamed".to_string(),
        |value| mermaid_identifier(&value),
    );
    let from_table = mermaid_identifier(from_table);
    diagram.push_str("    ");
    diagram.push_str(&from_table);
    diagram.push_str(" }|--|| ");
    diagram.push_str(&to_table);
    diagram.push_str(" : \"");
    diagram.push_str(&from_column);
    diagram.push_str("_to_");
    diagram.push_str(&to_column);
    diagram.push_str("\"\n");
}

fn configured_pool() -> Result<&'static rullst_core::db::RullstPool, sqlx::Error> {
    rullst_core::db::safe_pool()
        .ok_or_else(|| sqlx::Error::Configuration("Studio database pool is not configured".into()))
}

async fn get_sqlite_schema() -> Result<String, sqlx::Error> {
    let pool = configured_pool()?;
    let tables = rullst_orm::_sqlx::query(
        "SELECT name FROM sqlite_schema WHERE type = 'table' \
         AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations' ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    let mut diagram = String::from("erDiagram\n");

    for table_row in tables {
        let table = table_row.try_get::<String, _>("name")?;
        let mut columns_query = QueryBuilder::<rullst_orm::RullstDatabase>::new(
            "SELECT name, type AS kind, pk FROM pragma_table_info(",
        );
        columns_query.push_bind(&table).push(")");
        let columns = columns_query.build().fetch_all(pool).await?;
        append_entity(&mut diagram, &table, &columns);

        let mut relations_query = QueryBuilder::<rullst_orm::RullstDatabase>::new(
            "SELECT `table` AS to_table, `from` AS from_column, `to` AS to_column \
             FROM pragma_foreign_key_list(",
        );
        relations_query.push_bind(&table).push(")");
        for relation in relations_query.build().fetch_all(pool).await? {
            append_relation(&mut diagram, &table, &relation);
        }
    }
    Ok(diagram)
}

async fn get_postgres_schema() -> Result<String, sqlx::Error> {
    let pool = configured_pool()?;
    let tables = rullst_orm::_sqlx::query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name != '_sqlx_migrations' \
         ORDER BY table_name",
    )
    .fetch_all(pool)
    .await?;
    let mut diagram = String::from("erDiagram\n");

    for table_row in tables {
        let table = table_row.try_get::<String, _>("table_name")?;
        let mut columns_query = QueryBuilder::<rullst_orm::RullstDatabase>::new(
            "SELECT c.column_name AS name, c.data_type AS kind, \
             CASE WHEN tc.constraint_type = 'PRIMARY KEY' THEN 1 ELSE 0 END AS pk \
             FROM information_schema.columns c \
             LEFT JOIN information_schema.key_column_usage kcu \
               ON c.table_schema = kcu.table_schema AND c.table_name = kcu.table_name \
              AND c.column_name = kcu.column_name \
             LEFT JOIN information_schema.table_constraints tc \
               ON kcu.constraint_schema = tc.constraint_schema \
              AND kcu.constraint_name = tc.constraint_name \
              AND tc.constraint_type = 'PRIMARY KEY' \
             WHERE c.table_schema = 'public' AND c.table_name = ",
        );
        columns_query
            .push_bind(&table)
            .push(" ORDER BY c.ordinal_position");
        let columns = columns_query.build().fetch_all(pool).await?;
        append_entity(&mut diagram, &table, &columns);

        let mut relations_query = QueryBuilder::<rullst_orm::RullstDatabase>::new(
            "SELECT kcu.column_name AS from_column, ccu.table_name AS to_table, \
                    ccu.column_name AS to_column \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
               ON tc.constraint_name = kcu.constraint_name \
              AND tc.table_schema = kcu.table_schema \
             JOIN information_schema.constraint_column_usage ccu \
               ON ccu.constraint_name = tc.constraint_name \
              AND ccu.table_schema = tc.table_schema \
             WHERE tc.constraint_type = 'FOREIGN KEY' \
               AND tc.table_schema = 'public' AND tc.table_name = ",
        );
        relations_query.push_bind(&table);
        for relation in relations_query.build().fetch_all(pool).await? {
            append_relation(&mut diagram, &table, &relation);
        }
    }
    Ok(diagram)
}

async fn get_mysql_schema() -> Result<String, sqlx::Error> {
    let pool = configured_pool()?;
    let tables = rullst_orm::_sqlx::query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = DATABASE() AND table_name != '_sqlx_migrations' \
         ORDER BY table_name",
    )
    .fetch_all(pool)
    .await?;
    let mut diagram = String::from("erDiagram\n");

    for table_row in tables {
        let table = table_row.try_get::<String, _>("table_name")?;
        let mut columns_query = QueryBuilder::<rullst_orm::RullstDatabase>::new(
            "SELECT column_name AS name, data_type AS kind, \
                    CASE WHEN column_key = 'PRI' THEN 1 ELSE 0 END AS pk \
             FROM information_schema.columns WHERE table_schema = DATABASE() \
             AND table_name = ",
        );
        columns_query
            .push_bind(&table)
            .push(" ORDER BY ordinal_position");
        let columns = columns_query.build().fetch_all(pool).await?;
        append_entity(&mut diagram, &table, &columns);

        let mut relations_query = QueryBuilder::<rullst_orm::RullstDatabase>::new(
            "SELECT column_name AS from_column, referenced_table_name AS to_table, \
                    referenced_column_name AS to_column \
             FROM information_schema.key_column_usage \
             WHERE table_schema = DATABASE() AND referenced_table_name IS NOT NULL \
             AND table_name = ",
        );
        relations_query.push_bind(&table);
        for relation in relations_query.build().fetch_all(pool).await? {
            append_relation(&mut diagram, &table, &relation);
        }
    }
    Ok(diagram)
}

async fn schema_for_driver(driver: &str) -> Result<String, sqlx::Error> {
    match driver {
        "postgres" => get_postgres_schema().await,
        "mysql" | "mariadb" => get_mysql_schema().await,
        "sqlite" | "libsql" | "turso" => get_sqlite_schema().await,
        _ => Err(sqlx::Error::Configuration(
            format!("ER diagram does not support database driver `{driver}`").into(),
        )),
    }
}

async fn render_er_diagram() -> Html<String> {
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let (diagram, notice) = match schema_for_driver(driver).await {
        Ok(diagram) => (diagram, String::new()),
        Err(error) => (
            String::from("erDiagram\n"),
            format!(
                "<p class=\"mb-4 text-amber-300\">Schema unavailable: {}</p>",
                rullst_core::html::escape_str(&error.to_string())
            ),
        ),
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
        mermaid.initialize({{ startOnLoad: true, theme: 'dark', securityLevel: 'strict' }});
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
        {notice}
        <div class="bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-2xl flex-1 p-8 flex items-center justify-center">
            <pre class="mermaid">{}</pre>
        </div>
    </div>
</body>
</html>"#,
        rullst_core::html::escape_str(&diagram)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn er_diagram_endpoint_exposes_real_or_unavailable_state() {
        let response = router()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let html = render_er_diagram().await.0;
        assert!(html.contains("Visual ER Diagram"));
        assert!(html.contains("class=\"mermaid\""));
        assert!(html.contains("erDiagram"));
        assert!(html.contains("securityLevel: 'strict'"));
    }

    #[test]
    fn mermaid_identifiers_are_bounded_and_cannot_inject_directives() {
        let identifier = mermaid_identifier("9 users<script>%%{init:evil}");
        assert_eq!(identifier, "entity_9_users_script____init_evil_");
        assert!(identifier.len() <= 71);
        assert!(!identifier.contains('<'));
        assert!(!identifier.contains('%'));
        assert!(!identifier.contains('{'));
    }
}
