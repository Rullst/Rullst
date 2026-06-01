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
use std::sync::Arc;

// ─── Field Metadata & Reflection ─────────────────────────────────────────────

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
    /// A short icon/emoji representing the model in the sidebar (e.g. "👤").
    fn nexus_icon() -> &'static str {
        "📋"
    }
    /// A list of FieldMeta describing each column in this model's schema.
    fn nexus_fields() -> Vec<FieldMeta>;
    /// The name of the primary key column (defaults to "id").
    fn nexus_pk() -> &'static str {
        "id"
    }
}

// ─── Registry ─────────────────────────────────────────────────────────────────

/// Internal representation of a registered model used by the Nexus Panel engine.
#[derive(Clone)]
#[allow(dead_code)]
struct RegistryEntry {
    pub table: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub pk: &'static str,
    pub fields: Vec<FieldMeta>,
}

/// Shared state passed into all Nexus route handlers.
#[derive(Clone)]
#[allow(dead_code)]
struct NexusState {
    pub registry: Arc<Vec<RegistryEntry>>,
    pub brand: Arc<String>,
    pub db_url: Arc<Option<String>>,
}

// ─── Nexus Builder ────────────────────────────────────────────────────────────

/// The main entry point for configuring and mounting the Rullst Nexus Panel.
///
/// # Example
/// ```rust,no_run
/// # use rullst::nexus::Nexus;
/// let nexus_router = Nexus::new()
///     .with_brand("My SaaS")
///     .with_db("sqlite://./db.sqlite3")
///     .build();
/// ```
pub struct Nexus {
    registry: Vec<RegistryEntry>,
    brand: String,
    db_url: Option<String>,
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
            db_url: None,
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

    /// Sets the database URL used by the panel to execute live queries.
    pub fn with_db(mut self, url: impl Into<String>) -> Self {
        self.db_url = Some(url.into());
        self
    }

    /// Builds and returns an Axum Router for the Nexus Panel.
    /// Mount it with `.nest("/nexus", nexus.build())` on your app's router.
    pub fn build(self) -> AxumRouter {
        let state = Arc::new(NexusState {
            registry: Arc::new(self.registry),
            brand: Arc::new(self.brand),
            db_url: Arc::new(self.db_url),
        });

        AxumRouter::new()
            .route("/", get(nexus_dashboard))
            .route("/table/{table}", get(nexus_table_view))
            .route("/table/{table}/search", get(nexus_table_search))
            .route("/table/{table}/new", get(nexus_new_form))
            .route("/table/{table}", post(nexus_create_record))
            .route("/table/{table}/{id}/edit", get(nexus_edit_form))
            .route("/table/{table}/{id}", put(nexus_update_record))
            .route("/table/{table}/{id}", delete(nexus_delete_record))
            .route("/chat", get(nexus_chat_page))
            .route("/chat/query", post(nexus_chat_query))
            .with_state(state)
    }
}

// ─── Query Params ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PaginationParams {
    page: Option<u32>,
    q: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct ChatRequest {
    message: String,
}

// ─── Route Handlers ───────────────────────────────────────────────────────────

/// GET /nexus — Dashboard overview.
async fn nexus_dashboard(State(state): State<Arc<NexusState>>) -> Html<String> {
    let models_sidebar = render_sidebar(&state, None);

    let mut stats_cards = String::new();
    for m in state.registry.iter() {
        let t = m.table;
        let ic = m.icon;
        let lb = m.label;
        stats_cards.push_str(&format!(
            "<a href=\"/nexus/table/{t}\" class=\"nexus-stat-card\" \
             hx-get=\"/nexus/table/{t}\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
             <div class=\"nexus-stat-icon\">{ic}</div>\
             <div class=\"nexus-stat-label\">{lb}</div>\
             <div class=\"nexus-stat-hint\">Click to manage &rarr;</div>\
             </a>"
        ));
    }

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

    Html(render_shell(&state, &models_sidebar, &content))
}

/// GET /nexus/table/{table} — Model list view with pagination.
async fn nexus_table_view(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
    Query(params): Query<PaginationParams>,
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

    let content = render_table_view(&state, entry, page, &q);
    Html(render_shell(
        &state,
        &render_sidebar(&state, Some(&table)),
        &content,
    ))
    .into_response()
}

/// GET /nexus/table/{table}/search — HTMX search fragment (no shell).
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
    Html(render_table_rows(entry, &q))
}

/// GET /nexus/table/{table}/new — New record form.
async fn nexus_new_form(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    Html(render_record_form(entry, None))
}

/// POST /nexus/table/{table} — Create a new record (demo handler).
async fn nexus_create_record(
    State(state): State<Arc<NexusState>>,
    Path(table): Path<String>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    Html(format!(
        "<div class=\"nexus-toast nexus-toast-success\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
         &#9989; New {} record created successfully!\
         </div>",
        entry.label
    ))
}

/// GET /nexus/table/{table}/{id}/edit — Edit record form.
async fn nexus_edit_form(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    Html(render_record_form(entry, Some(&id)))
}

/// PUT /nexus/table/{table}/{id} — Update a record (demo handler).
async fn nexus_update_record(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    Html(format!(
        "<div class=\"nexus-toast nexus-toast-success\" hx-swap-oob=\"true\" id=\"nexus-toast\">\
         &#9989; {} #{} updated successfully!\
         </div>",
        entry.label,
        crate::html::escape_str(&id)
    ))
}

/// DELETE /nexus/table/{table}/{id} — Delete a record (demo handler).
async fn nexus_delete_record(
    State(state): State<Arc<NexusState>>,
    Path((table, id)): Path<(String, String)>,
) -> Html<String> {
    let entry = match find_entry(&state, &table) {
        Some(e) => e,
        None => return Html("<p class=\"nexus-error\">Table not found.</p>".to_string()),
    };
    Html(format!(
        "<tr id=\"row-{id}\" class=\"nexus-row-deleted\">\
         <td colspan=\"99\">\
         <div class=\"nexus-toast nexus-toast-warning\">\
         &#128465;&#65039; {} #{} deleted.\
         </div></td></tr>",
        entry.label,
        crate::html::escape_str(&id)
    ))
}

/// GET /nexus/chat — AI Query Assistant page.
async fn nexus_chat_page(State(state): State<Arc<NexusState>>) -> Html<String> {
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
    content.push_str(&crate::html::escape_str(&schema_summary));
    content.push_str("</pre></div>");
    content.push_str("<div class=\"nexus-chat-panel\">");
    content.push_str("<div class=\"nexus-chat-messages\" id=\"nexus-chat-messages\">");
    content.push_str("<div class=\"nexus-chat-bubble nexus-chat-assistant\">");
    content.push_str("<span class=\"nexus-chat-avatar\">&#129302;</span>");
    content.push_str("<div class=\"nexus-chat-text\">Hello! I know your full database schema. Ask me anything &mdash; for example:<br><em>\"List all users created this week\"</em> or <em>\"How many posts are published?\"</em></div>");
    content.push_str("</div></div>");
    // Form: use &quot; for the getElementById argument to avoid single-quote issues
    content.push_str("<form class=\"nexus-chat-form\" hx-post=\"/nexus/chat/query\" hx-target=\"#nexus-chat-messages\" hx-swap=\"beforeend\" hx-on::after-request=\"this.reset(); document.getElementById(&quot;nexus-chat-messages&quot;).scrollTop = 99999;\">");
    content.push_str("<input type=\"text\" name=\"message\" class=\"nexus-chat-input\" placeholder=\"Ask about your data...\" autocomplete=\"off\" required />");
    content.push_str(
        "<button type=\"submit\" class=\"nexus-btn nexus-btn-ai\">Send &#9992;&#65039;</button>",
    );
    content.push_str("</form></div></div>");

    Html(render_shell(
        &state,
        &render_sidebar(&state, None),
        &content,
    ))
}

/// POST /nexus/chat/query — AI Query HTMX endpoint.
async fn nexus_chat_query(
    State(state): State<Arc<NexusState>>,
    axum::extract::Form(req): axum::extract::Form<ChatRequest>,
) -> Html<String> {
    let user_msg = crate::html::escape_str(&req.message);

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

// ─── Rendering Helpers ────────────────────────────────────────────────────────

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
    }
}

fn field_kind_sql(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Number => "INTEGER",
        FieldKind::Boolean => "INTEGER",
        FieldKind::Date | FieldKind::DateTime => "TEXT",
        FieldKind::Json => "TEXT",
        _ => "TEXT",
    }
}

fn field_kind_input_type(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Email => "email",
        FieldKind::Url => "url",
        FieldKind::Number => "number",
        FieldKind::Password => "password",
        FieldKind::Date => "date",
        FieldKind::DateTime => "datetime-local",
        _ => "text",
    }
}

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
            crate::html::escape_str(message)
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
        out.push_str(&format!(
            "<a href=\"/nexus/table/{t}\" class=\"nexus-nav-link{active_class}\" \
             hx-get=\"/nexus/table/{t}\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
             <span class=\"nexus-nav-icon\">{ic}</span><span>{lb}</span></a>"
        ));
    }
    out.push_str("<div class=\"nexus-nav-divider\"></div>");
    out.push_str(
        "<a href=\"/nexus/chat\" class=\"nexus-nav-link nexus-nav-ai\" \
         hx-get=\"/nexus/chat\" hx-target=\"#nexus-content\" hx-push-url=\"true\">\
         <span class=\"nexus-nav-icon\">&#129302;</span><span>AI Assistant</span></a>",
    );
    out
}

fn render_table_rows(entry: &RegistryEntry, q: &str) -> String {
    let visible_fields: Vec<&FieldMeta> = entry.fields.iter().filter(|f| !f.hidden).collect();

    let sample_rows: Vec<Vec<String>> = (1u32..=5)
        .map(|i| {
            visible_fields
                .iter()
                .map(|f| match f.kind {
                    FieldKind::Email => format!("user{}@example.com", i),
                    FieldKind::Number => i.to_string(),
                    FieldKind::Boolean => {
                        if i % 2 == 0 {
                            "&#9989; Yes".to_string()
                        } else {
                            "&#10060; No".to_string()
                        }
                    }
                    FieldKind::Date => format!("2026-0{}-01", i),
                    _ => format!("Sample {} {}", f.label, i),
                })
                .collect()
        })
        .filter(|row: &Vec<String>| {
            if q.is_empty() {
                return true;
            }
            let q_lower = q.to_lowercase();
            row.iter().any(|c| c.to_lowercase().contains(&q_lower))
        })
        .collect();

    if sample_rows.is_empty() {
        return format!(
            "<tr><td colspan=\"{}\" class=\"nexus-empty-row\">&#128269; No results matching \"{}\"</td></tr>",
            visible_fields.len() + 1,
            crate::html::escape_str(q)
        );
    }

    let t = entry.table;
    let mut out = String::new();
    for (i, row) in sample_rows.iter().enumerate() {
        let id = i + 1;
        let mut cells = String::new();
        for c in row {
            cells.push_str(&format!("<td class=\"nexus-td\">{c}</td>"));
        }
        out.push_str(&format!(
            "<tr id=\"row-{id}\" class=\"nexus-tr\">\
             {cells}\
             <td class=\"nexus-td nexus-td-actions\">\
             <button class=\"nexus-action-btn nexus-action-edit\" \
             hx-get=\"/nexus/table/{t}/{id}/edit\" \
             hx-target=\"#nexus-modal-body\" \
             hx-on::after-request=\"document.getElementById(&quot;nexus-modal&quot;).showModal()\">&#9999;&#65039;</button>\
             <button class=\"nexus-action-btn nexus-action-delete\" \
             hx-delete=\"/nexus/table/{t}/{id}\" \
             hx-target=\"#row-{id}\" \
             hx-confirm=\"Delete this record?\">&#128465;&#65039;</button>\
             </td></tr>"
        ));
    }
    out
}

fn render_table_view(_state: &NexusState, entry: &RegistryEntry, page: u32, q: &str) -> String {
    let visible_fields: Vec<&FieldMeta> = entry.fields.iter().filter(|f| !f.hidden).collect();
    let t = entry.table;
    let lb = entry.label;
    let ic = entry.icon;
    let lb_singular = entry.label.trim_end_matches('s');
    let q_esc = crate::html::escape_str(q);

    let mut headers = String::new();
    for f in &visible_fields {
        headers.push_str(&format!("<th class=\"nexus-th\">{}</th>", f.label));
    }
    let rows = render_table_rows(entry, q);

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
    out.push_str(&format!("<h1 class=\"nexus-page-title\">{ic} {lb}</h1>"));
    out.push_str(&format!(
        "<p class=\"nexus-page-subtitle\">Manage all records in the <code>{t}</code> table.</p>"
    ));
    out.push_str("</div>");
    out.push_str(&format!(
        "<button class=\"nexus-btn nexus-btn-primary\" \
         hx-get=\"/nexus/table/{t}/new\" \
         hx-target=\"#nexus-modal-body\" \
         hx-on::after-request=\"document.getElementById(&quot;nexus-modal&quot;).showModal()\">\
         &#xFF0B; New {lb_singular}</button>"
    ));
    out.push_str("</div>");

    out.push_str("<div class=\"nexus-toolbar\">");
    out.push_str("<div class=\"nexus-search-wrap\">");
    out.push_str("<span class=\"nexus-search-icon\">&#128269;</span>");
    out.push_str(&format!(
        "<input type=\"text\" class=\"nexus-search-input\" \
         placeholder=\"Search {lb}...\" value=\"{q_esc}\" \
         hx-get=\"/nexus/table/{t}/search\" \
         hx-trigger=\"keyup changed delay:300ms\" \
         hx-target=\"#nexus-table-body\" \
         name=\"q\" />"
    ));
    out.push_str("</div>");
    out.push_str(&format!(
        "<span class=\"nexus-page-badge\">Page {page}</span>"
    ));
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

    out.push_str("<div class=\"nexus-pagination\">");
    out.push_str(&prev_btn);
    out.push_str(&format!(
        "<span class=\"nexus-page-indicator\">Page {page}</span>"
    ));
    out.push_str(&next_btn);
    out.push_str("</div>");

    // Modal dialog
    out.push_str("<dialog id=\"nexus-modal\" class=\"nexus-modal\">");
    out.push_str("<div class=\"nexus-modal-inner\">");
    out.push_str("<button class=\"nexus-modal-close\" onclick=\"document.getElementById(&quot;nexus-modal&quot;).close()\">&#x2715;</button>");
    out.push_str("<div id=\"nexus-modal-body\"></div>");
    out.push_str("</div></dialog>");

    // Toast target
    out.push_str("<div id=\"nexus-toast\" class=\"nexus-toast\" aria-live=\"polite\"></div>");

    out
}

fn render_record_form(entry: &RegistryEntry, id: Option<&str>) -> String {
    let is_edit = id.is_some();
    let form_title = if is_edit {
        format!("&#9999;&#65039; Edit {} #{}", entry.label, id.unwrap_or(""))
    } else {
        format!("&#10133; New {}", entry.label.trim_end_matches('s'))
    };

    let method_lower = if is_edit { "put" } else { "post" };
    let action_url = if let Some(id) = id {
        format!("/nexus/table/{}/{}", entry.table, id)
    } else {
        format!("/nexus/table/{}", entry.table)
    };
    let submit_label = if is_edit {
        "&#128190; Save Changes"
    } else {
        "&#9989; Create"
    };

    let mut fields_html = String::new();
    for f in entry.fields.iter().filter(|f| !f.readonly || is_edit) {
        let readonly_attr = if f.readonly { " readonly" } else { "" };
        let disabled_attr = if f.readonly { " disabled" } else { "" };
        let placeholder = format!("Enter {}...", f.label);

        let input = match f.kind {
            FieldKind::Textarea | FieldKind::Json => {
                format!(
                    "<textarea id=\"field-{}\" name=\"{}\" class=\"nexus-input nexus-textarea\"{} placeholder=\"{}\"></textarea>",
                    f.name, f.name, readonly_attr, placeholder
                )
            }
            FieldKind::Boolean => {
                format!(
                    "<label class=\"nexus-toggle\"><input type=\"checkbox\" id=\"field-{}\" name=\"{}\"{} /><span class=\"nexus-toggle-slider\"></span></label>",
                    f.name, f.name, disabled_attr
                )
            }
            _ => {
                let itype = field_kind_input_type(&f.kind);
                format!(
                    "<input type=\"{itype}\" id=\"field-{}\" name=\"{}\" class=\"nexus-input\"{} placeholder=\"{}\" />",
                    f.name, f.name, readonly_attr, placeholder
                )
            }
        };

        let readonly_badge = if f.readonly {
            "<span class=\"nexus-badge-readonly\">readonly</span>"
        } else {
            ""
        };

        fields_html.push_str(&format!(
            "<div class=\"nexus-field-group\">\
             <label class=\"nexus-label\" for=\"field-{}\">{} {readonly_badge}</label>\
             {input}\
             </div>",
            f.name, f.label
        ));
    }

    let mut out = String::new();
    out.push_str(&format!(
        "<form class=\"nexus-form\" hx-{method_lower}=\"{action_url}\" \
         hx-target=\"#nexus-toast\" hx-swap=\"outerHTML\" \
         hx-on::after-request=\"document.getElementById(&quot;nexus-modal&quot;).close()\">"
    ));
    out.push_str(&format!(
        "<h2 class=\"nexus-modal-title\">{form_title}</h2>"
    ));
    out.push_str("<div class=\"nexus-fields-grid\">");
    out.push_str(&fields_html);
    out.push_str("</div>");
    out.push_str("<div class=\"nexus-form-actions\">");
    out.push_str("<button type=\"button\" class=\"nexus-btn nexus-btn-ghost\" onclick=\"document.getElementById(&quot;nexus-modal&quot;).close()\">Cancel</button>");
    out.push_str(&format!(
        "<button type=\"submit\" class=\"nexus-btn nexus-btn-primary\">{submit_label}</button>"
    ));
    out.push_str("</div></form>");
    out
}

// ─── Shell Renderer ───────────────────────────────────────────────────────────

fn render_shell(state: &NexusState, sidebar: &str, content: &str) -> String {
    let brand = crate::html::escape_str(state.brand.as_str());
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\" data-theme=\"dark\">\n<head>\n");
    out.push_str("<meta charset=\"UTF-8\" />\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n");
    out.push_str(&format!("<title>{brand} &mdash; Nexus Panel</title>\n"));
    out.push_str("<meta name=\"description\" content=\"Rullst Nexus: Auto-Generated CMS &amp; AI Admin Panel\" />\n");
    out.push_str("<script src=\"https://unpkg.com/htmx.org@2.0.4\" defer></script>\n");
    out.push_str("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\n");
    out.push_str("<link href=\"https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&amp;family=JetBrains+Mono:wght@400;500&amp;display=swap\" rel=\"stylesheet\">\n");
    out.push_str("<style>\n");
    out.push_str(NEXUS_CSS);
    out.push_str("\n</style>\n</head>\n<body class=\"nexus-body\">\n");

    // Sidebar
    out.push_str("<nav class=\"nexus-sidebar\" id=\"nexus-sidebar\">");
    out.push_str("<div class=\"nexus-brand\">");
    out.push_str("<span class=\"nexus-brand-icon\">&#127963;&#65039;</span>");
    out.push_str(&format!("<span class=\"nexus-brand-name\">{brand}</span>"));
    out.push_str("</div>");
    out.push_str("<div class=\"nexus-nav-label\">MODELS</div>");
    out.push_str(sidebar);
    out.push_str("<div class=\"nexus-sidebar-footer\">");
    out.push_str("<a href=\"/\" class=\"nexus-nav-link nexus-nav-home\"><span class=\"nexus-nav-icon\">&#127968;</span><span>Back to App</span></a>");
    out.push_str("<div class=\"nexus-version\">Rullst Nexus v1.0</div>");
    out.push_str("</div></nav>");

    // Main
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

// ─── Premium Dark-Mode CSS ────────────────────────────────────────────────────

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
.nexus-modal { border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg-800); color: var(--text-100); padding: 0; max-width: 580px; width: 90vw; max-height: 85vh; overflow-y: auto; box-shadow: 0 24px 64px rgba(0,0,0,0.6); }
.nexus-modal::backdrop { background: rgba(0,0,0,0.7); backdrop-filter: blur(4px); }
.nexus-modal-inner { position: relative; padding: 28px; }
.nexus-modal-close { position: absolute; top: 16px; right: 16px; background: var(--bg-700); border: 1px solid var(--border); color: var(--text-300); border-radius: 6px; width: 28px; height: 28px; cursor: pointer; font-size: 13px; display: flex; align-items: center; justify-content: center; transition: all var(--transition); }
.nexus-modal-close:hover { background: var(--red); border-color: var(--red); color: #fff; }
.nexus-modal-title { font-size: 18px; font-weight: 700; margin-bottom: 20px; }

/* == Form ============================================================== */
.nexus-fields-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 24px; }
.nexus-field-group { display: flex; flex-direction: column; gap: 6px; }
.nexus-label { font-size: 12px; font-weight: 600; color: var(--text-300); text-transform: uppercase; letter-spacing: 0.04em; display: flex; align-items: center; gap: 6px; }
.nexus-input { background: var(--bg-700); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-100); font-family: var(--font-sans); font-size: 13.5px; padding: 10px 12px; width: 100%; outline: none; transition: border-color var(--transition), box-shadow var(--transition); }
.nexus-input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-glow); }
.nexus-input[readonly] { opacity: 0.55; cursor: not-allowed; }
.nexus-textarea { min-height: 96px; resize: vertical; font-family: var(--font-mono); font-size: 12px; }
.nexus-badge-readonly { background: var(--bg-500); border: 1px solid var(--border); color: var(--text-500); border-radius: 4px; font-size: 10px; padding: 1px 5px; font-weight: 500; }
.nexus-toggle { display: flex; align-items: center; }
.nexus-form-actions { display: flex; justify-content: flex-end; gap: 10px; border-top: 1px solid var(--border); padding-top: 20px; }
.nexus-error { color: var(--red); font-size: 14px; }

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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestUser;
    impl NexusModel for TestUser {
        fn nexus_table() -> &'static str {
            "users"
        }
        fn nexus_label() -> &'static str {
            "Users"
        }
        fn nexus_icon() -> &'static str {
            "👤"
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
            "📝"
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
        assert_eq!(TestUser::nexus_icon(), "👤");
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
        let nexus = Nexus::new()
            .register::<TestUser>()
            .with_brand("My App")
            .with_db("sqlite://./test.db");
        let _router = nexus.build();
    }

    #[test]
    fn test_field_kind_sql_mapping() {
        assert_eq!(field_kind_sql(&FieldKind::Number), "INTEGER");
        assert_eq!(field_kind_sql(&FieldKind::Text), "TEXT");
        assert_eq!(field_kind_sql(&FieldKind::Boolean), "INTEGER");
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
    }

    #[test]
    fn test_render_table_rows_with_search() {
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "👤",
            pk: "id",
            fields: TestUser::nexus_fields(),
        };
        let rows = render_table_rows(&entry, "example.com");
        assert!(rows.contains("example.com"));
    }

    #[test]
    fn test_render_table_rows_empty_search() {
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "👤",
            pk: "id",
            fields: TestUser::nexus_fields(),
        };
        let rows = render_table_rows(&entry, "zzznomatch99999xyz");
        assert!(rows.contains("No results"));
    }

    #[test]
    fn test_render_record_form_new() {
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "👤",
            pk: "id",
            fields: TestUser::nexus_fields(),
        };
        let form = render_record_form(&entry, None);
        assert!(form.contains("New User"));
        assert!(form.contains("hx-post"));
        assert!(form.contains("Create"));
    }

    #[test]
    fn test_render_record_form_edit() {
        let entry = RegistryEntry {
            table: "users",
            label: "Users",
            icon: "👤",
            pk: "id",
            fields: TestUser::nexus_fields(),
        };
        let form = render_record_form(&entry, Some("42"));
        assert!(form.contains("Edit Users #42"));
        assert!(form.contains("hx-put"));
        assert!(form.contains("Save Changes"));
    }

    #[test]
    fn test_find_entry_found() {
        let state = NexusState {
            registry: Arc::new(vec![RegistryEntry {
                table: "users",
                label: "Users",
                icon: "👤",
                pk: "id",
                fields: TestUser::nexus_fields(),
            }]),
            brand: Arc::new("Test".to_string()),
            db_url: Arc::new(None),
        };
        assert!(find_entry(&state, "users").is_some());
        assert!(find_entry(&state, "missing").is_none());
    }

    #[test]
    fn test_mock_ai_response_list() {
        let resp = generate_mock_ai_response("list all users", "");
        assert!(resp.contains("SELECT"));
    }

    #[test]
    fn test_mock_ai_response_count() {
        let resp = generate_mock_ai_response("how many posts are there?", "");
        assert!(resp.contains("COUNT"));
    }

    #[test]
    fn test_render_sidebar_no_active() {
        let state = NexusState {
            registry: Arc::new(vec![RegistryEntry {
                table: "users",
                label: "Users",
                icon: "👤",
                pk: "id",
                fields: vec![],
            }]),
            brand: Arc::new("Test".to_string()),
            db_url: Arc::new(None),
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
                icon: "👤",
                pk: "id",
                fields: vec![],
            }]),
            brand: Arc::new("Test".to_string()),
            db_url: Arc::new(None),
        };
        let sidebar = render_sidebar(&state, Some("users"));
        assert!(sidebar.contains("nexus-nav-active"));
    }

    #[test]
    fn test_render_shell_contains_brand() {
        let state = NexusState {
            registry: Arc::new(vec![]),
            brand: Arc::new("MySaaS".to_string()),
            db_url: Arc::new(None),
        };
        let html = render_shell(&state, "", "<p>content</p>");
        assert!(html.contains("MySaaS"));
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("nexus-body"));
        assert!(html.contains("<p>content</p>"));
    }
}
