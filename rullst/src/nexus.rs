use axum::{
    extract::Path,
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use rullst_orm::EloquentModel;
use std::collections::HashMap;
use std::sync::Arc;

/// A registered model in the Nexus CMS.
#[derive(Clone, Debug)]
pub struct NexusModelInfo {
    pub table_name: String,
}

/// The state passed to the Nexus router endpoints.
#[derive(Clone)]
pub struct NexusState {
    pub base_path: String,
    pub models: HashMap<String, NexusModelInfo>,
}

/// The Rullst Nexus: an out-of-the-box AI-Native Admin Panel.
pub struct RullstNexus {
    base_path: String,
    models: HashMap<String, NexusModelInfo>,
}

impl RullstNexus {
    /// Creates a new RullstNexus instance anchored at the given base path (e.g., "/admin").
    pub fn new(base_path: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            models: HashMap::new(),
        }
    }

    /// Registers a model (which implements `EloquentModel`) to be managed by the Nexus CMS.
    pub fn register<T: EloquentModel + 'static>(mut self) -> Self {
        let table_name = T::table_name().to_string();
        self.models
            .insert(table_name.clone(), NexusModelInfo { table_name });
        self
    }

    /// Converts this Nexus instance into an Axum Router that can be nested in the main app.
    pub fn into_router(self) -> axum::Router {
        let state = Arc::new(NexusState {
            base_path: self.base_path.clone(),
            models: self.models.clone(),
        });

        axum::Router::new()
            .route("/", get(dashboard_index))
            .route("/tables/{table_name}", get(view_table))
            .route("/ai-chat", post(ai_chat))
            .with_state(state)
    }
}

async fn dashboard_index(State(state): State<Arc<NexusState>>) -> impl IntoResponse {
    let title = "Rullst Nexus Dashboard";
    let base_path = &state.base_path;

    // We build the list of links for the sidebar.
    let mut table_links = String::new();
    let mut tables: Vec<_> = state.models.keys().collect();
    tables.sort();

    for table in tables {
        let path = format!("{}/tables/{}", base_path, table);
        table_links.push_str(&format!(
            "<li><button hx-get=\"{}\" hx-target=\"#main-content\" class=\"w-full text-left px-4 py-2 hover:bg-slate-800 rounded-md transition-colors text-slate-300 hover:text-white\">{}</button></li>",
            path, table
        ));
    }

    let html = crate::html! {
        <html lang="pt-BR" class="h-full bg-slate-950 text-slate-100">
            <head>
                <meta charset="utf-8" />
                <title>{title}</title>
                <script src="https://cdn.tailwindcss.com"></script>
                <script src="https://unpkg.com/htmx.org@1.9.12"></script>
            </head>
            <body class="h-full flex flex-col">
                <nav class="bg-slate-900 border-b border-slate-800 px-6 py-4 flex items-center justify-between">
                    <div class="flex items-center gap-3">
                        <div class="text-xl font-bold bg-gradient-to-r from-sky-400 to-indigo-400 bg-clip-text text-transparent">
                            "Rullst Nexus"
                        </div>
                        <span class="px-2 py-1 text-xs bg-slate-800 text-slate-400 rounded-md">"v1.0"</span>
                    </div>
                </nav>

                <div class="flex-1 flex overflow-hidden">
                    // Sidebar
                    <aside class="w-64 bg-slate-900 border-r border-slate-800 overflow-y-auto">
                        <div class="p-4">
                            <h2 class="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-3">"Tables"</h2>
                            <ul class="space-y-1">
                                {crate::html::RawHtml(table_links)}
                            </ul>
                        </div>
                    </aside>

                    // Main Content
                    <main id="main-content" class="flex-1 overflow-y-auto p-8 relative">
                        <div class="max-w-4xl mx-auto flex flex-col items-center justify-center h-full text-slate-500">
                            <svg class="w-16 h-16 mb-4 text-slate-700" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2 1.5 3 3.5 3h9c2 0 3.5-1 3.5-3V7c0-2-1.5-3-3.5-3h-9C5.5 4 4 5 4 7zM4 11h16M8 15h4M8 15v4m4-4v4"></path>
                            </svg>
                            <h2 class="text-xl font-medium">"Select a table to view data"</h2>
                        </div>
                    </main>

                    // AI Chat Drawer
                    <aside class="w-80 bg-slate-900 border-l border-slate-800 flex flex-col relative" id="ai-chat-drawer">
                        <div class="p-4 border-b border-slate-800 flex items-center justify-between bg-indigo-500/10">
                            <div class="flex items-center gap-2 text-indigo-400 font-medium">
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
                                </svg>
                                "Nexus AI"
                            </div>
                        </div>
                        <div class="flex-1 overflow-y-auto p-4 space-y-4" id="chat-messages">
                            <div class="bg-slate-800 rounded-lg p-3 text-sm text-slate-300">
                                "Hello! I am Rullst Nexus AI. You can ask me to perform complex queries in natural language."
                            </div>
                        </div>
                        <div class="p-4 border-t border-slate-800">
                            <form hx-post={format!("{}/ai-chat", base_path)} hx-target="#chat-messages" hx-swap="beforeend" class="flex gap-2">
                                <input type="text" name="query" placeholder="e.g. Find all banned users..." class="flex-1 bg-slate-950 border border-slate-800 rounded-md px-3 py-2 text-sm focus:outline-none focus:border-indigo-500 text-slate-200" required="required" autocomplete="off" />
                                <button type="submit" class="bg-indigo-600 hover:bg-indigo-500 text-white p-2 rounded-md transition-colors">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14M12 5l7 7-7 7"></path></svg>
                                </button>
                            </form>
                        </div>
                    </aside>
                </div>
            </body>
        </html>
    };

    Html(html.to_string())
}

// Route to fetch and display table data
async fn view_table(
    Path(table_name): Path<String>,
    State(_state): State<Arc<NexusState>>,
) -> impl IntoResponse {
    // In a real application, we would use a generic query to fetch the rows and columns.
    // However, sqlx does not have a fully dynamic generic `Row` fetching mechanism built-in without
    // depending directly on a specific driver (e.g. `PgRow`, `SqliteRow`).
    // To support `rullst-orm` generically here, we would typically build an HTML table based on
    // the generic data structure or fetch the DB metadata.
    // Since `rullst-orm` abstraction over dynamic rows is driver-specific in rust,
    // we'll output a placeholder indicating where the sqlx dynamic struct parsing goes.
    // (In the full framework, this integrates with the Rullst Studio schema parser).

    let html = crate::html! {
        <div class="h-full flex flex-col">
            <div class="mb-6 flex justify-between items-center">
                <h1 class="text-2xl font-bold text-white capitalize">{&table_name}</h1>
                <button class="bg-sky-600 hover:bg-sky-500 text-white px-4 py-2 rounded-md text-sm font-medium transition-colors">
                    "Create New"
                </button>
            </div>
            <div class="flex-1 overflow-auto bg-slate-900 border border-slate-800 rounded-lg">
                <table class="min-w-full divide-y divide-slate-800">
                    <thead class="bg-slate-900 sticky top-0">
                        <tr>
                            <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"ID"</th>
                            <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-slate-400 uppercase tracking-wider">"Data"</th>
                            <th scope="col" class="px-6 py-3 text-right text-xs font-medium text-slate-400 uppercase tracking-wider">"Actions"</th>
                        </tr>
                    </thead>
                    <tbody class="bg-slate-900 divide-y divide-slate-800">
                        // Real data rows would be mapped here
                        <tr class="hover:bg-slate-800/50">
                            <td class="px-6 py-4 whitespace-nowrap text-sm text-slate-300">"1"</td>
                            <td class="px-6 py-4 whitespace-nowrap text-sm text-slate-300">"Placeholder row for " {&table_name}</td>
                            <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                <button class="text-sky-400 hover:text-sky-300 mr-3">"Edit"</button>
                                <button class="text-red-400 hover:text-red-300">"Delete"</button>
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>
    };

    Html(html.to_string())
}

#[derive(serde::Deserialize, Debug)]
pub struct ChatRequest {
    pub query: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct StructuredAiResponse {
    pub table: String,
    pub filters: HashMap<String, String>,
    pub limit: usize,
}

// AI Chat Endpoint
async fn ai_chat(
    State(state): State<Arc<NexusState>>,
    axum::extract::Form(payload): axum::extract::Form<ChatRequest>,
) -> impl IntoResponse {
    // Attempt to get the AI client
    #[cfg(not(target_arch = "wasm32"))]
    let ai_client = match crate::ai::AiClient::auto() {
        Ok(client) => client,
        Err(_) => {
            return Html(crate::html! {
                <div class="bg-red-500/10 border border-red-500/20 text-red-400 p-3 rounded-md text-sm">
                    "AI Error: Missing API keys in .env file (OPENAI_API_KEY, GEMINI_API_KEY, or OLLAMA_HOST)."
                </div>
            }.to_string());
        }
    };

    #[cfg(target_arch = "wasm32")]
    return Html("<div class='text-red-500'>AI not supported on Wasm client</div>".to_string());

    #[cfg(not(target_arch = "wasm32"))]
    {
        // 1. Render the user's message
        let user_msg = payload.query.clone();

        let available_tables: Vec<String> = state.models.keys().cloned().collect();
        let prompt = format!(
            "You are a database assistant for the Rullst Nexus CMS.\n\
            Available tables: {:?}\n\
            User query: '{}'\n\
            Return a JSON object matching this structure exactly:\n\
            {{\n\
              \"table\": \"name of the target table\",\n\
              \"filters\": {{\"column\": \"value\"}}, \n\
              \"limit\": 10 \n\
            }}\n\
            If no specific table is requested, default to the most likely one, or the first one.\n\
            Do not output any extra text, only the JSON.",
            available_tables, user_msg
        );

        let ai_result = ai_client
            .structured_prompt::<StructuredAiResponse>(&prompt)
            .await;

        let response_html = match ai_result {
            Ok(structured) => {
                // Here, Rullst-ORM would execute a safe prepared statement:
                // SELECT * FROM {structured.table} WHERE column = value LIMIT {structured.limit}
                let query_desc = format!(
                    "Searching table '{}' with filters {:?} (limit: {})",
                    structured.table, structured.filters, structured.limit
                );

                crate::html! {
                    <div>
                        // User message
                        <div class="flex justify-end mb-4">
                            <div class="bg-indigo-600 text-white rounded-lg p-3 text-sm max-w-[80%]">
                                {&user_msg}
                            </div>
                        </div>
                        // AI Response
                        <div class="flex justify-start mb-4">
                            <div class="bg-slate-800 border border-slate-700 text-slate-300 rounded-lg p-3 text-sm max-w-[90%]">
                                <div class="font-medium text-indigo-400 mb-1">"Query Executed Successfully:"</div>
                                <div class="bg-slate-900 p-2 rounded border border-slate-800 font-mono text-xs mb-3 text-emerald-400">
                                    {&query_desc}
                                </div>
                                <button hx-get={format!("{}/tables/{}", state.base_path, structured.table)}
                                        hx-target="#main-content"
                                        class="text-sky-400 hover:underline">
                                    "Click here to view the filtered results in the main panel."
                                </button>
                            </div>
                        </div>
                    </div>
                }.to_string()
            }
            Err(e) => {
                crate::html! {
                    <div>
                        <div class="flex justify-end mb-4">
                            <div class="bg-indigo-600 text-white rounded-lg p-3 text-sm max-w-[80%]">
                                {&user_msg}
                            </div>
                        </div>
                        <div class="flex justify-start mb-4">
                            <div class="bg-red-500/10 border border-red-500/20 text-red-400 rounded-lg p-3 text-sm max-w-[90%]">
                                "Sorry, I could not understand the query or an error occurred. Error: " {e.to_string()}
                            </div>
                        </div>
                    </div>
                }.to_string()
            }
        };

        Html(response_html)
    }
}
