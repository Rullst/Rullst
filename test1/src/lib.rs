use rullst::{html, routes, Router, response::{Html, IntoResponse}};
use rullst::htmx::{HtmxRequest, render_page};

pub mod migrations;


use rullst::db::{Orm, FromRow};

// 1. Define your database model using the built-in rullst-orm ORM!
#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
}


// Main route: uses hybrid SSR with render_page
pub async fn home(htmx: HtmxRequest) -> impl IntoResponse {
    let name = "Rullst";
    // ORM usage example: Fetch active users from database
    let db_status = match User::all().await {
        Ok(users) => format!("Database connected! Total users: {}", users.len()),
        Err(e) => format!("Database offline or not configured: {}", e),
    };

    let content = html! {
        <div class="flex flex-col items-center justify-center min-h-screen bg-slate-950 text-slate-100 p-6 font-sans">
            <div class="max-w-xl text-center space-y-6">
                <h1 class="text-5xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                    "Welcome to " {name}
                </h1>
                
                <p class="text-slate-400 text-lg">
                    "The ultimate full-stack framework for Rust. Focused on Security, Maintainability, and Speed."
                </p>

                <div class="inline-block px-4 py-2 bg-slate-900 border border-slate-800 rounded-lg text-sm text-sky-400 font-mono">
                    {db_status}
                </div>

                <div class="bg-slate-900/50 backdrop-blur-md p-6 rounded-xl border border-slate-800 space-y-4">
                    <h2 class="text-xl font-bold text-slate-200">"HTMX Reactive Counter"</h2>
                    <div id="counter-box" class="flex flex-col items-center gap-3">
                        <button hx-post="/clicked" 
                                hx-target="#counter-box" 
                                hx-swap="outerHTML" 
                                class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                            "Click here to increment"
                        </button>
                        <p class="text-sm text-slate-400">"Clicks received on server: 0"</p>
                    </div>
                </div>
            </div>
        </div>
    };

    render_page(&htmx, "Welcome to Rullst", content)
}

// State for counter
use std::sync::atomic::{AtomicUsize, Ordering};
static CLICK_COUNT: AtomicUsize = AtomicUsize::new(0);

// Reactive HTMX endpoint
pub async fn clicked() -> impl IntoResponse {
    let current_clicks = CLICK_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    
    Html(html! {
        <div id="counter-box" class="flex flex-col items-center gap-3">
            <button hx-post="/clicked" 
                    hx-target="#counter-box" 
                    hx-swap="outerHTML" 
                    class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                "Click here to increment"
            </button>
            <p class="text-sm text-emerald-400 font-medium">"Clicks received on server: " {current_clicks.to_string()}</p>
        </div>
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {
    let router = routes![
        get("/" => home),
        post("/clicked" => clicked),
    ].layer(rullst::server::from_fn(rullst::security::headers_middleware));
    Box::into_raw(Box::new(router))
}
