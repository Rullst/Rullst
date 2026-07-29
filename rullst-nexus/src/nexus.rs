//! # Rullst Nexus Panel
//!
//! Auto-Generated CMS & AI-Powered Admin Panel for Rullst applications.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use rullst::nexus::{Nexus, NexusModel, FieldMeta, FieldKind};
//!
//! // 1. Implement NexusModel for your struct
//! struct User;
//! impl NexusModel for User {
//!     fn nexus_table() -> &'static str { "users" }
//!     fn nexus_label() -> &'static str { "Users" }
//!     fn nexus_fields() -> Vec<FieldMeta> {
//!         vec![
//!             FieldMeta { name: "id", label: "ID", kind: FieldKind::Number, hidden: true, readonly: true },
//!             FieldMeta { name: "name", label: "Name", kind: FieldKind::Text, hidden: false, readonly: false },
//!             FieldMeta { name: "email", label: "Email", kind: FieldKind::Email, hidden: false, readonly: false },
//!         ]
//!     }
//! }
//!
//! // 2. Register your models and mount the panel
//! async fn setup() {
//!     let nexus = Nexus::new()
//!         .register::<User>()
//!         .with_brand("My App");
//!     let _router = nexus.build();
//! }
//! ```

use axum::{
    Router as AxumRouter,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::Arc;

fn sanitize_identifier(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(64) // Strict length limit for security
        .collect()
}

// â”€â”€â”€ Field Metadata & Reflection â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// The semantic type of a model field, used to render the correct HTML input.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    /// Single-line plain text.
    Text,
    /// Long-form textarea content.
    Textarea,
    /// A valid email address input.
    Email,
    /// A URL link.
    Url,
    /// Numeric integer or float value.
    Number,
    /// A boolean checkbox.
    Boolean,
    /// Date picker (YYYY-MM-DD).
    Date,
    /// Date + time picker (YYYY-MM-DDTHH:MM).
    DateTime,
    /// A password field that hides its value.
    Password,
    /// A JSON object displayed as textarea.
    Json,
    /// A foreign key pointing to another table.
    ForeignKey {
        /// Target table name (e.g. "categories").
        table: &'static str,
        /// Column of target table to use as option label (e.g. "name").
        label_col: &'static str,
    },
    /// A dropdown menu for Enum values.
    Enum { options: Vec<&'static str> },
}

/// Describes a single field/column in a model's schema for the Nexus Panel.
#[derive(Debug, Clone)]
pub struct FieldMeta {
    /// Database/struct column name (e.g. "created_at").
    pub name: &'static str,
    /// Human-readable label shown in the UI (e.g. "Created At").
    pub label: &'static str,
    /// Semantic type that determines which input widget to render.
    pub kind: FieldKind,
    /// If true, hides this field from list/table views (still visible on edit forms).
    pub hidden: bool,
    /// If true, the field is displayed but cannot be modified via the edit form.
    pub readonly: bool,
}

/// The core reflection trait that unlocks Nexus Panel integration for any model.
///
/// Implement this trait to register your model with the Nexus Panel.
/// The derive macro `#[derive(Nexus)]` will auto-generate this implementation in a future release.
pub trait NexusModel: Send + Sync + 'static {
    /// The database table name (e.g. "users").
    fn nexus_table() -> &'static str;
    /// A human-readable plural label for the collection (e.g. "Users").
    fn nexus_label() -> &'static str;
    /// A short icon/emoji representing the model in the sidebar (e.g. "ðŸ‘¤").
    fn nexus_icon() -> &'static str {
        "ðŸ“‹"
    }
    /// A list of FieldMeta describing each column in this model's schema.
    fn nexus_fields() -> Vec<FieldMeta>;
    /// The name of the primary key column (defaults to "id").
    fn nexus_pk() -> &'static str {
        "id"
    }
}

// â”€â”€â”€ Registry â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Internal representation of a registered model used by the Nexus Panel engine.
#[derive(Clone)]
struct RegistryEntry {
    pub table: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub pk: &'static str,
    pub fields: Vec<FieldMeta>,
}

/// Shared state passed into all Nexus route handlers.
#[derive(Clone)]
struct NexusState {
    pub registry: Arc<Vec<RegistryEntry>>,
    pub brand: Arc<String>,
}

// â”€â”€â”€ Nexus Builder â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// The main entry point for configuring and mounting the Rullst Nexus Panel.
///
/// # Example
/// ```rust,no_run
/// # use rullst::nexus::Nexus;
/// let nexus_router = Nexus::new()
///     .with_brand("My SaaS")
///     .with_auth("admin", "secret_pass")
///     .build();
/// ```
pub struct Nexus {
    registry: Vec<RegistryEntry>,
    brand: String,
    auth: Option<(String, String)>,
}

impl Default for Nexus {
    fn default() -> Self {
        Self::new()
    }
}

impl Nexus {
    /// Creates a new Nexus builder with default settings.
    pub fn new() -> Self {
        Nexus {
            registry: Vec::new(),
            brand: "Rullst Nexus".to_string(),
            auth: None,
        }
    }

    /// Registers a model to be managed by the Nexus Panel.
    pub fn register<M: NexusModel>(mut self) -> Self {
        self.registry.push(RegistryEntry {
            table: M::nexus_table(),
            label: M::nexus_label(),
            icon: M::nexus_icon(),
            pk: M::nexus_pk(),
            fields: M::nexus_fields(),
        });
        self
    }

    /// Sets the brand/app name displayed in the Nexus Panel header.
    pub fn with_brand(mut self, brand: impl Into<String>) -> Self {
        self.brand = brand.into();
        self
    }

    /// The Nexus panel now extracts the database pool directly from the application's extensions.
    /// This method is maintained solely for backward compatibility and is a no-op.
    pub fn with_db(self, _db_url: &str) -> Self {
        self
    }

    /// Exposes optional HTTP Basic Authentication credentials to secure the panel.
    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth = Some((username.into(), password.into()));
        self
    }

    /// Builds and returns an Axum Router for the Nexus Panel.
    /// Mount it with `.nest("/nexus", nexus.build())` on your app's router.
    pub fn build(self) -> AxumRouter {
        let state = Arc::new(NexusState {
            registry: Arc::new(self.registry),
            brand: Arc::new(self.brand),
        });

        let router = AxumRouter::new()
            .route("/", get(nexus_dashboard))
            .route("/table/{table}", get(nexus_table_view))
            .route("/table/{table}/search", get(nexus_table_search))
            .route("/table/{table}/new", get(nexus_new_form))
            .route("/table/{table}", post(nexus_create_record))
            .route("/table/{table}/{id}/edit", get(nexus_edit_form))
            .route("/table/{table}/{id}", put(nexus_update_record))
            .route("/table/{table}/{id}", delete(nexus_delete_record))
            .route("/table/{table}/batch", post(nexus_batch_action))
            .route("/chat", get(nexus_chat_page))
            .route("/chat/query", post(nexus_chat_query))
            .layer(axum::middleware::from_fn(
                rullst_core::security::csrf_middleware,
            ));

        let router = if let Some((username, password)) = self.auth {
            router.layer(axum::middleware::from_fn(
                move |req: axum::extract::Request, next: axum::middleware::Next| {
                    let expected_username = username.clone();
                    let expected_password = password.clone();
                    async move {
                        if let Some(auth_header) =
                            req.headers().get(axum::http::header::AUTHORIZATION)
                            && let Ok(auth_str) = auth_header.to_str()
                            && let Some(encoded) = auth_str.strip_prefix("Basic ")
                        {
                            use base64::Engine;
                            if let Ok(decoded) =
                                base64::engine::general_purpose::STANDARD.decode(encoded)
                                && let Ok(decoded_str) = String::from_utf8(decoded)
                                && let Some((parts_user, parts_pass)) = decoded_str.split_once(':')
                            {
                                use subtle::ConstantTimeEq;
                                if parts_user == expected_username
                                    && parts_pass.len() == expected_password.len()
                                    && parts_pass
                                        .as_bytes()
                                        .ct_eq(expected_password.as_bytes())
                                        .into()
                                {
                                    return next.run(req).await;
                                }
                            }
                        }
                        axum::response::Response::builder()
                            .status(axum::http::StatusCode::UNAUTHORIZED)
                            .header(
                                axum::http::header::WWW_AUTHENTICATE,
                                "Basic realm=\"Nexus Admin Panel\"",
                            )
                            .body(axum::body::Body::empty())
                            .unwrap_or_else(|_| {
                                let mut res =
                                    axum::response::Response::new(axum::body::Body::empty());
                                *res.status_mut() = axum::http::StatusCode::UNAUTHORIZED;
                                res
                            })
                    }
                },
            ))
        } else {
            eprintln!(
                "âš ï¸  Nexus Warning: Nexus admin panel has NO authentication configured. Use `.with_auth(username, password)` to protect it in production."
            );
            router
        };

        router.with_state(state)
    }
}

// â”€â”€â”€ Query Params â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Deserialize)]
struct PaginationParams {
    page: Option<u32>,
    q: Option<String>,
    sort_by: Option<String>,
    order: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct ChatRequest {
    message: String,
}

// â”€â”€â”€ Route Handlers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// GET /nexus â€” Dashboard overview.
async fn nexus_dashboard(
    State(state): State<Arc<NexusState>>,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let models_sidebar = render_sidebar(&state, None);

    let stats_cards = state.registry.iter().fold(
        String::with_capacity(state.registry.len() * 256),
        |mut acc, m| {
            let t = m.table;
            let ic = m.icon;
            let lb = m.label;
            let _ = write!(
                acc,
                "<a href=\"/nexus/table/{t}\" class=\"nexus-stat-card\" \
                 hx-get=\"/nexus/table/{t}\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
                 <div class=\"nexus-stat-icon\">{ic}</div>\
                 <div class=\"nexus-stat-label\">{lb}</div>\
                 <div class=\"nexus-stat-hint\">Click to manage &rarr;</div>\
                 </a>"
            );
            acc
        },
    );

    let mut content = String::new();
    content.push_str("<div class=\"nexus-page-header\">");
    content.push_str("<h1 class=\"nexus-page-title\">&#127963;&#65039; Dashboard</h1>");
    content.push_str("<p class=\"nexus-page-subtitle\">Welcome to the Rullst Nexus Panel. Select a model to begin.</p>");
    content.push_str("</div>");
    content.push_str("<div class=\"nexus-stat-grid\">");
    content.push_str(&stats_cards);
    content.push_str("</div>");
    content.push_str("<div class=\"nexus-welcome-box\">");
    content.push_str("<div class=\"nexus-welcome-icon\">&#9889;</div>");
    content.push_str("<h2>Auto-Generated CMS</h2>");
    content.push_str("<p>Every model you register appears here with full CRUD, search, and pagination &mdash; zero configuration required.</p>");
    content.push_str("<a href=\"/nexus/chat\" class=\"nexus-btn nexus-btn-ai\" hx-get=\"/nexus/chat\" hx-target=\"#nexus-content\" hx-push-url=\"true\">&#129302; Open AI Query Assistant</a>");
    content.push_str("</div>");

    if headers.contains_key("hx-request") {
        Html(content)
    } else {
        Html(render_shell(&state, &models_sidebar, &content))
    }
}

/// GET /nexus/table/{table} â€” Model list view with pagination.
async fn nexus_table_view(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    Query(params): Query<PaginationParams>,
    headers: axum::http::HeaderMap,
) -> Response {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Html("<p>Table not found.</p>".to_string()),
            )
                .into_response();
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let q = params.q.clone().unwrap_or_default();
    let sort_by = params.sort_by.as_deref();
    let order = params.order.as_deref();

    let content = render_table_view(&state, entry, page, &q, sort_by, order).await;
    if headers.contains_key("hx-request") {
        Html(content).into_response()
    } else {
        Html(render_shell(
            &state,
            &render_sidebar(&state, Some(&table)),
            &content,
        ))
        .into_response()
    }
}

/// GET /nexus/table/{table}/search â€” HTMX search fragment (no shell).
async fn nexus_table_search(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    let q = params.q.clone().unwrap_or_default();
    let page = params.page.unwrap_or(1).max(1);
    let sort_by = params.sort_by.as_deref();
    let order = params.order.as_deref();
    Html(render_table_rows(entry, &q, page, sort_by, order).await)
}

/// GET /nexus/table/{table}/new â€” New record form.
async fn nexus_new_form(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    Html(render_record_form(&state, entry, None).await)
}

/// POST /nexus/table/{table} â€” Create a new record.
#[cfg_attr(mutants, mutants::skip)]
async fn nexus_create_record(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    axum::extract::Form(data): axum::extract::Form<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
            )
                .into_response();
        }
    };

    let mut keys = Vec::new();
    let mut values = Vec::new();
    for f in &entry.fields {
        if let Some(val) = data.get(f.name) {
            if f.name == entry.pk && val.trim().is_empty() {
                continue;
            }
            keys.push(f.name);
            values.push(val);
        }
    }

    if keys.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-danger\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#10060; No values provided to create {}\
                 </div>",
                entry.label
            ))
        ).into_response();
    }

    let clean_table = sanitize_identifier(&table);
    let clean_keys: Vec<String> = keys.iter().map(|k| sanitize_identifier(k)).collect();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        clean_table,
        clean_keys.join(", "),
        (0..clean_keys.len())
            .map(|i| format!("${}", i + 1))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    for v in values {
        query = query.bind(v);
    }

    let mut success = false;
    let mut err_msg = String::new();

    if let Some(pool) = rullst_core::db::safe_pool() {
        match query.execute(pool).await {
            Ok(_) => {
                success = true;
            }
            Err(e) => {
                err_msg = e.to_string();
            }
        }
    } else {
        err_msg = "Database pool not initialized".to_string();
    }

    if success {
        (
            axum::http::StatusCode::OK,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-success\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#9989; New {} record created successfully!\
                 </div>",
                entry.label
            ))
        ).into_response()
    } else {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-danger\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#10060; Failed to create {}: {}\
                 </div>",
                entry.label,
                rullst_core::html::escape_str(&err_msg)
            ))
        ).into_response()
    }
}

/// GET /nexus/table/{table}/{id}/edit â€” Edit record form.
async fn nexus_edit_form(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    Html(render_record_form(&state, entry, Some(&id)).await)
}

/// PUT /nexus/table/{table}/{id} â€” Update a record.
async fn nexus_update_record(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
    axum::extract::Form(data): axum::extract::Form<std::collections::HashMap<String, String>>,
) -> impl axum::response::IntoResponse {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
            )
                .into_response();
        }
    };

    let clean_table = sanitize_identifier(&table);
    let clean_pk = sanitize_identifier(entry.pk);
    let mut updates = Vec::new();
    let mut values = Vec::new();
    for f in &entry.fields {
        if f.name != entry.pk
            && let Some(val) = data.get(f.name)
        {
            let clean_field = sanitize_identifier(f.name);
            updates.push(format!("{} = ${}", clean_field, updates.len() + 1));
            values.push(val);
        }
    }

    let sql = format!(
        "UPDATE {} SET {} WHERE {} = ${}",
        clean_table,
        updates.join(", "),
        clean_pk,
        updates.len() + 1
    );
    let mut query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
    for v in values {
        query = query.bind(v);
    }
    query = query.bind(id.clone());

    let mut success = false;
    let mut err_msg = String::new();

    if let Some(pool) = rullst_core::db::safe_pool() {
        match query.execute(pool).await {
            Ok(_) => {
                success = true;
            }
            Err(e) => {
                err_msg = e.to_string();
            }
        }
    } else {
        err_msg = "Database pool not initialized".to_string();
    }

    if success {
        (
            axum::http::StatusCode::OK,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-success\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#9989; {} #{} updated successfully!\
                 </div>",
                entry.label,
                rullst_core::html::escape_str(&id)
            ))
        ).into_response()
    } else {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-danger\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#10060; Failed to update {}: {}\
                 </div>",
                entry.label,
                rullst_core::html::escape_str(&err_msg)
            ))
        ).into_response()
    }
}

/// DELETE /nexus/table/{table}/{id} â€” Delete a record.
async fn nexus_delete_record(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
) -> impl axum::response::IntoResponse {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
            )
                .into_response();
        }
    };

    let clean_table = sanitize_identifier(&table);
    let clean_pk = sanitize_identifier(entry.pk);
    let sql = format!("DELETE FROM {} WHERE {} = ?", clean_table, clean_pk);
    let mut success = false;
    let mut err_msg = String::new();

    if let Some(pool) = rullst_core::db::safe_pool() {
        match rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()))
            .bind(&id)
            .execute(pool)
            .await
        {
            Ok(_) => {
                success = true;
            }
            Err(e) => {
                err_msg = e.to_string();
            }
        }
    } else {
        err_msg = "Database pool not initialized".to_string();
    }

    if success {
        (
            axum::http::StatusCode::OK,
            Html(format!(
                "<tr id=\"row-{id}\" class=\"nexus-row-deleted\">\
                 <td colspan=\"99\">\
                 <div class=\"nexus-toast nexus-toast-warning\">\
                 &#128465;&#65039; {} #{} deleted.\
                 </div></td></tr>",
                entry.label,
                rullst_core::html::escape_str(&id)
            )),
        )
            .into_response()
    } else {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<div class=\"nexus-toast nexus-toast-danger\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
                 &#10060; Failed to delete {} #{}: {}\
                 </div>",
                entry.label,
                rullst_core::html::escape_str(&id),
                rullst_core::html::escape_str(&err_msg)
            ))
        ).into_response()
    }
}

/// GET /nexus/chat â€” AI Query Assistant page.
async fn nexus_chat_page(
    State(state): State<Arc<NexusState>>,
    headers: axum::http::HeaderMap,
) -> Html<String> {
    let schema_summary: String = state
        .registry
        .iter()
        .map(|m| {
            let cols: Vec<String> = m
                .fields
                .iter()
                .map(|f| format!("{} ({})", f.name, field_kind_label(&f.kind)))
                .collect();
            format!("* {} ({}): {}", m.label, m.table, cols.join(", "))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut content = String::new();
    content.push_str("<div class=\"nexus-page-header\">");
    content.push_str("<h1 class=\"nexus-page-title\">&#129302; AI Query Assistant</h1>");
    content.push_str("<p class=\"nexus-page-subtitle\">Ask questions about your data in plain language. Powered by <strong>rullst::ai</strong>.</p>");
    content.push_str("</div>");
    content.push_str("<div class=\"nexus-chat-layout\">");
    content.push_str("<div class=\"nexus-chat-schema\">");
    content.push_str("<div class=\"nexus-schema-title\">&#128202; Database Schema</div>");
    content.push_str("<pre class=\"nexus-schema-pre\">");
    content.push_str(&rullst_core::html::escape_str(&schema_summary));
    content.push_str("</pre></div>");
    content.push_str("<div class=\"nexus-chat-panel\">");
    content.push_str("<div class=\"nexus-chat-messages\" id=\"nexus-chat-messages\">");
    content.push_str("<div class=\"nexus-chat-bubble nexus-chat-assistant\">");
    content.push_str("<span class=\"nexus-chat-avatar\">&#129302;</span>");
    content.push_str("<div class=\"nexus-chat-text\">Hello! I know your full database schema. Ask me anything &mdash; for example:<br><em>\"List all users created this week\"</em> or <em>\"How many posts are published?\"</em><br><br><small style=\"color: var(--text-300);\">&#128161; <b>Tip:</b> To execute real natural-language queries, you can inject an AI provider into your Nexus instance:<br><code class=\"nexus-code\" style=\"margin-top: 8px; display: block;\">.with_ai(AiClient::new(AiProvider::Gemini { api_key: env!(\"GEMINI_KEY\") }))<br>// Or use AiProvider::OpenAI, AiProvider::Anthropic, etc.</code></small></div>");
    content.push_str("</div></div>");
    content.push_str("<form class=\"nexus-chat-form\" hx-post=\"/nexus/chat/query\" hx-target=\"#nexus-chat-messages\" hx-swap=\"beforeend\" hx-on::after-request=\"this.reset(); document.getElementById(&quot;nexus-chat-messages&quot;).scrollTop = 99999;\">");
    content.push_str("<input type=\"text\" name=\"message\" class=\"nexus-chat-input\" placeholder=\"Ask about your data...\" aria-label=\"Ask the AI assistant\" autocomplete=\"off\" required />");
    content.push_str(
        "<button type=\"submit\" class=\"nexus-btn nexus-btn-ai\">Send &#9992;&#65039;</button>",
    );
    content.push_str("</form></div></div>");

    if headers.contains_key("hx-request") {
        Html(content)
    } else {
        Html(render_shell(
            &state,
            &render_sidebar(&state, None),
            &content,
        ))
    }
}

/// POST /nexus/chat/query â€” AI Query HTMX endpoint.
#[cfg_attr(mutants, mutants::skip)]
async fn nexus_chat_query(
    State(state): State<Arc<NexusState>>,
    axum::extract::Form(req): axum::extract::Form<ChatRequest>,
) -> Html<String> {
    let user_msg = rullst_core::html::escape_str(&req.message);

    let schema: String = state
        .registry
        .iter()
        .map(|m| {
            let cols: Vec<String> = m
                .fields
                .iter()
                .map(|f| format!("{} {}", f.name, field_kind_sql(&f.kind)))
                .collect();
            format!("CREATE TABLE {} ({});", m.table, cols.join(", "))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let ai_response = generate_mock_ai_response(&req.message, &schema);

    Html(format!(
        "<div class=\"nexus-chat-bubble nexus-chat-user\">\
         <span class=\"nexus-chat-avatar\">&#128100;</span>\
         <div class=\"nexus-chat-text\">{user_msg}</div>\
         </div>\
         <div class=\"nexus-chat-bubble nexus-chat-assistant\">\
         <span class=\"nexus-chat-avatar\">&#129302;</span>\
         <div class=\"nexus-chat-text\">{ai_response}</div>\
         </div>"
    ))
}

// â”€â”€â”€ Rendering Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn find_entry<'a>(state: &'a NexusState, table: &str) -> Option<&'a RegistryEntry> {
    state.registry.iter().find(|e| e.table == table)
}

fn field_kind_label(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Text => "text",
        FieldKind::Textarea => "textarea",
        FieldKind::Email => "email",
        FieldKind::Url => "url",
        FieldKind::Number => "number",
        FieldKind::Boolean => "boolean",
        FieldKind::Date => "date",
        FieldKind::DateTime => "datetime",
        FieldKind::Password => "password",
        FieldKind::Json => "json",
        FieldKind::ForeignKey { .. } => "relation",
        FieldKind::Enum { .. } => "enum",
    }
}

#[cfg_attr(mutants, mutants::skip)]
fn field_kind_sql(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Number => "INTEGER",
        FieldKind::Boolean => "INTEGER",
        FieldKind::ForeignKey { .. } => "INTEGER",
        FieldKind::Date | FieldKind::DateTime => "TEXT",
        FieldKind::Json => "TEXT",
        FieldKind::Enum { .. } => "TEXT",
        _ => "TEXT",
    }
}

#[cfg(all(test, not(miri)))]
fn field_kind_input_type(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Email => "email",
        FieldKind::Url => "url",
        FieldKind::Number => "number",
        FieldKind::Password => "password",
        FieldKind::Date => "date",
        FieldKind::DateTime => "datetime-local",
        FieldKind::ForeignKey { .. } => "select",
        FieldKind::Enum { .. } => "select",
        _ => "text",
    }
}

#[cfg_attr(mutants, mutants::skip)]
fn generate_mock_ai_response(message: &str, schema: &str) -> String {
    let msg_lower = message.to_lowercase();
    if msg_lower.contains("select")
        || msg_lower.contains("list")
        || msg_lower.contains("show")
        || msg_lower.contains("quais")
        || msg_lower.contains("mostrar")
    {
        "<p>Based on your schema, here&#39;s a suggested query:</p>\
         <code class=\"nexus-code\">SELECT * FROM your_table ORDER BY id DESC LIMIT 20;</code>\
         <small>&#128161; Connect an AI provider via <code>rullst::ai::AiClient</code> to execute real natural-language queries.</small>"
            .to_string()
    } else if msg_lower.contains("count")
        || msg_lower.contains("how many")
        || msg_lower.contains("quantos")
    {
        format!(
            "<p>Here&#39;s a count query:</p>\
             <code class=\"nexus-code\">SELECT COUNT(*) as total FROM your_table;</code>\
             <small>&#128161; Your schema has {} table(s) registered with the Nexus Panel.</small>",
            schema.lines().count()
        )
    } else {
        format!(
            "<p>I understand you&#39;re asking: <em>{}</em></p>\
             <p>To enable real AI-powered SQL generation, configure an AI provider:</p>\
             <code class=\"nexus-code\">AiClient::new(AiProvider::Gemini {{ api_key: env!(\"GEMINI_KEY\") }})</code>",
            rullst_core::html::escape_str(message)
        )
    }
}

fn render_sidebar(state: &NexusState, active_table: Option<&str>) -> String {
    let mut out = String::new();
    for m in state.registry.iter() {
        let is_active = active_table == Some(m.table);
        let active_class = if is_active { " nexus-nav-active" } else { "" };
        let t = m.table;
        let lb = m.label;
        let ic = m.icon;
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                "<a href=\"/nexus/table/{t}\" class=\"nexus-nav-link{active_class}\" \
             hx-get=\"/nexus/table/{t}\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
             <span class=\"nexus-nav-icon\">{ic}</span><span>{lb}</span></a>"
            ),
        );
    }
    out.push_str("<div class=\"nexus-nav-divider\"></div>");
    out.push_str(
        "<a href=\"/nexus/chat\" class=\"nexus-nav-link nexus-nav-ai\" \
         hx-get=\"/nexus/chat\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
         <span class=\"nexus-nav-icon\">&#129302;</span><span>AI Assistant</span></a>",
    );
    out
}

fn build_table_query(
    entry: &RegistryEntry,
    visible_fields: &[&FieldMeta],
    q: &str,
    page: u32,
    sort_by: Option<&str>,
    order: Option<&str>,
) -> (String, Vec<String>) {
    let clean_table = sanitize_identifier(entry.table);
    let _clean_pk = sanitize_identifier(entry.pk);
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");

    let mut sql = format!("SELECT * FROM {}", clean_table);
    let mut binds = Vec::new();

    if !q.is_empty() {
        let mut clauses = Vec::new();
        for f in visible_fields {
            if matches!(
                f.kind,
                FieldKind::Text | FieldKind::Email | FieldKind::Url | FieldKind::Textarea
            ) {
                let clean_field = sanitize_identifier(f.name);
                if driver == "postgres" {
                    clauses.push(format!("{} ILIKE ${}", clean_field, clauses.len() + 1));
                } else {
                    clauses.push(format!("{} LIKE ?", clean_field));
                }
            }
        }
        if !clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" OR "));
            for _ in &clauses {
                binds.push(format!("%{}%", q));
            }
        }
    }

    let limit = 20;
    let offset = (page.max(1) - 1) * limit;

    let sort_col = sort_by
        .filter(|&c| visible_fields.iter().any(|f| f.name == c))
        .unwrap_or(entry.pk);
    let sort_dir = order
        .filter(|&o| o.eq_ignore_ascii_case("asc") || o.eq_ignore_ascii_case("desc"))
        .unwrap_or("DESC");
    let clean_sort_col = sanitize_identifier(sort_col);

    let _ = std::fmt::Write::write_fmt(
        &mut sql,
        format_args!(
            " ORDER BY {} {} LIMIT {} OFFSET {}",
            clean_sort_col, sort_dir, limit, offset
        ),
    );

    (sql, binds)
}

fn render_empty_state_html(cols: usize, table: &str, q: &str) -> String {
    if q.is_empty() {
        format!(
            "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">No records found in table `{}`.</td></tr>",
            cols, table
        )
    } else {
        format!(
            "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">&#128269; No results matching \"{}\"</td></tr>",
            cols,
            rullst_core::html::escape_str(q)
        )
    }
}

#[cfg_attr(mutants, mutants::skip)]
async fn render_table_rows(
    entry: &RegistryEntry,
    q: &str,
    page: u32,
    sort_by: Option<&str>,
    order: Option<&str>,
) -> String {
    let visible_fields: Vec<&FieldMeta> = entry.fields.iter().filter(|f| !f.hidden).collect();
    let (sql, binds) = build_table_query(entry, &visible_fields, q, page, sort_by, order);

    let pool = match rullst_core::db::safe_pool() {
        Some(p) => p,
        None => {
            return format!(
                "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">&#10071; Database not initialized. Please configure database_url.</td></tr>",
                visible_fields.len() + 1
            );
        }
    };

    let sql_safe = rullst_orm::_sqlx::AssertSqlSafe(sql.as_str());
    let mut query = rullst_orm::_sqlx::query(sql_safe);
    for bind in binds {
        query = query.bind(bind);
    }

    use rullst_orm::_sqlx::Row;
    let rows_result = query.fetch_all(pool).await;

    let db_rows = match rows_result {
        Ok(r) => r,
        Err(e) => {
            return format!(
                "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">&#10071; Database Error: {}</td></tr>",
                visible_fields.len() + 1,
                rullst_core::html::escape_str(&e.to_string())
            );
        }
    };

    if db_rows.is_empty() {
        return render_empty_state_html(visible_fields.len() + 1, entry.table, q);
    }

    let t = entry.table;
    let pk = entry.pk;

    db_rows.into_iter().fold(
        String::with_capacity(2048),
        |mut out, row| {
            let row_id: String = match row.try_get::<String, _>(pk) {
                Ok(s) => s,
                Err(_) => match row.try_get::<i64, _>(pk) {
                    Ok(i) => i.to_string(),
                    Err(_) => match row.try_get::<i32, _>(pk) {
                        Ok(i) => i.to_string(),
                        Err(_) => "0".to_string(),
                    },
                },
            };

            let cells = visible_fields.iter().fold(String::with_capacity(256), |mut cells, f| {
                let val_str = match f.kind {
                    FieldKind::Boolean => {
                        let b = row.try_get::<bool, _>(f.name).unwrap_or(false);
                        if b {
                            "&#9989; Yes".to_string()
                        } else {
                            "&#10060; No".to_string()
                        }
                    }
                    FieldKind::Number | FieldKind::ForeignKey { .. } => {
                        if let Ok(v) = row.try_get::<i64, _>(f.name) {
                            v.to_string()
                        } else if let Ok(v) = row.try_get::<f64, _>(f.name) {
                            v.to_string()
                        } else if let Ok(v) = row.try_get::<i32, _>(f.name) {
                            v.to_string()
                        } else {
                            "0".to_string()
                        }
                    }
                    _ => row
                        .try_get::<String, _>(f.name)
                        .unwrap_or_else(|_| "-".to_string()),
                };

                let clean_val = if val_str.starts_with("&#") {
                    val_str
                } else {
                    rullst_core::html::escape_str(&val_str).to_string()
                };

                let _ = std::fmt::Write::write_fmt(&mut cells, format_args!("<td class=\"nexus-td\">{}</td>", clean_val));
                cells
            });

            let checkbox_cell = format!("<td class=\"nexus-td text-center\"><input type=\"checkbox\" name=\"selected_ids\" value=\"{row_id}\" class=\"nexus-batch-check\" /></td>");

            let _ = std::fmt::Write::write_fmt(&mut out, format_args!(
                "<tr id=\"row-{row_id}\" class=\"nexus-tr\">\
                 {checkbox_cell}\
                 {cells}\
                 <td class=\"nexus-td nexus-td-actions\">\
                 <button type=\"button\" class=\"nexus-action-btn nexus-action-edit\" \
                 hx-get=\"/nexus/table/{t}/{row_id}/edit\" \
                 hx-target=\"#nexus-modal-body\" \
                 hx-on::after-request=\"document.getElementById(&quot;nexus-modal&quot;).showModal()\">&#9999;&#65039;</button>\
                 <button type=\"button\" class=\"nexus-action-btn nexus-action-delete\" \
                 hx-delete=\"/nexus/table/{t}/{row_id}\" \
                 hx-target=\"#row-{row_id}\" \
                 hx-confirm=\"Delete this record?\">&#128465;&#65039;</button>\
                 </td></tr>"
            ));
            out
        }
    )
}

#[cfg_attr(mutants, mutants::skip)]
async fn render_table_view(
    _state: &NexusState,
    entry: &RegistryEntry,
    page: u32,
    q: &str,
    sort_by: Option<&str>,
    order: Option<&str>,
) -> String {
    let visible_fields: Vec<&FieldMeta> = entry.fields.iter().filter(|f| !f.hidden).collect();
    let t = entry.table;
    let lb = entry.label;
    let ic = entry.icon;
    let lb_singular = entry.label.trim_end_matches('s');
    let q_esc = rullst_core::html::escape_str(q);

    let q_qs = if !q.is_empty() {
        format!("&q={}", q)
    } else {
        String::new()
    };

    let mut headers = String::with_capacity(512);
    headers.push_str("<th class=\"nexus-th text-center\" style=\"width: 40px;\"><input type=\"checkbox\" onchange=\"document.querySelectorAll('.nexus-batch-check').forEach(c => c.checked = this.checked)\" /></th>");
    let headers_cols = visible_fields
        .iter()
        .fold(String::with_capacity(256), |mut acc, f| {
            let next_order = if sort_by == Some(f.name) && order == Some("asc") { "desc" } else { "asc" };
            let sort_icon = if sort_by == Some(f.name) {
                if order == Some("asc") { " &#9650;" } else { " &#9660;" }
            } else {
                ""
            };
            let _ = std::fmt::Write::write_fmt(
                &mut acc,
                format_args!(
                    "<th class=\"nexus-th\"><a href=\"#\" hx-get=\"/nexus/table/{t}?page={page}{q_qs}&sort_by={}&order={next_order}\" hx-target=\"#nexus-content\" hx-push-url=\"true\" style=\"color:inherit;text-decoration:none;\">{}{}</a></th>",
                    f.name, f.label, sort_icon
                ),
            );
            acc
        });
    headers.push_str(&headers_cols);
    let rows = render_table_rows(entry, q, page, sort_by, order).await;

    let prev_btn = if page > 1 {
        let prev = page - 1;
        format!(
            "<a href=\"/nexus/table/{t}?page={prev}\" class=\"nexus-btn nexus-btn-ghost\" \
             hx-get=\"/nexus/table/{t}?page={prev}\" hx-target=\"#nexus-content\" hx-push-url=\"true\">&larr; Prev</a>"
        )
    } else {
        "<span></span>".to_string()
    };
    let next = page + 1;
    let next_btn = format!(
        "<a href=\"/nexus/table/{t}?page={next}\" class=\"nexus-btn nexus-btn-ghost\" \
         hx-get=\"/nexus/table/{t}?page={next}\" hx-target=\"#nexus-content\" hx-push-url=\"true\">Next &rarr;</a>"
    );

    let mut out = String::new();
    out.push_str("<div class=\"nexus-page-header\">");
    out.push_str("<div>");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!("<h1 class=\"nexus-page-title\">{ic} {lb}</h1>"),
    );
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            "<p class=\"nexus-page-subtitle\">Manage all records in the <code>{t}</code> table.</p>"
        ),
    );
    out.push_str("</div>");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            "<button class=\"nexus-btn nexus-btn-primary\" \
         hx-get=\"/nexus/table/{t}/new\" \
         hx-target=\"#nexus-modal-body\" \
         hx-on::after-request=\"document.getElementById(&quot;nexus-modal&quot;).showModal()\">\
         &#xFF0B; New {lb_singular}</button>"
        ),
    );
    out.push_str("</div>");

    out.push_str("<form method=\"POST\" action=\"/nexus/table/");
    out.push_str(t);
    out.push_str("/batch\" class=\"nexus-batch-form\">");

    out.push_str("<div class=\"nexus-toolbar\" style=\"display: flex; justify-content: space-between; align-items: center;\">");
    out.push_str("<div class=\"nexus-search-wrap\">");
    out.push_str("<span class=\"nexus-search-icon\">&#128269;</span>");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            "<input type=\"text\" class=\"nexus-search-input\" aria-label=\"Search records\" \
         placeholder=\"Search {lb}...\" value=\"{q_esc}\" \
         hx-get=\"/nexus/table/{t}/search\" \
         hx-trigger=\"keyup changed delay:300ms\" \
         hx-target=\"#nexus-table-body\" \
         name=\"q\" />"
        ),
    );
    out.push_str("</div>");

    out.push_str("<div style=\"display: flex; gap: 8px; align-items: center;\">");
    out.push_str(
        "<select name=\"action\" class=\"nexus-input\" style=\"width: auto; padding: 4px 8px;\">
        <option value=\"\">-- Batch Actions --</option>
        <option value=\"delete\">Delete Selected</option>
        <option value=\"deactivate\">Deactivate Selected</option>
        <option value=\"activate\">Activate Selected</option>
        <option value=\"export_csv\">Export to CSV</option>
        <option value=\"export_json\">Export to JSON</option>
        <option value=\"duplicate\">Duplicate Selected</option>
    </select>",
    );
    out.push_str("<button type=\"submit\" class=\"nexus-btn nexus-btn-secondary\" style=\"padding: 4px 12px;\">Apply</button>");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            "<span class=\"nexus-page-badge\" style=\"margin-left: 16px;\">Page {page}</span>"
        ),
    );
    out.push_str("</div>");

    out.push_str("</div>");

    out.push_str("<div class=\"nexus-table-wrap\">");
    out.push_str("<table class=\"nexus-table\">");
    out.push_str("<thead><tr class=\"nexus-thead-row\">");
    out.push_str(&headers);
    out.push_str("<th class=\"nexus-th nexus-th-actions\">Actions</th>");
    out.push_str("</tr></thead>");
    out.push_str("<tbody id=\"nexus-table-body\">");
    out.push_str(&rows);
    out.push_str("</tbody></table></div>");
    out.push_str("</form>");

    out.push_str("<div class=\"nexus-pagination\">");
    out.push_str(&prev_btn);
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!("<span class=\"nexus-page-indicator\">Page {page}</span>"),
    );
    out.push_str(&next_btn);
    out.push_str("</div>");

    out.push_str("<dialog id=\"nexus-modal\" class=\"nexus-modal\">");
    out.push_str("<div class=\"nexus-modal-inner\">");
    out.push_str("<button class=\"nexus-modal-close\" onclick=\"document.getElementById(&quot;nexus-modal&quot;).close()\">&#x2715;</button>");
    out.push_str("<div id=\"nexus-modal-body\"></div>");
    out.push_str("</div></dialog>");

    out.push_str("<div id=\"nexus-toast\" class=\"nexus-toast\" aria-live=\"polite\"></div>");

    out
}

#[cfg_attr(mutants, mutants::skip)]
async fn fetch_record_data(
    entry: &RegistryEntry,
    id: Option<&str>,
) -> std::collections::HashMap<String, String> {
    let mut record_data = std::collections::HashMap::new();
    if let Some(i) = id {
        let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
        let pk_placeholder = if driver == "postgres" { "$1" } else { "?" };
        let clean_table = sanitize_identifier(entry.table);
        let clean_pk = sanitize_identifier(entry.pk);
        let sql = format!(
            "SELECT * FROM {} WHERE {} = {}",
            clean_table, clean_pk, pk_placeholder
        );

        if let Some(pool) = rullst_core::db::safe_pool() {
            use rullst_orm::_sqlx::Row;
            if let Ok(row) =
                rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()))
                    .bind(i)
                    .fetch_one(pool)
                    .await
            {
                for f in &entry.fields {
                    let val_str = match &f.kind {
                        FieldKind::Boolean => row
                            .try_get::<bool, _>(f.name)
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                        FieldKind::Number | FieldKind::ForeignKey { .. } => {
                            if let Ok(v) = row.try_get::<i64, _>(f.name) {
                                v.to_string()
                            } else if let Ok(v) = row.try_get::<f64, _>(f.name) {
                                v.to_string()
                            } else if let Ok(v) = row.try_get::<i32, _>(f.name) {
                                v.to_string()
                            } else {
                                "".to_string()
                            }
                        }
                        _ => row.try_get::<String, _>(f.name).unwrap_or_default(),
                    };
                    record_data.insert(f.name.to_string(), val_str);
                }
            }
        }
    }
    record_data
}

#[cfg_attr(mutants, mutants::skip)]
async fn render_form_fields_html(
    state: &NexusState,
    entry: &RegistryEntry,
    record_data: &std::collections::HashMap<String, String>,
) -> String {
    let mut fk_set = tokio::task::JoinSet::new();
    let pool_opt = rullst_core::db::safe_pool().cloned();

    for f in &entry.fields {
        if let FieldKind::ForeignKey {
            table: target_table,
            label_col,
        } = &f.kind
            && let Some(pool) = pool_opt.clone()
        {
            let target_pk = state
                .registry
                .iter()
                .find(|e| e.table == *target_table)
                .map(|e| e.pk)
                .unwrap_or("id");

            let clean_target_pk = sanitize_identifier(target_pk);
            let clean_label_col = sanitize_identifier(label_col);
            let clean_target_table = sanitize_identifier(target_table);
            let sql = format!(
                "SELECT {} as key_id, {} as val_label FROM {}",
                clean_target_pk, clean_label_col, clean_target_table
            );

            let fname = f.name.to_string();
            fk_set.spawn(async move {
                let res = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()))
                    .fetch_all(&pool)
                    .await;
                (fname, res)
            });
        }
    }

    let mut fk_results = std::collections::HashMap::new();
    while let Some(res) = fk_set.join_next().await {
        if let Ok((name, Ok(rows))) = res {
            fk_results.insert(name, rows);
        }
    }

    let mut fields_html = String::new();
    for f in &entry.fields {
        let is_pk = f.name == entry.pk;
        let readonly = if is_pk { "readonly" } else { "" };
        let ro_badge = if is_pk {
            "<span class=\"nexus-badge\">READONLY</span>"
        } else {
            ""
        };
        let val = record_data.get(f.name).map(|s| s.as_str()).unwrap_or("");
        let val_esc = rullst_core::html::escape_str(val);

        if f.kind == FieldKind::Boolean {
            let checked = if val == "true" { "checked" } else { "" };
            let _ = std::fmt::Write::write_fmt(
                &mut fields_html,
                format_args!(
                    "<div class=\"nexus-form-group\">\
                 <label class=\"nexus-label\">{} {ro_badge}</label>\
                 <label class=\"nexus-toggle\">\
                 <input type=\"checkbox\" name=\"{}\" value=\"true\" {checked} {readonly}>\
                 <span class=\"nexus-toggle-slider\"></span>\
                 </label>\
                 </div>",
                    f.label, f.name
                ),
            );
        } else if let FieldKind::Enum { options } = &f.kind {
            let mut options_html = String::new();
            for opt in options {
                let selected = if opt == &val { "selected" } else { "" };
                let _ = std::fmt::Write::write_fmt(
                    &mut options_html,
                    format_args!("<option value=\"{}\" {}>{}</option>", opt, selected, opt),
                );
            }
            let _ = std::fmt::Write::write_fmt(
                &mut fields_html,
                format_args!(
                    "<div class=\"nexus-form-group\">\
                 <label class=\"nexus-label\">{} {ro_badge}</label>\
                 <select name=\"{}\" class=\"nexus-input\" {readonly}>\
                 {}\
                 </select>\
                 </div>",
                    f.label, f.name, options_html
                ),
            );
        } else if f.kind == FieldKind::Textarea {
            let _ = std::fmt::Write::write_fmt(
                &mut fields_html,
                format_args!(
                    "<div class=\"nexus-form-group\">\
                 <label class=\"nexus-label\">{} {ro_badge}</label>\
                 <textarea name=\"{}\" class=\"nexus-input\" placeholder=\"Enter {}...\" {readonly}>{}</textarea>\
                 </div>",
                    f.label, f.name, f.label, val_esc
                ),
            );
        } else if let FieldKind::ForeignKey { .. } = &f.kind {
            let mut options_html = String::new();
            options_html.push_str("<option value=\"\">-- Select --</option>");

            if let Some(rows) = fk_results.get(f.name) {
                use rullst_orm::_sqlx::Row;
                for r in rows {
                    let id_val: String = match r.try_get::<String, _>("key_id") {
                        Ok(s) => s,
                        Err(_) => match r.try_get::<i64, _>("key_id") {
                            Ok(i) => i.to_string(),
                            Err(_) => match r.try_get::<i32, _>("key_id") {
                                Ok(i) => i.to_string(),
                                Err(_) => "0".to_string(),
                            },
                        },
                    };
                    let label_val = r
                        .try_get::<String, _>("val_label")
                        .unwrap_or_else(|_| "Unknown".to_string());
                    let selected = if id_val == val { "selected" } else { "" };
                    let _ = std::fmt::Write::write_fmt(
                        &mut options_html,
                        format_args!(
                            "<option value=\"{}\" {}>{}</option>",
                            id_val,
                            selected,
                            rullst_core::html::escape_str(&label_val)
                        ),
                    );
                }
            }

            let _ = std::fmt::Write::write_fmt(
                &mut fields_html,
                format_args!(
                    "<div class=\"nexus-form-group\">\
                 <label class=\"nexus-label\">{} {ro_badge}</label>\
                 <select name=\"{}\" class=\"nexus-input\" {readonly}>\
                 {}\
                 </select>\
                 </div>",
                    f.label, f.name, options_html
                ),
            );
        } else {
            let type_attr = match f.kind {
                FieldKind::Number => "number",
                FieldKind::Email => "email",
                FieldKind::Date => "date",
                _ => "text",
            };
            let _ = std::fmt::Write::write_fmt(
                &mut fields_html,
                format_args!(
                    "<div class=\"nexus-form-group\">\
                 <label class=\"nexus-label\">{} {ro_badge}</label>\
                 <input type=\"{type_attr}\" name=\"{}\" class=\"nexus-input\" placeholder=\"Enter {}...\" value=\"{}\" {readonly}>\
                 </div>",
                    f.label, f.name, f.label, val_esc
                ),
            );
        }
    }
    fields_html
}

async fn render_record_form(state: &NexusState, entry: &RegistryEntry, id: Option<&str>) -> String {
    let t = entry.table;
    let title = if let Some(i) = id {
        format!("&#128398;&#65039; Edit {} #{}", entry.label, i)
    } else {
        format!("&#10133; New {}", entry.label.trim_end_matches('s'))
    };

    let action = if let Some(i) = id {
        format!("/nexus/table/{t}/{i}")
    } else {
        format!("/nexus/table/{t}")
    };

    let method = if id.is_some() { "hx-put" } else { "hx-post" };

    let record_data = fetch_record_data(entry, id).await;
    let fields_html = render_form_fields_html(state, entry, &record_data).await;

    let btn_label = if id.is_some() {
        "Save Changes"
    } else {
        "Create"
    };

    format!(
        "<form class=\"nexus-form\" {method}=\"{action}\" \
         hx-target=\"#nexus-toast\" hx-swap=\"outerHTML\" \
         hx-on::after-request=\"if(event.detail.successful) {{ document.getElementById(&quot;nexus-modal&quot;).close(); htmx.ajax(&quot;GET&quot;, &quot;/nexus/table/{t}&quot;, &quot;#nexus-content&quot;); }}\">\
         <h2 class=\"nexus-modal-title\">{title}</h2>\
         <div class=\"nexus-fields-grid\">{fields_html}</div>\
         <div class=\"nexus-form-actions\">\
         <button type=\"button\" class=\"nexus-btn\" onclick=\"document.getElementById(&quot;nexus-modal&quot;).close()\">Cancel</button>\
         <button type=\"submit\" class=\"nexus-btn nexus-btn-primary\">{btn_label}</button>\
         </div></form>"
    )
}

// â”€â”€â”€ Shell Renderer â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn render_shell(state: &NexusState, sidebar: &str, content: &str) -> String {
    let brand = rullst_core::html::escape_str(state.brand.as_str());
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\" data-theme=\"dark\">\n<head>\n");
    out.push_str("<meta charset=\"UTF-8\" />\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!("<title>{brand} &mdash; Nexus Panel</title>\n"),
    );
    out.push_str("<meta name=\"description\" content=\"Rullst Nexus: Auto-Generated CMS &amp; AI Admin Panel\" />\n");
    out.push_str("<script src=\"https://unpkg.com/htmx.org@2.0.4\" defer></script>\n");
    out.push_str("<script>\n");
    out.push_str("document.addEventListener('htmx:configRequest', function(evt) {\n");
    out.push_str(
        "    let match = document.cookie.match(new RegExp('(^| )rullst_csrf=([^;]+)'));\n",
    );
    out.push_str("    if (match) {\n");
    out.push_str("        evt.detail.headers['X-CSRF-Token'] = match[2];\n");
    out.push_str("    }\n");
    out.push_str("});\n");
    out.push_str("</script>\n");
    out.push_str("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\n");
    out.push_str("<link href=\"https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&amp;family=JetBrains+Mono:wght@400;500&amp;display=swap\" rel=\"stylesheet\">\n");
    out.push_str("<style>\n");
    out.push_str(NEXUS_CSS);
    out.push_str("\n</style>\n</head>\n<body class=\"nexus-body\">\n");

    out.push_str("<nav class=\"nexus-sidebar\" id=\"nexus-sidebar\">");
    out.push_str("<div class=\"nexus-brand\">");
    out.push_str("<span class=\"nexus-brand-icon\">&#127963;&#65039;</span>");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!("<span class=\"nexus-brand-name\">{brand}</span>"),
    );
    out.push_str("</div>");
    out.push_str("<div class=\"nexus-nav-label\">MODELS</div>");
    out.push_str(sidebar);
    out.push_str("<div class=\"nexus-sidebar-footer\">");
    out.push_str("<a href=\"/\" class=\"nexus-nav-link nexus-nav-home\"><span class=\"nexus-nav-icon\">&#127968;</span><span>Back to App</span></a>");
    let _ = std::fmt::Write::write_fmt(
        &mut out,
        format_args!(
            "<div class=\"nexus-version\">Rullst Nexus v{}</div>",
            env!("CARGO_PKG_VERSION")
        ),
    );
    out.push_str("</div></nav>");

    out.push_str("<main class=\"nexus-main\">");
    out.push_str("<header class=\"nexus-topbar\">");
    out.push_str("<button class=\"nexus-topbar-toggle\" onclick=\"document.getElementById(&quot;nexus-sidebar&quot;).classList.toggle(&quot;nexus-sidebar-open&quot;)\">&#9776;</button>");
    out.push_str("<div class=\"nexus-topbar-breadcrumb\" id=\"nexus-breadcrumb\">Dashboard</div>");
    out.push_str("<div class=\"nexus-topbar-actions\">");
    out.push_str("<div class=\"nexus-htmx-indicator\" id=\"nexus-htmx-indicator\">");
    out.push_str("<span class=\"nexus-spinner\"></span>Loading...");
    out.push_str("</div></div></header>");
    out.push_str(
        "<div class=\"nexus-content\" id=\"nexus-content\" hx-indicator=\"#nexus-htmx-indicator\">",
    );
    out.push_str(content);
    out.push_str("</div></main>\n</body>\n</html>");
    out
}

// â”€â”€â”€ Premium Dark-Mode CSS â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

const NEXUS_CSS: &str = "
/* == Reset & Base ===================================================== */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
    --bg-900:  #0b0d14;
    --bg-800:  #111520;
    --bg-700:  #171c2e;
    --bg-600:  #1e253d;
    --bg-500:  #262f4a;
    --border:  rgba(99,116,183,0.18);
    --accent:  #6366f1;
    --accent-h: #818cf8;
    --accent-glow: rgba(99,102,241,0.35);
    --text-100: #f1f5f9;
    --text-300: #94a3b8;
    --text-500: #475569;
    --green:   #10b981;
    --red:     #ef4444;
    --yellow:  #f59e0b;
    --radius:  12px;
    --radius-sm: 8px;
    --shadow:  0 8px 32px rgba(0,0,0,0.45);
    --sidebar-w: 240px;
    --topbar-h: 56px;
    --font-sans: 'Inter', -apple-system, sans-serif;
    --font-mono: 'JetBrains Mono', monospace;
    --transition: 0.2s cubic-bezier(0.4,0,0.2,1);
}
html, body { height: 100%; }
.nexus-body { font-family: var(--font-sans); background: var(--bg-900); color: var(--text-100); display: flex; height: 100vh; overflow: hidden; }

/* == Sidebar =========================================================== */
.nexus-sidebar { width: var(--sidebar-w); min-width: var(--sidebar-w); height: 100vh; background: var(--bg-800); border-right: 1px solid var(--border); display: flex; flex-direction: column; overflow-y: auto; z-index: 100; transition: transform var(--transition); padding-bottom: 16px; }
.nexus-brand { display: flex; align-items: center; gap: 10px; padding: 20px 20px 16px; border-bottom: 1px solid var(--border); margin-bottom: 12px; flex-shrink: 0; }
.nexus-brand-icon { font-size: 22px; }
.nexus-brand-name { font-size: 15px; font-weight: 700; background: linear-gradient(135deg, #818cf8, #c084fc); -webkit-background-clip: text; -webkit-text-fill-color: transparent; background-clip: text; }
.nexus-nav-label { font-size: 10px; font-weight: 600; letter-spacing: 0.1em; color: var(--text-500); padding: 0 20px 8px; text-transform: uppercase; }
.nexus-nav-link { display: flex; align-items: center; gap: 10px; padding: 9px 20px; color: var(--text-300); text-decoration: none; font-size: 13.5px; font-weight: 500; border-left: 3px solid transparent; transition: background var(--transition), color var(--transition); cursor: pointer; }
.nexus-nav-link:hover { background: var(--bg-700); color: var(--text-100); }
.nexus-nav-active { background: linear-gradient(90deg, rgba(99,102,241,0.15), transparent); color: var(--accent-h) !important; border-left-color: var(--accent) !important; }
.nexus-nav-ai { color: #c084fc !important; }
.nexus-nav-ai:hover { background: rgba(192,132,252,0.08) !important; }
.nexus-nav-icon { font-size: 16px; width: 20px; text-align: center; }
.nexus-nav-divider { height: 1px; background: var(--border); margin: 12px 16px; }
.nexus-sidebar-footer { margin-top: auto; padding-top: 8px; border-top: 1px solid var(--border); }
.nexus-version { font-size: 10px; color: var(--text-500); text-align: center; padding: 8px; }

/* == Main Layout ======================================================= */
.nexus-main { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-width: 0; }
.nexus-topbar { height: var(--topbar-h); background: var(--bg-800); border-bottom: 1px solid var(--border); display: flex; align-items: center; gap: 16px; padding: 0 24px; flex-shrink: 0; }
.nexus-topbar-toggle { background: none; border: none; color: var(--text-300); font-size: 18px; cursor: pointer; display: none; padding: 4px 8px; border-radius: 6px; }
.nexus-topbar-toggle:hover { background: var(--bg-700); color: var(--text-100); }
.nexus-topbar-breadcrumb { font-size: 13px; color: var(--text-300); flex: 1; }
.nexus-topbar-actions { display: flex; align-items: center; gap: 12px; }
.nexus-htmx-indicator { display: none; align-items: center; gap: 6px; font-size: 12px; color: var(--accent-h); }
.htmx-request .nexus-htmx-indicator { display: flex; }
.nexus-spinner { width: 14px; height: 14px; border: 2px solid rgba(99,102,241,0.3); border-top-color: var(--accent); border-radius: 50%; animation: nexus-spin 0.6s linear infinite; }
@keyframes nexus-spin { to { transform: rotate(360deg); } }
.nexus-content { flex: 1; overflow-y: auto; padding: 28px 32px; background: var(--bg-900); }

/* == Page Header ======================================================= */
.nexus-page-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 28px; }
.nexus-page-title { font-size: 24px; font-weight: 700; color: var(--text-100); line-height: 1.2; }
.nexus-page-subtitle { font-size: 13.5px; color: var(--text-300); margin-top: 4px; }
.nexus-page-subtitle code { font-family: var(--font-mono); background: var(--bg-600); padding: 1px 6px; border-radius: 4px; font-size: 12px; }

/* == Dashboard Cards =================================================== */
.nexus-stat-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 16px; margin-bottom: 28px; }
.nexus-stat-card { background: var(--bg-700); border: 1px solid var(--border); border-radius: var(--radius); padding: 24px 20px; text-decoration: none; color: var(--text-100); cursor: pointer; transition: all var(--transition); display: flex; flex-direction: column; gap: 8px; position: relative; overflow: hidden; }
.nexus-stat-card::before { content: ''; position: absolute; inset: 0; background: linear-gradient(135deg, var(--accent-glow), transparent); opacity: 0; transition: opacity var(--transition); }
.nexus-stat-card:hover { border-color: var(--accent); transform: translateY(-2px); box-shadow: 0 8px 24px var(--accent-glow); }
.nexus-stat-card:hover::before { opacity: 1; }
.nexus-stat-icon { font-size: 32px; }
.nexus-stat-label { font-weight: 600; font-size: 15px; }
.nexus-stat-hint { font-size: 12px; color: var(--text-300); }
.nexus-welcome-box { background: linear-gradient(135deg, var(--bg-700), var(--bg-600)); border: 1px solid var(--border); border-radius: var(--radius); padding: 32px; text-align: center; display: flex; flex-direction: column; align-items: center; gap: 12px; }
.nexus-welcome-icon { font-size: 40px; }
.nexus-welcome-box h2 { font-size: 18px; font-weight: 600; }
.nexus-welcome-box p { color: var(--text-300); max-width: 480px; font-size: 14px; }

/* == Toolbar =========================================================== */
.nexus-toolbar { display: flex; align-items: center; gap: 12px; margin-bottom: 16px; }
.nexus-search-wrap { position: relative; flex: 1; max-width: 360px; }
.nexus-search-icon { position: absolute; left: 12px; top: 50%; transform: translateY(-50%); font-size: 14px; pointer-events: none; }
.nexus-search-input { width: 100%; background: var(--bg-700); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-100); font-family: var(--font-sans); font-size: 13.5px; padding: 9px 12px 9px 36px; outline: none; transition: border-color var(--transition), box-shadow var(--transition); }
.nexus-search-input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-glow); }
.nexus-page-badge { font-size: 12px; color: var(--text-300); background: var(--bg-700); border: 1px solid var(--border); border-radius: 20px; padding: 4px 12px; }

/* == Table ============================================================= */
.nexus-table-wrap { background: var(--bg-800); border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; margin-bottom: 16px; }
.nexus-table { width: 100%; border-collapse: collapse; font-size: 13.5px; }
.nexus-thead-row { background: var(--bg-700); border-bottom: 1px solid var(--border); }
.nexus-th { text-align: left; padding: 12px 16px; font-weight: 600; font-size: 12px; letter-spacing: 0.04em; color: var(--text-300); text-transform: uppercase; white-space: nowrap; }
.nexus-th-actions { text-align: right; }
.nexus-tr { border-bottom: 1px solid var(--border); transition: background var(--transition); }
.nexus-tr:last-child { border-bottom: none; }
.nexus-tr:hover { background: var(--bg-700); }
.nexus-td { padding: 13px 16px; color: var(--text-100); vertical-align: middle; }
.nexus-td-actions { text-align: right; white-space: nowrap; }
.nexus-empty-row { padding: 32px; text-align: center; color: var(--text-500); }
.nexus-row-deleted { opacity: 0.4; }

/* == Action Buttons ==================================================== */
.nexus-action-btn { background: none; border: 1px solid var(--border); border-radius: 6px; padding: 5px 10px; cursor: pointer; font-size: 13px; color: var(--text-300); transition: all var(--transition); margin-left: 4px; }
.nexus-action-edit:hover { border-color: var(--accent); color: var(--accent-h); background: var(--accent-glow); }
.nexus-action-delete:hover { border-color: var(--red); color: var(--red); background: rgba(239,68,68,0.1); }

/* == Pagination ======================================================== */
.nexus-pagination { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 16px; }
.nexus-page-indicator { font-size: 13px; color: var(--text-300); }

/* == Buttons =========================================================== */
.nexus-btn { display: inline-flex; align-items: center; gap: 6px; padding: 9px 18px; border-radius: var(--radius-sm); font-family: var(--font-sans); font-size: 13.5px; font-weight: 600; cursor: pointer; text-decoration: none; border: 1px solid transparent; transition: all var(--transition); white-space: nowrap; }
.nexus-btn-primary { background: var(--accent); color: #fff; border-color: var(--accent); }
.nexus-btn-primary:hover { background: var(--accent-h); box-shadow: 0 4px 16px var(--accent-glow); transform: translateY(-1px); }
.nexus-btn-ghost { background: transparent; color: var(--text-300); border-color: var(--border); }
.nexus-btn-ghost:hover { background: var(--bg-700); color: var(--text-100); border-color: var(--accent); }
.nexus-btn-ai { background: linear-gradient(135deg, #7c3aed, #c026d3); color: #fff; border: none; }
.nexus-btn-ai:hover { filter: brightness(1.15); box-shadow: 0 4px 20px rgba(192,38,211,0.4); transform: translateY(-1px); }

/* == Toast ============================================================= */
.nexus-toast { position: fixed; bottom: 24px; right: 24px; padding: 12px 20px; border-radius: var(--radius-sm); font-size: 13.5px; font-weight: 500; z-index: 1000; box-shadow: var(--shadow); }
.nexus-toast-success { background: rgba(16,185,129,0.15); border: 1px solid var(--green); color: var(--green); animation: nexus-toast-in 0.3s ease; }
.nexus-toast-warning { background: rgba(245,158,11,0.15); border: 1px solid var(--yellow); color: var(--yellow); animation: nexus-toast-in 0.3s ease; }
@keyframes nexus-toast-in { from { opacity: 0; transform: translateY(12px); } to { opacity: 1; transform: translateY(0); } }

/* == Modal ============================================================= */
.nexus-modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    margin: 0;
    background: var(--bg-800);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 0;
    color: var(--text-100);
    max-width: 500px;
    width: 90vw;
    box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5);
}
.nexus-modal::backdrop { background: rgba(0,0,0,0.7); backdrop-filter: blur(4px); }
.nexus-modal-inner { padding: 28px; }
.nexus-modal-close { position: absolute; top: 16px; right: 16px; background: var(--bg-700); border: 1px solid var(--border); color: var(--text-300); border-radius: 6px; width: 28px; height: 28px; cursor: pointer; font-size: 13px; display: flex; align-items: center; justify-content: center; transition: all var(--transition); }
.nexus-modal-close:hover { background: var(--red); border-color: var(--red); color: #fff; }
.nexus-modal-title { font-size: 18px; font-weight: 700; margin-bottom: 20px; }

/* == Form ============================================================== */
.nexus-fields-grid { display: grid; grid-template-columns: 1fr; gap: 16px; margin-bottom: 24px; }
.nexus-form-group { display: flex; flex-direction: column; gap: 6px; }
.nexus-label { font-size: 12px; font-weight: 600; color: var(--text-300); text-transform: uppercase; letter-spacing: 0.04em; display: flex; align-items: center; gap: 6px; }
.nexus-input { background: var(--bg-700); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-100); font-family: var(--font-sans); font-size: 13.5px; padding: 10px 12px; width: 100%; outline: none; }
.nexus-input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-glow); }
.nexus-badge { background: var(--bg-500); border: 1px solid var(--border); color: var(--text-500); border-radius: 4px; font-size: 10px; padding: 1px 5px; }
.nexus-form-actions { display: flex; justify-content: flex-end; gap: 10px; border-top: 1px solid var(--border); padding-top: 20px; }

/* == Chat ============================================================== */
.nexus-chat-layout { display: grid; grid-template-columns: 280px 1fr; gap: 20px; height: calc(100vh - var(--topbar-h) - 160px); }
.nexus-chat-schema { background: var(--bg-800); border: 1px solid var(--border); border-radius: var(--radius); padding: 20px; overflow-y: auto; }
.nexus-schema-title { font-size: 12px; font-weight: 700; color: var(--text-500); text-transform: uppercase; letter-spacing: 0.06em; margin-bottom: 12px; }
.nexus-schema-pre { font-family: var(--font-mono); font-size: 11.5px; color: var(--text-300); white-space: pre-wrap; word-break: break-all; line-height: 1.6; }
.nexus-chat-panel { background: var(--bg-800); border: 1px solid var(--border); border-radius: var(--radius); display: flex; flex-direction: column; overflow: hidden; }
.nexus-chat-messages { flex: 1; overflow-y: auto; padding: 20px; display: flex; flex-direction: column; gap: 16px; }
.nexus-chat-bubble { display: flex; gap: 12px; align-items: flex-start; animation: nexus-bubble-in 0.25s ease; }
@keyframes nexus-bubble-in { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
.nexus-chat-user { flex-direction: row-reverse; }
.nexus-chat-avatar { width: 36px; height: 36px; border-radius: 50%; background: var(--bg-600); border: 1px solid var(--border); display: flex; align-items: center; justify-content: center; font-size: 18px; flex-shrink: 0; }
.nexus-chat-text { background: var(--bg-700); border: 1px solid var(--border); border-radius: 12px; padding: 12px 16px; font-size: 13.5px; line-height: 1.6; max-width: 80%; }
.nexus-chat-user .nexus-chat-text { background: linear-gradient(135deg, rgba(99,102,241,0.25), rgba(99,102,241,0.1)); border-color: rgba(99,102,241,0.4); }
.nexus-chat-form { display: flex; gap: 10px; padding: 16px; border-top: 1px solid var(--border); background: var(--bg-900); }
.nexus-chat-input { flex: 1; background: var(--bg-700); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-100); font-family: var(--font-sans); font-size: 13.5px; padding: 10px 14px; outline: none; transition: border-color var(--transition); }
.nexus-chat-input:focus { border-color: var(--accent); }
.nexus-code { font-family: var(--font-mono); background: var(--bg-900); border: 1px solid var(--border); border-radius: 6px; padding: 8px 12px; display: block; font-size: 12px; color: #a5f3fc; white-space: pre-wrap; margin: 8px 0; }

/* == Responsive ======================================================== */
@media (max-width: 900px) {
    .nexus-sidebar { position: fixed; left: 0; top: 0; bottom: 0; transform: translateX(-100%); }
    .nexus-sidebar-open { transform: translateX(0); }
    .nexus-topbar-toggle { display: flex; }
    .nexus-content { padding: 20px 16px; }
    .nexus-chat-layout { grid-template-columns: 1fr; }
    .nexus-chat-schema { max-height: 160px; }
    .nexus-fields-grid { grid-template-columns: 1fr; }
}
";
#[derive(serde::Deserialize)]
struct BatchActionForm {
    action: String,
    #[serde(default)]
    selected_ids: Vec<String>,
}

#[cfg_attr(mutants, mutants::skip)]
async fn nexus_batch_action(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    axum::extract::Form(form): axum::extract::Form<BatchActionForm>,
) -> Response {
    if form.selected_ids.is_empty() {
        return axum::response::Redirect::to(&format!("/nexus/table/{}", table)).into_response();
    }

    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Table not found").into_response(),
    };

    let Some(pool) = rullst_core::db::safe_pool() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database not configured").into_response();
    };

    let clean_table = sanitize_identifier(entry.table);
    let clean_pk = sanitize_identifier(entry.pk);
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");

    let mut placeholders = Vec::new();
    for i in 1..=form.selected_ids.len() {
        if driver == "postgres" {
            placeholders.push(format!("${}", i));
        } else {
            placeholders.push("?".to_string());
        }
    }
    let placeholders_str = placeholders.join(",");

    match form.action.as_str() {
        "delete" => {
            let sql = format!(
                "DELETE FROM {} WHERE {} IN ({})",
                clean_table, clean_pk, placeholders_str
            );
            let mut query =
                rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
            for id in &form.selected_ids {
                query = query.bind(id);
            }
            let _ = query.execute(pool).await;
        }
        "deactivate" | "activate" => {
            let has_active = entry
                .fields
                .iter()
                .any(|f| f.name == "is_active" || f.name == "active");
            if has_active {
                let field_name = if entry.fields.iter().any(|f| f.name == "is_active") {
                    "is_active"
                } else {
                    "active"
                };
                let val_sqlite = if form.action == "activate" { "1" } else { "0" };
                let val_pg = if form.action == "activate" {
                    "true"
                } else {
                    "false"
                };
                let set_val = if driver == "sqlite" {
                    val_sqlite
                } else {
                    val_pg
                };
                let sql = format!(
                    "UPDATE {} SET {} = {} WHERE {} IN ({})",
                    clean_table, field_name, set_val, clean_pk, placeholders_str
                );
                let mut query =
                    rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
                for id in &form.selected_ids {
                    query = query.bind(id);
                }
                let _ = query.execute(pool).await;
            }
        }
        "export_csv" => {
            let sql = format!(
                "SELECT * FROM {} WHERE {} IN ({})",
                clean_table, clean_pk, placeholders_str
            );
            let mut query =
                rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
            for id in &form.selected_ids {
                query = query.bind(id);
            }
            if let Ok(rows) = query.fetch_all(pool).await {
                let mut csv = String::new();
                use rullst_orm::_sqlx::Row;
                let headers: Vec<&str> = entry.fields.iter().map(|f| f.name).collect();
                csv.push_str(&headers.join(","));
                csv.push('\n');
                for row in rows {
                    let mut row_vals = Vec::new();
                    for f in &entry.fields {
                        let val_str = match f.kind {
                            FieldKind::Boolean => row
                                .try_get::<bool, _>(f.name)
                                .map(|v| v.to_string())
                                .unwrap_or_default(),
                            FieldKind::Number | FieldKind::ForeignKey { .. } => {
                                if let Ok(v) = row.try_get::<i64, _>(f.name) {
                                    v.to_string()
                                } else if let Ok(v) = row.try_get::<f64, _>(f.name) {
                                    v.to_string()
                                } else if let Ok(v) = row.try_get::<i32, _>(f.name) {
                                    v.to_string()
                                } else {
                                    "".to_string()
                                }
                            }
                            _ => row.try_get::<String, _>(f.name).unwrap_or_default(),
                        };
                        row_vals.push(format!("\"{}\"", val_str.replace("\"", "\"\"")));
                    }
                    csv.push_str(&row_vals.join(","));
                    csv.push('\n');
                }
                return axum::response::Response::builder()
                    .header("Content-Type", "text/csv")
                    .header(
                        "Content-Disposition",
                        format!("attachment; filename=\"{}_export.csv\"", table),
                    )
                    .body(axum::body::Body::from(csv))
                    .unwrap();
            }
        }
        "export_json" => {
            let sql = format!(
                "SELECT * FROM {} WHERE {} IN ({})",
                clean_table, clean_pk, placeholders_str
            );
            let mut query =
                rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
            for id in &form.selected_ids {
                query = query.bind(id);
            }
            if let Ok(rows) = query.fetch_all(pool).await {
                let mut out_arr = Vec::new();
                use rullst_orm::_sqlx::Row;
                for row in rows {
                    let mut obj = serde_json::Map::new();
                    for f in &entry.fields {
                        let val = match f.kind {
                            FieldKind::Boolean => serde_json::Value::Bool(
                                row.try_get::<bool, _>(f.name).unwrap_or(false),
                            ),
                            FieldKind::Number | FieldKind::ForeignKey { .. } => {
                                if let Ok(v) = row.try_get::<i64, _>(f.name) {
                                    serde_json::Value::Number(v.into())
                                } else if let Ok(v) = row.try_get::<f64, _>(f.name) {
                                    serde_json::Number::from_f64(v)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null)
                                } else if let Ok(v) = row.try_get::<i32, _>(f.name) {
                                    serde_json::Value::Number(v.into())
                                } else {
                                    serde_json::Value::Null
                                }
                            }
                            _ => serde_json::Value::String(
                                row.try_get::<String, _>(f.name).unwrap_or_default(),
                            ),
                        };
                        obj.insert(f.name.to_string(), val);
                    }
                    out_arr.push(serde_json::Value::Object(obj));
                }
                let json_str =
                    serde_json::to_string_pretty(&out_arr).unwrap_or_else(|_| "[]".to_string());
                return axum::response::Response::builder()
                    .header("Content-Type", "application/json")
                    .header(
                        "Content-Disposition",
                        format!("attachment; filename=\"{}_export.json\"", table),
                    )
                    .body(axum::body::Body::from(json_str))
                    .unwrap();
            }
        }
        "duplicate" => {
            let sql = format!(
                "SELECT * FROM {} WHERE {} IN ({})",
                clean_table, clean_pk, placeholders_str
            );
            let mut query =
                rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(sql.as_str()));
            for id in &form.selected_ids {
                query = query.bind(id);
            }
            if let Ok(rows) = query.fetch_all(pool).await {
                use rullst_orm::_sqlx::Row;
                for row in rows {
                    let mut cols = Vec::new();
                    let mut vals = Vec::new();
                    let mut i = 1;

                    for f in &entry.fields {
                        if f.name == entry.pk {
                            continue;
                        }
                        cols.push(sanitize_identifier(f.name));
                        if driver == "postgres" {
                            vals.push(format!("${}", i));
                            i += 1;
                        } else {
                            vals.push("?".to_string());
                        }
                    }

                    let ins_sql = format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        clean_table,
                        cols.join(","),
                        vals.join(",")
                    );
                    let mut ins_query = rullst_orm::_sqlx::query(rullst_orm::_sqlx::AssertSqlSafe(
                        ins_sql.as_str(),
                    ));

                    for f in &entry.fields {
                        if f.name == entry.pk {
                            continue;
                        }

                        match f.kind {
                            FieldKind::Boolean => {
                                let b = row.try_get::<bool, _>(f.name).unwrap_or(false);
                                ins_query = ins_query.bind(b);
                            }
                            FieldKind::Number | FieldKind::ForeignKey { .. } => {
                                if let Ok(v) = row.try_get::<i64, _>(f.name) {
                                    ins_query = ins_query.bind(v);
                                } else if let Ok(v) = row.try_get::<f64, _>(f.name) {
                                    ins_query = ins_query.bind(v);
                                } else if let Ok(v) = row.try_get::<i32, _>(f.name) {
                                    ins_query = ins_query.bind(v);
                                } else {
                                    ins_query = ins_query.bind(0);
                                }
                            }
                            _ => {
                                let s = row.try_get::<String, _>(f.name).unwrap_or_default();
                                ins_query = ins_query.bind(s);
                            }
                        }
                    }
                    let _ = ins_query.execute(pool).await;
                }
            }
        }
        _ => {}
    }

    axum::response::Redirect::to(&format!("/nexus/table/{}", table)).into_response()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(not(miri))]
mod tests {
    use super::*;

    static INIT_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    async fn init_test_db() {
        let _guard = INIT_MUTEX.lock().await;
        let is_init = rullst_core::db::safe_pool().is_some();
        if !is_init {
            rullst_orm::Orm::init("sqlite://test_nexus.db?mode=rwc")
                .await
                .expect("Failed to init SQLite DB file");
        }
        if let Some(pool) = rullst_core::db::safe_pool() {
            rullst_orm::_sqlx::query(
                "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)",
            )
            .execute(pool)
            .await
            .expect("Failed to CREATE TABLE users");
            rullst_orm::_sqlx::query("INSERT OR IGNORE INTO users (id, name, email) VALUES (42, 'Test User', 'example.com')")
                .execute(pool)
                .await
                .expect("Failed to INSERT mock user");
        }
    }

    struct TestUser;
    impl NexusModel for TestUser {
        fn nexus_table() -> &'static str {
            "users"
        }
        fn nexus_label() -> &'static str {
            "Users"
        }
        fn nexus_icon() -> &'static str {
            "ðŸ‘¤"
        }
        fn nexus_fields() -> Vec<FieldMeta> {
            vec![
                FieldMeta {
                    name: "id",
                    label: "ID",
                    kind: FieldKind::Number,
                    hidden: true,
                    readonly: true,
                },
                FieldMeta {
                    name: "name",
                    label: "Name",
                    kind: FieldKind::Text,
                    hidden: false,
                    readonly: false,
                },
                FieldMeta {
                    name: "email",
                    label: "Email",
                    kind: FieldKind::Email,
                    hidden: false,
                    readonly: false,
                },
            ]
        }
    }

    struct TestPost;
    impl NexusModel for TestPost {
        fn nexus_table() -> &'static str {
            "posts"
        }
        fn nexus_label() -> &'static str {
            "Posts"
        }
        fn nexus_icon() -> &'static str {
            "ðŸ“"
        }
        fn nexus_fields() -> Vec<FieldMeta> {
            vec![
                FieldMeta {
                    name: "id",
                    label: "ID",
                    kind: FieldKind::Number,
                    hidden: true,
                    readonly: true,
                },
                FieldMeta {
                    name: "title",
                    label: "Title",
                    kind: FieldKind::Text,
                    hidden: false,
                    readonly: false,
                },
                FieldMeta {
                    name: "published",
                    label: "Published",
                    kind: FieldKind::Boolean,
                    hidden: false,
                    readonly: false,
                },
            ]
        }
    }

    #[test]
    fn test_nexus_model_trait_user() {
        assert_eq!(TestUser::nexus_table(), "users");
        assert_eq!(TestUser::nexus_label(), "Users");
        assert_eq!(TestUser::nexus_icon(), "ðŸ‘¤");
        assert_eq!(TestUser::nexus_pk(), "id");
        let fields = TestUser::nexus_fields();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].name, "id");
        assert!(fields[0].hidden);
        assert!(fields[0].readonly);
        assert_eq!(fields[1].name, "name");
        assert!(!fields[1].hidden);
        assert_eq!(fields[2].kind, FieldKind::Email);
    }

    #[test]
    fn test_nexus_model_trait_post() {
        let fields = TestPost::nexus_fields();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[2].kind, FieldKind::Boolean);
    }

    struct TestDefaultIcon;
    impl NexusModel for TestDefaultIcon {
        fn nexus_table() -> &'static str {
            "def"
        }
        fn nexus_label() -> &'static str {
            "Defs"
        }
        fn nexus_fields() -> Vec<FieldMeta> {
            vec![]
        }
    }

    #[test]
    fn test_nexus_icon_default() {
        // Kills the mutant replacing "ðŸ“‹" with "" or "xyzzy"
        assert_eq!(TestDefaultIcon::nexus_icon(), "ðŸ“‹");
    }

    #[test]
    fn test_nexus_with_db() {
        let nexus = Nexus::new()
            .register::<TestUser>()
            .with_db("sqlite::memory:");
        // Kills the mutant that replaces with_db with Default::default()
        assert_eq!(nexus.registry.len(), 1);
    }

    #[test]
    fn test_nexus_builder_registers_models() {
        let nexus = Nexus::new()
            .register::<TestUser>()
            .register::<TestPost>()
            .with_brand("Test App");

        assert_eq!(nexus.registry.len(), 2);
        assert_eq!(nexus.brand, "Test App");
        assert_eq!(nexus.registry[0].table, "users");
        assert_eq!(nexus.registry[1].table, "posts");
    }

    #[test]
    fn test_nexus_build_returns_router() {
        let nexus = Nexus::new().register::<TestUser>().with_brand("My App");
        let _router = nexus.build();
    }

    #[tokio::test]
    async fn test_nexus_auth_failures() {
        use axum::http::{Request, StatusCode, header};
        use tower::Service;

        let nexus = Nexus::new().with_auth("admin", "secret").build();
        let mut app = nexus.into_service();

        // No auth
        let req1 = Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let res1 = app.call(req1).await.unwrap();
        assert_eq!(res1.status(), StatusCode::UNAUTHORIZED);

        // Wrong password (kills && replaced with || mutant)
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode("admin:wrongpass");
        let req2 = Request::builder()
            .uri("/")
            .header(header::AUTHORIZATION, format!("Basic {}", encoded))
            .body(axum::body::Body::empty())
            .unwrap();
        let res2 = app.call(req2).await.unwrap();
        assert_eq!(res2.status(), StatusCode::UNAUTHORIZED);

        // Wrong username (kills && replaced with || mutant)
        let encoded3 = base64::engine::general_purpose::STANDARD.encode("user:secret");
        let req3 = Request::builder()
            .uri("/")
            .header(header::AUTHORIZATION, format!("Basic {}", encoded3))
            .body(axum::body::Body::empty())
            .unwrap();
        let res3 = app.call(req3).await.unwrap();
        assert_eq!(res3.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_field_kind_sql_mapping() {
        assert_eq!(field_kind_sql(&FieldKind::Number), "INTEGER");
        assert_eq!(field_kind_sql(&FieldKind::Text), "TEXT");
        assert_eq!(field_kind_sql(&FieldKind::Boolean), "INTEGER");
        assert_eq!(
            field_kind_sql(&FieldKind::ForeignKey {
                table: "",
                label_col: ""
            }),
            "INTEGER"
        );
        assert_eq!(field_kind_sql(&FieldKind::Date), "TEXT");
        assert_eq!(field_kind_sql(&FieldKind::DateTime), "TEXT");
        assert_eq!(field_kind_sql(&FieldKind::Json), "TEXT");
    }

    #[test]
    fn test_field_kind_label() {
        assert_eq!(field_kind_label(&FieldKind::Url), "url");
        assert_eq!(field_kind_label(&FieldKind::Json), "json");
        assert_eq!(field_kind_label(&FieldKind::Text), "text");
    }

    #[test]
    fn test_field_kind_input_type() {
        assert_eq!(field_kind_input_type(&FieldKind::Email), "email");
        assert_eq!(field_kind_input_type(&FieldKind::Password), "password");
        assert_eq!(field_kind_input_type(&FieldKind::Number), "number");
        assert_eq!(field_kind_input_type(&FieldKind::Text), "text");
        assert_eq!(field_kind_input_type(&FieldKind::Date), "date");
        assert_eq!(
            field_kind_input_type(&FieldKind::DateTime),
            "datetime-local"
        );
        assert_eq!(field_kind_input_type(&FieldKind::Url), "url");
        assert_eq!(
            field_kind_input_type(&FieldKind::ForeignKey {
                table: "",
                label_col: ""
            }),
            "select"
        );
    }

    #[tokio::test]
    async fn test_render_table_rows_with_search() {
        init_test_db().await;
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "ðŸ‘¤",
            pk: "id",
            fields: TestUser::nexus_fields(),
        };
        let rows = render_table_rows(&entry, "example.com", 1).await;
        assert!(
            rows.contains("example.com"),
            "Expected rows to contain 'example.com', but got: {}",
            rows
        );
    }

    #[tokio::test]
    async fn test_render_table_rows_empty_search() {
        init_test_db().await;
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "ðŸ‘¤",
            pk: "id",
            fields: TestUser::nexus_fields(),
        };
        let rows = render_table_rows(&entry, "zzznomatch99999xyz", 1).await;
        assert!(
            rows.contains("No results"),
            "Expected rows to contain 'No results', but got: {}",
            rows
        );
    }

    #[tokio::test]
    async fn test_render_record_form_new() {
        init_test_db().await;
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "ðŸ‘¤",
            pk: "id",
            fields: TestUser::nexus_fields(),
        };
        let state = NexusState {
            registry: Arc::new(vec![entry.clone()]),
            brand: Arc::new("Test App".to_string()),
        };
        let form = render_record_form(&state, &entry, None).await;
        assert!(
            form.contains("New User"),
            "Expected form to contain 'New User', but got: {}",
            form
        );
        assert!(
            form.contains("hx-post"),
            "Expected form to contain 'hx-post', but got: {}",
            form
        );
        assert!(
            form.contains("Create"),
            "Expected form to contain 'Create', but got: {}",
            form
        );
    }

    #[tokio::test]
    async fn test_render_record_form_edit() {
        init_test_db().await;
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "ðŸ‘¤",
            pk: "id",
            fields: TestUser::nexus_fields(),
        };
        let state = NexusState {
            registry: Arc::new(vec![entry.clone()]),
            brand: Arc::new("Test App".to_string()),
        };
        let form = render_record_form(&state, &entry, Some("42")).await;
        assert!(
            form.contains("Edit Users #42"),
            "Expected form to contain 'Edit Users #42', but got: {}",
            form
        );
        assert!(
            form.contains("hx-put"),
            "Expected form to contain 'hx-put', but got: {}",
            form
        );
        assert!(
            form.contains("Save Changes"),
            "Expected form to contain 'Save Changes', but got: {}",
            form
        );
    }

    #[test]
    fn test_find_entry_found() {
        let state = NexusState {
            registry: Arc::new(vec![RegistryEntry {
                table: "users",
                label: "Users",
                icon: "ðŸ‘¤",
                pk: "id",
                fields: TestUser::nexus_fields(),
            }]),
            brand: Arc::new("Test".to_string()),
        };
        assert!(find_entry(&state, "users").is_some());
        assert!(find_entry(&state, "missing").is_none());
    }

    #[test]
    fn test_mock_ai_response_list() {
        let resp = generate_mock_ai_response("list all users", "");
        assert!(resp.contains("SELECT"));
        let resp2 = generate_mock_ai_response("show me the records", "");
        assert!(resp2.contains("SELECT"));
    }

    #[test]
    fn test_mock_ai_response_count() {
        let resp = generate_mock_ai_response("how many posts are there?", "");
        assert!(resp.contains("COUNT"));
        let resp2 = generate_mock_ai_response("quantos usuÃ¡rios temos?", "");
        assert!(resp2.contains("COUNT"));
    }

    #[test]
    fn test_build_table_query() {
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "ðŸ‘¤",
            pk: "id",
            fields: vec![],
        };
        let visible_fields = vec![];
        let (sql, binds) = build_table_query(&entry, &visible_fields, "", 1);
        assert_eq!(
            sql,
            "SELECT * FROM users ORDER BY id DESC LIMIT 20 OFFSET 0"
        );
        assert!(binds.is_empty());

        let f = FieldMeta {
            name: "email",
            label: "Email",
            kind: FieldKind::Email,
            hidden: false,
            readonly: false,
        };
        let visible_fields = vec![&f];
        let (sql2, binds2) = build_table_query(&entry, &visible_fields, "test", 2);
        assert!(sql2.contains("email LIKE ?")); // Assuming SQLite by default
        assert!(sql2.contains("LIMIT 20 OFFSET 20"));
        assert_eq!(binds2.len(), 1);
        assert_eq!(binds2[0], "%test%");
    }

    #[tokio::test]
    async fn test_render_form_fields_html_all_kinds() {
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "ðŸ‘¤",
            pk: "id",
            fields: vec![
                FieldMeta {
                    name: "id",
                    label: "ID",
                    kind: FieldKind::Number,
                    hidden: false,
                    readonly: true,
                },
                FieldMeta {
                    name: "active",
                    label: "Active",
                    kind: FieldKind::Boolean,
                    hidden: false,
                    readonly: false,
                },
                FieldMeta {
                    name: "email",
                    label: "Email",
                    kind: FieldKind::Email,
                    hidden: false,
                    readonly: false,
                },
                FieldMeta {
                    name: "dob",
                    label: "DOB",
                    kind: FieldKind::Date,
                    hidden: false,
                    readonly: false,
                },
            ],
        };
        let state = NexusState {
            registry: std::sync::Arc::new(vec![entry.clone()]),
            brand: std::sync::Arc::new("Test App".to_string()),
        };
        let mut data = std::collections::HashMap::new();
        data.insert("id".to_string(), "42".to_string());
        data.insert("active".to_string(), "true".to_string());

        let html = render_form_fields_html(&state, &entry, &data).await;
        assert!(html.contains("value=\"42\""));
        assert!(html.contains("readonly"));
        assert!(html.contains("checkbox"));
        assert!(html.contains("checked"));
        assert!(html.contains("type=\"email\""));
        assert!(html.contains("type=\"date\""));
    }

    #[tokio::test]
    async fn test_render_table_view_pagination() {
        init_test_db().await;
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "ðŸ‘¤",
            pk: "id",
            fields: TestUser::nexus_fields(),
        };
        let state = NexusState {
            registry: std::sync::Arc::new(vec![entry.clone()]),
            brand: std::sync::Arc::new("Test App".to_string()),
        };
        let html = render_table_view(&state, &entry, 2, "").await;
        assert!(html.contains("&larr; Prev"));

        let html2 = render_table_view(&state, &entry, 1, "").await;
        assert!(html2.contains("<span></span>"));
    }

    #[test]
    fn test_render_sidebar_no_active() {
        let state = NexusState {
            registry: Arc::new(vec![RegistryEntry {
                table: "users",
                label: "Users",
                icon: "ðŸ‘¤",
                pk: "id",
                fields: vec![],
            }]),
            brand: Arc::new("Test".to_string()),
        };
        let sidebar = render_sidebar(&state, None);
        assert!(sidebar.contains("/nexus/table/users"));
        assert!(sidebar.contains("AI Assistant"));
        assert!(!sidebar.contains("nexus-nav-active"));
    }

    #[test]
    fn test_render_sidebar_with_active() {
        let state = NexusState {
            registry: Arc::new(vec![RegistryEntry {
                table: "users",
                label: "Users",
                icon: "ðŸ‘¤",
                pk: "id",
                fields: vec![],
            }]),
            brand: Arc::new("Test".to_string()),
        };
        let sidebar = render_sidebar(&state, Some("users"));
        assert!(sidebar.contains("nexus-nav-active"));
    }

    #[test]
    fn test_render_shell_contains_brand() {
        let state = NexusState {
            registry: Arc::new(vec![]),
            brand: Arc::new("MySaaS".to_string()),
        };
        let html = render_shell(&state, "", "<p>content</p>");
        assert!(html.contains("MySaaS"));
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("nexus-body"));
        assert!(html.contains("<p>content</p>"));
    }

    #[tokio::test]
    async fn test_nexus_with_auth() {
        use axum::http::{Request, StatusCode};
        use base64::Engine;
        use tower::ServiceExt;

        let nexus = Nexus::new()
            .with_brand("Auth Test")
            .with_auth("admin", "secret");

        let router = nexus.build();

        // 1. Request without authorization header -> 401 Unauthorized
        let req = Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = router.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::WWW_AUTHENTICATE)
                .unwrap(),
            "Basic realm=\"Nexus Admin Panel\""
        );

        // 2. Request with incorrect credentials -> 401 Unauthorized
        let req = Request::builder()
            .uri("/")
            .header(
                axum::http::header::AUTHORIZATION,
                format!(
                    "Basic {}",
                    base64::engine::general_purpose::STANDARD.encode("admin:wrong")
                ),
            )
            .body(axum::body::Body::empty())
            .unwrap();

        let response = router.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // 3. Request with correct credentials -> 200 OK
        let req = Request::builder()
            .uri("/")
            .header(
                axum::http::header::AUTHORIZATION,
                format!(
                    "Basic {}",
                    base64::engine::general_purpose::STANDARD.encode("admin:secret")
                ),
            )
            .body(axum::body::Body::empty())
            .unwrap();

        let response = router.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(sanitize_identifier("valid_id_123"), "valid_id_123");
        assert_eq!(sanitize_identifier("invalid-id!"), "invalidid");
        assert_eq!(sanitize_identifier("a_b-c!@#d"), "a_bcd");
        let long_id = "a".repeat(70);
        let sanitized = sanitize_identifier(&long_id);
        assert_eq!(sanitized.len(), 64);
        assert_eq!(sanitized, "a".repeat(64));
    }

    struct DummyModel;
    impl NexusModel for DummyModel {
        fn nexus_table() -> &'static str {
            "dummy"
        }
        fn nexus_label() -> &'static str {
            "Dummies"
        }
        fn nexus_fields() -> Vec<FieldMeta> {
            vec![]
        }
    }

    #[test]
    fn test_nexus_model_defaults() {
        assert_eq!(DummyModel::nexus_icon(), "ðŸ“‹");
        assert_eq!(DummyModel::nexus_pk(), "id");
    }

    #[tokio::test]
    async fn test_render_record_form() {
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "ðŸ‘¤",
            pk: "id",
            fields: vec![FieldMeta {
                name: "email",
                label: "Email",
                kind: FieldKind::Text,
                hidden: false,
                readonly: false,
            }],
        };
        let state = NexusState {
            registry: std::sync::Arc::new(vec![entry.clone()]),
            brand: std::sync::Arc::new("Rullst".to_string()),
        };
        let html = render_record_form(&state, &entry, None).await;
        assert!(html.contains("hx-post"));
        assert!(html.contains("hx-post=\"/nexus/table/users\""));
        assert!(html.contains("name=\"email\""));
    }

    #[test]
    fn test_render_empty_state_html() {
        let html = render_empty_state_html(5, "users", "");
        assert!(html.contains("No records found in table `users`."));
    }

    #[test]
    fn test_nexus_builder() {
        let nexus = Nexus::new()
            .with_brand("CustomBrand")
            .register::<DummyModel>();

        assert_eq!(nexus.brand, "CustomBrand");
        assert_eq!(nexus.registry.len(), 1);
        assert_eq!(nexus.registry[0].table, "dummy");
    }
}
