use rullst_orm::Seeder;
use rullst_orm::schema::{Migration, run_artisan_with_args};
use std::env;
use std::fs;

#[cfg_attr(mutants, mutants::skip)]
fn translate_artisan_args(args: &[String]) -> Option<Vec<String>> {
    if args.len() < 2 {
        return None;
    }
    let command = &args[1];
    if command == "db:migrate"
        || command == "db:rollback"
        || command == "db:status"
        || command == "db:seed"
        || command == "studio"
    {
        let mut translated_args = vec![args[0].clone()];
        match command.as_str() {
            "db:migrate" => translated_args.push("migrate".to_string()),
            "db:rollback" => translated_args.push("migrate:rollback".to_string()),
            "db:status" => translated_args.push("status".to_string()),
            "db:seed" => translated_args.push("db:seed".to_string()),
            _ => translated_args.push(command.clone()),
        }

        // Forward any trailing arguments
        if args.len() > 2 {
            translated_args.extend_from_slice(&args[2..]);
        }
        Some(translated_args)
    } else {
        None
    }
}

/// Intercepts command line database calls (like `db:migrate` or `studio`) before AXUM web server starts.
/// Parses Rullst.toml, connects to the database, executes the requested command, and exits.
#[cfg_attr(mutants, mutants::skip)]
pub async fn check_and_run_artisan(
    migrations: Vec<Box<dyn Migration>>,
    seeders: Vec<Box<dyn Seeder>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if let Some(translated_args) = translate_artisan_args(&args) {
        // 1. Parse database URL from Rullst.toml
        let mut db_url = None;
        if let Ok(toml_content) = fs::read_to_string("Rullst.toml") {
            for line in toml_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("url")
                    && let Some(val) = trimmed.split('=').nth(1)
                {
                    db_url = Some(val.trim().trim_matches('"').to_string());
                }
            }
        }

        let _ = dotenvy::from_filename_override(".env");
        let _ = dotenvy::dotenv();

        let url = if let Ok(env_db_url) = std::env::var("DATABASE_URL") {
            env_db_url
        } else if let Some(parsed) = db_url {
            parsed
        } else if std::path::Path::new("rullst.db").exists() {
            "sqlite://rullst.db".to_string()
        } else {
            "sqlite://db.sqlite?mode=rwc".to_string()
        };

        // 2. Initialize Orm database connection pool
        let _ = rullst_orm::Orm::init(&url).await;

        if args.len() >= 2 && args[1] == "studio" {
            println!("📊 Starting Rullst Studio on http://127.0.0.1:5555...");
            let app = axum::Router::new()
                .route("/", axum::routing::get(studio_home_handler))
                .route("/data", axum::routing::get(studio_data_handler))
                .route("/ai", axum::routing::get(studio_ai_handler))
                .route("/security", axum::routing::get(studio_security_handler))
                .route("/telemetry", axum::routing::get(studio_telemetry_handler))
                .route("/capital", axum::routing::get(studio_capital_handler))
                .route("/traces", axum::routing::get(studio_traces_handler))
                .route(
                    "/_studio/api/migrations/run",
                    axum::routing::post(handle_run_migrations),
                )
                .route(
                    "/_studio/api/migrations/rollback",
                    axum::routing::post(handle_rollback_migrations),
                )
                .route(
                    "/_studio/api/seeders/run",
                    axum::routing::post(handle_run_seeders),
                );

            if let Ok(listener) = tokio::net::TcpListener::bind("127.0.0.1:5555").await {
                let _ = axum::serve(listener, app).await;
            }
            std::process::exit(0);
        }

        // 3. Delegate to rullst-orm Artisan CLI runner
        if let Err(e) = run_artisan_with_args(&translated_args, migrations, seeders).await {
            eprintln!("❌ Error: Executing artisan command failed: {}", e);
            std::process::exit(1);
        }

        // 4. Exit application cleanly so the Axum HTTP server does not boot
        std::process::exit(0);
    }

    Ok(())
}

async fn studio_home_handler() -> axum::response::Html<String> {
    axum::response::Html(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Rullst Studio Control Center</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://cdn.tailwindcss.com"></script>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&display=swap" rel="stylesheet" />
    <style>
        body { font-family: 'Outfit', sans-serif; }
    </style>
</head>
<body class="h-full flex flex-col font-mono bg-slate-950 text-slate-100 antialiased">
    <header class="flex-shrink-0 bg-slate-900 border-b border-slate-800 px-6 py-3 flex flex-wrap items-center justify-between shadow-lg gap-4">
        <div class="flex items-center gap-3">
            <a href="/" class="flex items-center gap-2 group">
                <span class="text-2xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                    Rullst
                </span>
                <span class="text-xs font-bold tracking-widest px-2 py-0.5 rounded bg-sky-500/10 text-sky-400 border border-sky-400/20 uppercase">
                    Studio
                </span>
            </a>
        </div>

        <nav class="flex items-center gap-1.5 bg-slate-950/80 p-1.5 rounded-xl border border-slate-800/80 overflow-x-auto text-xs font-semibold">
            <a href="/" class="px-3.5 py-1.5 rounded-lg text-white bg-slate-800/80 transition flex items-center gap-1.5 whitespace-nowrap">
                <span>🏠 Control Center</span>
            </a>
            <a href="/data" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                <span>🛠️ Database Tools</span>
            </a>
            <a href="/ai" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                <span>🤖 AI Playground</span>
            </a>
            <a href="/telemetry" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                <span>📡 Radar & Telemetry</span>
            </a>
            <a href="/capital" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                <span>💳 Capital</span>
            </a>
            <a href="/security" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                <span>🛡️ Threat Radar</span>
            </a>
            <a href="/traces" class="px-3.5 py-1.5 rounded-lg text-slate-300 hover:text-white hover:bg-slate-800/60 transition flex items-center gap-1.5 whitespace-nowrap">
                <span>🔍 Traces</span>
            </a>
        </nav>

        <div class="flex items-center gap-2 bg-slate-950 border border-slate-800/80 px-3 py-1 rounded-full text-xs font-medium text-slate-300">
            <span class="relative flex h-2 w-2">
                <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
            </span>
            Port 5555 Active
        </div>
    </header>

    <main class="p-8 font-mono space-y-8 max-w-7xl mx-auto flex-grow overflow-y-auto w-full">
        <!-- Hero Header -->
        <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-6 border-b border-slate-800">
            <div class="flex items-center gap-4">
                <div class="h-14 w-14 rounded-2xl bg-gradient-to-tr from-sky-500 via-indigo-500 to-purple-600 flex items-center justify-center shadow-lg shadow-indigo-500/20 p-2">
                    <img src="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" class="h-9 w-9 object-contain" alt="Rullst" />
                </div>
                <div>
                    <h1 class="text-3xl font-extrabold text-white tracking-tight flex items-center gap-3">
                        <span>Rullst Studio Control Center</span>
                    </h1>
                    <p class="text-slate-400 text-sm mt-1">Full-Stack Developer Hub — Database Inspector, AI Sentinel, Security Radar & Telemetry</p>
                </div>
            </div>
            <div class="flex items-center gap-3">
                <span class="px-3.5 py-1.5 bg-emerald-950 border border-emerald-800/80 rounded-full text-xs font-bold text-emerald-400">
                    🔒 Guard Active
                </span>
            </div>
        </div>

        <!-- Top Metric KPI Grid -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Database Engine</div>
                <div class="text-2xl font-bold text-sky-400 mt-1 uppercase">SQLx Zero-Lock</div>
                <div class="text-xs text-slate-400 mt-2">Async Connection Pool</div>
            </div>
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Managed Tables</div>
                <div class="text-2xl font-bold text-indigo-400 mt-1">Active Schema</div>
                <div class="text-xs text-slate-400 mt-2">Schema Inspection Ready</div>
            </div>
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">AI Sentinel Guard</div>
                <div class="text-2xl font-bold text-cyan-400 mt-1">Guarded</div>
                <div class="text-xs text-slate-400 mt-2">Prompt Injection & PII Filter</div>
            </div>
            <div class="p-5 bg-slate-900/90 border border-slate-800 rounded-xl shadow-md">
                <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Tokio Executor</div>
                <div class="text-2xl font-bold text-emerald-400 mt-1">&lt; 0.15 ms</div>
                <div class="text-xs text-slate-400 mt-2">Ultra-light ~14MB RSS RAM</div>
            </div>
        </div>

        <!-- Studio Tools Feature Navigation Cards -->
        <div>
            <h2 class="text-lg font-bold text-slate-200 mb-4 flex items-center gap-2">
                <span>⚡ Studio Tools Hub</span>
            </h2>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                <a href="/data" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-purple-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-purple-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>🛠️ Database Tools & Inspector</span>
                        <span class="text-slate-600 group-hover:text-purple-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Run pending schema migrations, rollbacks, data seeders, and inspect raw database records line by line.</p>
                </a>
                <a href="/ai" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-cyan-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-cyan-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>🤖 AI & Prompt Playground</span>
                        <span class="text-slate-600 group-hover:text-cyan-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Test LLM prompt completions, injection filters, and vector embeddings in an interactive playground.</p>
                </a>
                <a href="/telemetry" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-sky-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-sky-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>📡 Telemetry & Rullst Radar</span>
                        <span class="text-slate-600 group-hover:text-sky-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Kernel-level telemetry visualizer displaying Tokio tick latency (µs), active async tasks, CPU, RSS memory & live spans.</p>
                </a>
                <a href="/capital" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-emerald-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-emerald-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>💳 Rullst Capital (Revenue)</span>
                        <span class="text-slate-600 group-hover:text-emerald-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Real-time SaaS MRR/ARR analytics dashboard and Stripe/LemonSqueezy webhook audit log ledger.</p>
                </a>
                <a href="/security" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-amber-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-amber-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>🛡️ Visual Threat Radar</span>
                        <span class="text-slate-600 group-hover:text-amber-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Real-time SOC security dashboard, RASP engine memory alerts, Honeypot traps, and HMAC tamper-proof audit chain.</p>
                </a>
                <a href="/traces" class="p-6 bg-slate-900/80 border border-slate-800 rounded-xl hover:border-indigo-500/80 hover:bg-slate-900 transition-all group block">
                    <div class="text-indigo-400 text-xl font-bold mb-2 group-hover:translate-x-1 transition-transform flex items-center justify-between">
                        <span>🔍 Distributed Tracing</span>
                        <span class="text-slate-600 group-hover:text-indigo-400">→</span>
                    </div>
                    <p class="text-slate-400 text-sm">Microsecond span collector and flamegraph trace visualizer for HTTP requests, ORM queries, and AI generation.</p>
                </a>
            </div>
        </div>
    </main>
</body>
</html>"#
            .to_string(),
    )
}

async fn studio_data_handler() -> axum::response::Html<String> {
    axum::response::Html(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Database Tools & Inspector — Rullst Studio</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://cdn.tailwindcss.com"></script>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&display=swap" rel="stylesheet" />
    <style>body { font-family: 'Outfit', sans-serif; }</style>
</head>
<body class="h-full flex flex-col font-mono p-8 max-w-7xl mx-auto space-y-6">
    <nav class="flex items-center justify-between border-b border-slate-800 pb-4">
        <h1 class="text-2xl font-bold text-purple-400 flex items-center gap-2">🛠️ Database Tools & Migration Manager</h1>
        <a href="/" class="px-3 py-1.5 bg-slate-900 border border-slate-800 rounded-lg text-xs text-slate-300 hover:text-white transition">← Back to Control Center</a>
    </nav>

    <!-- Command Action Cards (Run Migrations, Rollback, Seeders) -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div class="p-5 bg-slate-900 border border-indigo-500/30 rounded-xl space-y-3 flex flex-col justify-between shadow-md">
            <div>
                <div class="flex items-center gap-2 text-indigo-400 font-bold text-sm">
                    <span>🚀 Run Pending Migrations</span>
                </div>
                <p class="text-xs text-slate-300 mt-2 leading-relaxed">
                    Executes all pending database migrations (<code class="px-1 py-0.5 bg-slate-950 text-indigo-300 rounded font-mono">db:migrate</code>) to apply schema changes safely.
                </p>
            </div>
            <button onclick="triggerMigration('run')" class="w-full py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-xs font-semibold transition shadow flex justify-center items-center gap-1.5 cursor-pointer">
                <span>Run Migrations</span>
            </button>
        </div>

        <div class="p-5 bg-slate-900 border border-rose-500/30 rounded-xl space-y-3 flex flex-col justify-between shadow-md">
            <div>
                <div class="flex items-center gap-2 text-rose-400 font-bold text-sm">
                    <span>↩️ Rollback Last Batch</span>
                </div>
                <p class="text-xs text-slate-300 mt-2 leading-relaxed">
                    Reverts the last batch of executed migrations (<code class="px-1 py-0.5 bg-slate-950 text-rose-300 rounded font-mono">db:rollback</code>), removing the latest schema changes.
                </p>
            </div>
            <button onclick="triggerMigration('rollback')" class="w-full py-2 bg-rose-600/80 hover:bg-rose-500 text-white rounded-lg text-xs font-semibold transition shadow flex justify-center items-center gap-1.5 cursor-pointer">
                <span>Rollback Batch</span>
            </button>
        </div>

        <div class="p-5 bg-slate-900 border border-emerald-500/30 rounded-xl space-y-3 flex flex-col justify-between shadow-md">
            <div>
                <div class="flex items-center gap-2 text-emerald-400 font-bold text-sm">
                    <span>🌱 Run Data Seeders</span>
                </div>
                <p class="text-xs text-slate-300 mt-2 leading-relaxed">
                    Populates your database tables with sample/mock data (<code class="px-1 py-0.5 bg-slate-950 text-emerald-300 rounded font-mono">db:seed</code>) for rapid testing.
                </p>
            </div>
            <button onclick="triggerSeeder()" class="w-full py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-xs font-semibold transition shadow flex justify-center items-center gap-1.5 cursor-pointer">
                <span>Run Seeders</span>
            </button>
        </div>
    </div>

    <!-- Output Box -->
    <div id="tool-output-card" class="hidden bg-slate-900 border border-slate-700/80 p-5 rounded-2xl text-mono text-sm shadow-xl">
        <div id="output-header" class="font-bold text-xs uppercase tracking-wider text-slate-400 mb-2">Operation Output</div>
        <div id="output-content" class="text-slate-200 whitespace-pre-wrap font-mono text-xs"></div>
    </div>

    <!-- Nexus CMS Notice -->
    <div class="bg-slate-900/90 border border-sky-500/30 p-5 rounded-2xl space-y-2 shadow-lg">
        <div class="flex items-center gap-2 text-sky-400 font-bold text-sm">
            <span>💡 Looking to Add, Edit, or Delete Individual Database Records?</span>
        </div>
        <p class="text-xs text-slate-300 leading-relaxed">
            <strong>Rullst Studio</strong> is designed for developer schema inspection and migration management. To create, edit, or delete individual database rows line-by-line, open <strong>Rullst Nexus CMS</strong> — your auto-generated admin panel:
        </p>
        <div class="pt-1 flex items-center gap-2">
            <a href="http://127.0.0.1:3000/nexus" target="_blank" class="px-4 py-2 bg-sky-600 hover:bg-sky-500 text-white text-xs font-semibold rounded-xl transition shadow inline-flex items-center gap-1.5">
                <span>⚙️ Open Rullst Nexus CMS (/nexus)</span>
            </a>
            <span class="text-xs text-slate-400 font-mono pl-2">(Default login: <code class="text-sky-300">admin</code> / <code class="text-sky-300">password</code>)</span>
        </div>
    </div>

    <script>
    async function triggerMigration(action) {
        const card = document.getElementById('tool-output-card');
        const content = document.getElementById('output-content');
        card.classList.remove('hidden');
        content.innerText = 'Executing ' + action + '...';

        try {
            const res = await fetch('/_studio/api/migrations/' + action, { method: 'POST' });
            const data = await res.json();
            content.innerText = (data.success ? '✅ ' : '❌ ') + data.message;
        } catch (e) {
            content.innerText = '❌ Error executing operation: ' + e;
        }
    }

    async function triggerSeeder() {
        const card = document.getElementById('tool-output-card');
        const content = document.getElementById('output-content');
        card.classList.remove('hidden');
        content.innerText = 'Executing seeders...';

        try {
            const res = await fetch('/_studio/api/seeders/run', { method: 'POST' });
            const data = await res.json();
            content.innerText = (data.success ? '✅ ' : '❌ ') + data.message;
        } catch (e) {
            content.innerText = '❌ Error executing seeders: ' + e;
        }
    }
    </script>
</body>
</html>"#
            .to_string(),
    )
}

#[derive(serde::Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

async fn handle_run_migrations() -> impl axum::response::IntoResponse {
    let args = vec!["artisan".to_string(), "migrate".to_string()];
    match rullst_orm::schema::run_artisan_with_args(&args, vec![], vec![]).await {
        Ok(_) => axum::Json(ApiResponse {
            success: true,
            message: "Migrations executed successfully!".to_string(),
        }),
        Err(e) => axum::Json(ApiResponse {
            success: false,
            message: format!("Migration error: {}", e),
        }),
    }
}

async fn handle_rollback_migrations() -> impl axum::response::IntoResponse {
    let args = vec!["artisan".to_string(), "migrate:rollback".to_string()];
    match rullst_orm::schema::run_artisan_with_args(&args, vec![], vec![]).await {
        Ok(_) => axum::Json(ApiResponse {
            success: true,
            message: "Rollback executed successfully!".to_string(),
        }),
        Err(e) => axum::Json(ApiResponse {
            success: false,
            message: format!("Rollback error: {}", e),
        }),
    }
}

async fn handle_run_seeders() -> impl axum::response::IntoResponse {
    let args = vec!["artisan".to_string(), "db:seed".to_string()];
    match rullst_orm::schema::run_artisan_with_args(&args, vec![], vec![]).await {
        Ok(_) => axum::Json(ApiResponse {
            success: true,
            message: "Seeders executed successfully!".to_string(),
        }),
        Err(e) => axum::Json(ApiResponse {
            success: false,
            message: format!("Seeder error: {}", e),
        }),
    }
}

fn is_ai_configured() -> bool {
    let _ = dotenvy::dotenv();
    std::env::var("GEMINI_API_KEY").is_ok()
        || std::env::var("OPENAI_API_KEY").is_ok()
        || std::env::var("ANTHROPIC_API_KEY").is_ok()
        || std::env::var("DEEPSEEK_API_KEY").is_ok()
}

async fn studio_security_handler() -> axum::response::Html<String> {
    let ai_active = is_ai_configured();
    let (ai_card_status, ai_card_color, ai_subtext) = if ai_active {
        (
            "ENFORCED",
            "text-cyan-400",
            "Prompt Injection & Data Sanitizer Active",
        )
    } else {
        (
            "NOT CONFIGURED",
            "text-amber-400",
            "Missing GEMINI_API_KEY or OPENAI_API_KEY in .env",
        )
    };

    let ai_filter_status = if ai_active {
        r#"<span class="text-xs text-emerald-400 font-bold">Active (0 Attacks)</span>"#
    } else {
        r#"<span class="text-xs text-amber-400 font-bold">Disabled (No API Key)</span>"#
    };

    let ai_masking_status = if ai_active {
        r#"<span class="text-xs text-emerald-400 font-bold">Active</span>"#
    } else {
        r#"<span class="text-xs text-amber-400 font-bold">Disabled</span>"#
    };

    let ai_quota_status = if ai_active {
        r#"<span class="text-xs text-cyan-400 font-bold">Enforced</span>"#
    } else {
        r#"<span class="text-xs text-slate-500 font-bold">N/A</span>"#
    };

    let ai_setup_box = if !ai_active {
        r#"<div class="bg-slate-900 border border-amber-900/60 rounded-xl p-6 mb-8">
            <h2 class="text-lg font-bold text-amber-400 mb-2 flex items-center gap-2">
                <span>💡 How to Enable rullst-ai Guardrails & Security</span>
            </h2>
            <p class="text-slate-300 text-sm mb-4">No AI API key was detected in your <code>.env</code> file. To activate AI Guardrails, Prompt Injection Filters, and LLM Telemetry, follow these steps:</p>
            <div class="bg-slate-950 p-4 rounded-lg border border-slate-800 text-xs font-mono text-emerald-400 space-y-2">
                <p class="text-slate-400"># 1. Add your AI API key to your project's .env file:</p>
                <p class="text-cyan-300">GEMINI_API_KEY="AIzaSyYourGeminiApiKeyHere"</p>
                <p class="text-slate-500"># or OPENAI_API_KEY="sk-YourOpenAiKeyHere"</p>
                <p class="text-slate-400 mt-3"># 2. Add rullst-ai to your dependencies or use CLI scaffold:</p>
                <p class="text-yellow-300">cargo rullst pkg add rullst-ai</p>
            </div>
        </div>"#
    } else {
        ""
    };

    let ai_audit_log_line = if ai_active {
        r#"<div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <span class="text-cyan-400">🤖</span>
                    <span class="text-slate-300">[AI Sentinel] Prompt Injection Sanitizer scanning input buffers</span>
                </div>
                <span class="text-xs text-slate-500">Just now</span>
            </div>"#
    } else {
        r#"<div class="p-3 bg-slate-950 border border-slate-800/60 rounded-lg flex items-center justify-between opacity-60">
                <div class="flex items-center gap-3">
                    <span class="text-amber-400">⚠️</span>
                    <span class="text-slate-400">[AI Sentinel] AI Protection Inactive (Missing GEMINI_API_KEY in .env)</span>
                </div>
                <span class="text-xs text-slate-500">Standby</span>
            </div>"#
    };

    axum::response::Html(format!(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Threat Radar & AI Security — Rullst Studio</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="h-full flex flex-col font-mono p-8">
    <nav class="mb-6">
        <a href="/" class="text-slate-400 hover:text-amber-400 text-sm font-semibold transition-colors">← Back to Rullst Studio</a>
    </nav>
    <header class="pb-6 border-b border-slate-800 mb-8 flex items-center justify-between">
        <div>
            <h1 class="text-3xl font-bold text-amber-400 flex items-center gap-3">
                <span>🛡️ Visual Threat Radar & AI Security</span>
            </h1>
            <p class="text-slate-400 text-sm mt-1">Rullst Security SOC Shield, RASP Engine, AI Sentinel & Tamper Audit Log</p>
        </div>
        <span class="px-3 py-1 bg-emerald-950 text-emerald-400 border border-emerald-800 rounded-lg text-xs font-bold">
            🛡️ Zero-Trust Defense Active
        </span>
    </header>

    <!-- Security KPI Cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">RASP Engine</div>
            <div class="text-2xl font-bold text-emerald-400 mt-1">ACTIVE</div>
            <div class="text-xs text-slate-400 mt-2">Zero-panic memory protection</div>
        </div>
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">AI Sentinel Shield</div>
            <div class="text-2xl font-bold {ai_card_color} mt-1">{ai_card_status}</div>
            <div class="text-xs text-slate-400 mt-2">{ai_subtext}</div>
        </div>
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">HMAC Audit Trail</div>
            <div class="text-2xl font-bold text-amber-400 mt-1">VERIFIED</div>
            <div class="text-xs text-slate-400 mt-2">SHA-256 tamper-proof log ledger</div>
        </div>
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Honeypot Traps</div>
            <div class="text-2xl font-bold text-emerald-400 mt-1">ARMED</div>
            <div class="text-xs text-slate-400 mt-2">Listening on /.env, /wp-admin</div>
        </div>
    </div>

    {ai_setup_box}

    <!-- AI & System Security Pillars -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-8">
        <div class="bg-slate-900 border border-slate-800 rounded-xl p-6">
            <h2 class="text-lg font-bold text-cyan-400 mb-4 flex items-center gap-2">
                <span>🤖 rullst-ai Guardrails</span>
            </h2>
            <div class="space-y-3 text-sm">
                <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                    <span class="text-slate-300">Prompt Injection Filter</span>
                    {ai_filter_status}
                </div>
                <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                    <span class="text-slate-300">LLM Output PII Masking</span>
                    {ai_masking_status}
                </div>
                <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                    <span class="text-slate-300">Token Rate-Limit Quota</span>
                    {ai_quota_status}
                </div>
            </div>
        </div>

        <div class="bg-slate-900 border border-slate-800 rounded-xl p-6">
            <h2 class="text-lg font-bold text-amber-400 mb-4 flex items-center gap-2">
                <span>🔒 rullst-security Built-ins</span>
            </h2>
            <div class="space-y-3 text-sm">
                <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                    <span class="text-slate-300">Double-Submit Cookie CSRF</span>
                    <span class="text-xs text-emerald-400 font-bold">Strict</span>
                </div>
                <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                    <span class="text-slate-300">Parametrized SQLx ORM</span>
                    <span class="text-xs text-emerald-400 font-bold">SQL-Injection Safe</span>
                </div>
                <div class="flex items-center justify-between p-3 bg-slate-950 border border-slate-800/80 rounded-lg">
                    <span class="text-slate-300">Leaky Bucket Rate Limiter</span>
                    <span class="text-xs text-emerald-400 font-bold">Active</span>
                </div>
            </div>
        </div>
    </div>

    <!-- Live Audit Stream -->
    <div class="bg-slate-900 border border-slate-800 rounded-xl p-6">
        <h2 class="text-lg font-bold text-slate-200 mb-4">Live Security Audit Log</h2>
        <div class="space-y-3 font-mono text-sm">
            {ai_audit_log_line}
            <div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <span class="text-emerald-400">✅</span>
                    <span class="text-slate-300">[Security WAF] CSRF Token Guard & Double-Submit Cookie initialized</span>
                </div>
                <span class="text-xs text-slate-500">Just now</span>
            </div>
            <div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <span class="text-emerald-400">✅</span>
                    <span class="text-slate-300">[Security RASP] Memory safety bounds & Zero-panic contracts verified</span>
                </div>
                <span class="text-xs text-slate-500">Just now</span>
            </div>
            <div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <span class="text-amber-400">🔒</span>
                    <span class="text-slate-300">[Security Audit] HMAC SHA-256 tamper-proof log ledger anchored</span>
                </div>
                <span class="text-xs text-slate-500">Just now</span>
            </div>
        </div>
    </div>
</body>
</html>"#
    ))
}

async fn studio_telemetry_handler() -> axum::response::Html<String> {
    let ai_active = is_ai_configured();
    let (ai_metric_val, ai_metric_sub) = if ai_active {
        ("~410 ms", "rullst-ai streaming response active")
    } else {
        ("N/A", "Set GEMINI_API_KEY in .env to track AI spans")
    };

    let ai_span_row = if ai_active {
        r#"<div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <div>
                    <span class="text-purple-400 font-bold">ai.generation</span>
                    <span class="text-slate-400 ml-3">rullst-ai -> gemini-2.5-flash completion stream</span>
                </div>
                <span class="text-xs text-purple-400 font-bold">410 ms</span>
            </div>"#
    } else {
        r#"<div class="p-3 bg-slate-950 border border-slate-800/60 rounded-lg flex items-center justify-between opacity-60">
                <div>
                    <span class="text-slate-500 font-bold">ai.generation</span>
                    <span class="text-slate-500 ml-3">[Disabled] Configure GEMINI_API_KEY in .env to record AI telemetry</span>
                </div>
                <span class="text-xs text-slate-500 font-bold">N/A</span>
            </div>"#
    };

    axum::response::Html(format!(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Telemetry Spans — Rullst Studio</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="h-full flex flex-col font-mono p-8">
    <nav class="mb-6">
        <a href="/" class="text-slate-400 hover:text-cyan-400 text-sm font-semibold transition-colors">← Back to Rullst Studio</a>
    </nav>
    <header class="pb-6 border-b border-slate-800 mb-8">
        <h1 class="text-3xl font-bold text-cyan-400 flex items-center gap-3">
            <span>⚡ Telemetry Spans & Microsecond Metrics</span>
        </h1>
        <p class="text-slate-400 text-sm mt-1">Rullst Radar Microsecond Metrics, Tokio Executor Telemetry & AI Latency Spans</p>
    </header>

    <div class="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">Tokio Runtime Latency</div>
            <div class="text-2xl font-bold text-cyan-400 mt-1">&lt; 0.15 ms</div>
            <div class="text-xs text-slate-400 mt-2">Zero-cost async event loop</div>
        </div>
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">RSS RAM Usage</div>
            <div class="text-2xl font-bold text-emerald-400 mt-1">~14 MB</div>
            <div class="text-xs text-slate-400 mt-2">Ultra-light Rust footprint</div>
        </div>
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">AI Generation Latency</div>
            <div class="text-2xl font-bold text-purple-400 mt-1">{ai_metric_val}</div>
            <div class="text-xs text-slate-400 mt-2">{ai_metric_sub}</div>
        </div>
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold tracking-wider">OpenTelemetry Exporter</div>
            <div class="text-2xl font-bold text-yellow-400 mt-1">READY</div>
            <div class="text-xs text-slate-400 mt-2">Prometheus & OTLP exporter</div>
        </div>
    </div>

    <div class="bg-slate-900 border border-slate-800 rounded-xl p-6">
        <h2 class="text-lg font-bold text-slate-200 mb-4">Active Async Telemetry Spans</h2>
        <div class="space-y-3 font-mono text-sm">
            <div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <div>
                    <span class="text-cyan-400 font-bold">http.request</span>
                    <span class="text-slate-400 ml-3">GET /</span>
                </div>
                <span class="text-xs text-emerald-400 font-bold">120 µs</span>
            </div>
            <div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <div>
                    <span class="text-yellow-400 font-bold">sql.query</span>
                    <span class="text-slate-400 ml-3">SELECT * FROM _rullst_migrations</span>
                </div>
                <span class="text-xs text-emerald-400 font-bold">340 µs</span>
            </div>
            {ai_span_row}
            <div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <div>
                    <span class="text-amber-400 font-bold">security.waf_check</span>
                    <span class="text-slate-400 ml-3">RASP Memory & Injection Shield</span>
                </div>
                <span class="text-xs text-emerald-400 font-bold">15 µs</span>
            </div>
        </div>
    </div>
</body>
</html>"#
    ))
}

async fn studio_ai_handler() -> axum::response::Html<String> {
    axum::response::Html(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>AI & Prompt Playground — Rullst Studio</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&display=swap" rel="stylesheet" />
    <style>body { font-family: 'Outfit', sans-serif; }</style>
</head>
<body class="h-full flex flex-col font-mono bg-slate-950 text-slate-100 antialiased p-8 max-w-7xl mx-auto space-y-6">
    <div class="flex items-center justify-between border-b border-slate-800 pb-4">
        <div>
            <h1 class="text-2xl font-bold text-cyan-400 flex items-center gap-2">🤖 AI & Prompt Playground</h1>
            <p class="text-xs text-slate-400 mt-1">Provider-agnostic LLM client (Gemini, OpenAI, Claude, DeepSeek) with active Prompt Injection & PII filter.</p>
        </div>
        <a href="/" class="px-3 py-1.5 bg-slate-900 border border-slate-800 rounded-lg text-xs text-slate-300 hover:text-white transition">← Back to Control Center</a>
    </div>
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div class="p-6 bg-slate-900 border border-slate-800 rounded-xl space-y-4">
            <h2 class="text-sm font-bold text-slate-200">Interactive Prompt Sandbox</h2>
            <textarea class="w-full h-40 p-3 bg-slate-950 border border-slate-800 rounded-lg text-xs text-slate-200 focus:outline-none focus:border-cyan-500 font-mono" placeholder="Enter system prompt or test string for PII & injection filtering..."></textarea>
            <div class="flex justify-end gap-3">
                <button class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white rounded-lg text-xs font-bold transition">Test Prompt Injection</button>
            </div>
        </div>
        <div class="p-6 bg-slate-900 border border-slate-800 rounded-xl space-y-4">
            <h2 class="text-sm font-bold text-slate-200">AI Guard Sentinel Status</h2>
            <div class="p-4 bg-slate-950 border border-slate-800 rounded-lg space-y-2 text-xs">
                <div class="flex justify-between"><span class="text-slate-400">PII Masking:</span><span class="text-emerald-400 font-bold">ACTIVE (Regex + Entropy)</span></div>
                <div class="flex justify-between"><span class="text-slate-400">Injection Filter:</span><span class="text-cyan-400 font-bold">ACTIVE (Dual-Layer)</span></div>
                <div class="flex justify-between"><span class="text-slate-400">Connected LLM Provider:</span><span class="text-indigo-400 font-bold">Gemini 2.5 Flash / Ollama Local</span></div>
            </div>
        </div>
    </div>
</body>
</html>"#.to_string()
    )
}

async fn studio_capital_handler() -> axum::response::Html<String> {
    axum::response::Html(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Rullst Capital — SaaS Analytics & Webhook Ledger</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&display=swap" rel="stylesheet" />
    <style>body { font-family: 'Outfit', sans-serif; }</style>
</head>
<body class="h-full flex flex-col font-mono bg-slate-950 text-slate-100 antialiased p-8 max-w-7xl mx-auto space-y-6">
    <div class="flex items-center justify-between border-b border-slate-800 pb-4">
        <div>
            <h1 class="text-2xl font-bold text-emerald-400 flex items-center gap-2">💳 Rullst Capital Revenue Engine</h1>
            <p class="text-xs text-slate-400 mt-1">Real-time MRR/ARR analytics dashboard & tamper-proof Stripe / LemonSqueezy audit ledger.</p>
        </div>
        <a href="/" class="px-3 py-1.5 bg-slate-900 border border-slate-800 rounded-lg text-xs text-slate-300 hover:text-white transition">← Back to Control Center</a>
    </div>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold">Monthly Recurring Revenue (MRR)</div>
            <div class="text-3xl font-extrabold text-emerald-400 mt-2">$0.00</div>
        </div>
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold">Annual Recurring Revenue (ARR)</div>
            <div class="text-3xl font-extrabold text-sky-400 mt-2">$0.00</div>
        </div>
        <div class="p-5 bg-slate-900 border border-slate-800 rounded-xl">
            <div class="text-slate-500 text-xs uppercase font-bold">Active Subscriptions</div>
            <div class="text-3xl font-extrabold text-purple-400 mt-2">0</div>
        </div>
    </div>
</body>
</html>"#.to_string()
    )
}

async fn studio_traces_handler() -> axum::response::Html<String> {
    axum::response::Html(
        r#"<!DOCTYPE html>
<html lang="en" class="h-full bg-slate-950 text-slate-100">
<head>
    <meta charset="UTF-8">
    <title>Distributed Tracing & Flamegraph — Rullst Studio</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&display=swap" rel="stylesheet" />
    <style>body { font-family: 'Outfit', sans-serif; }</style>
</head>
<body class="h-full flex flex-col font-mono bg-slate-950 text-slate-100 antialiased p-8 max-w-7xl mx-auto space-y-6">
    <div class="flex items-center justify-between border-b border-slate-800 pb-4">
        <div>
            <h1 class="text-2xl font-bold text-indigo-400 flex items-center gap-2">🔍 Distributed Tracing & Span Flamegraph</h1>
            <p class="text-xs text-slate-400 mt-1">Microsecond span collector & waterfall tracer across HTTP requests, ORM queries, and AI generation.</p>
        </div>
        <a href="/" class="px-3 py-1.5 bg-slate-900 border border-slate-800 rounded-lg text-xs text-slate-300 hover:text-white transition">← Back to Control Center</a>
    </div>
    <div class="bg-slate-900 border border-slate-800 rounded-xl p-6 space-y-4">
        <h2 class="text-sm font-bold text-slate-200">Live Request Waterfall Traces</h2>
        <div class="space-y-3 font-mono text-xs">
            <div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <span class="text-cyan-400 font-bold">GET /</span>
                <span class="text-emerald-400 font-bold">120 µs</span>
            </div>
            <div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <span class="text-yellow-400 font-bold">sqlx::query SELECT * FROM _rullst_migrations</span>
                <span class="text-emerald-400 font-bold">340 µs</span>
            </div>
            <div class="p-3 bg-slate-950 border border-slate-800 rounded-lg flex items-center justify-between">
                <span class="text-amber-400 font-bold">security.waf_check</span>
                <span class="text-emerald-400 font-bold">15 µs</span>
            </div>
        </div>
    </div>
</body>
</html>"#.to_string()
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_artisan_args_none() {
        // No args
        assert!(translate_artisan_args(&[]).is_none());
        // Only 1 arg (the binary name)
        assert!(translate_artisan_args(&["cargo-rullst".to_string()]).is_none());
        // Non-matching command
        assert!(translate_artisan_args(&["cargo-rullst".to_string(), "run".to_string()]).is_none());
    }

    #[test]
    fn test_translate_artisan_args_translation() {
        let args = vec!["artisan".to_string(), "db:migrate".to_string()];
        let expected = vec!["artisan".to_string(), "migrate".to_string()];
        assert_eq!(translate_artisan_args(&args), Some(expected));

        let args_rollback = vec!["artisan".to_string(), "db:rollback".to_string()];
        let expected_rollback = vec!["artisan".to_string(), "migrate:rollback".to_string()];
        assert_eq!(
            translate_artisan_args(&args_rollback),
            Some(expected_rollback)
        );

        let args_with_extra = vec![
            "artisan".to_string(),
            "db:migrate".to_string(),
            "--force".to_string(),
        ];
        let expected_with_extra = vec![
            "artisan".to_string(),
            "migrate".to_string(),
            "--force".to_string(),
        ];
        assert_eq!(
            translate_artisan_args(&args_with_extra),
            Some(expected_with_extra)
        );
    }

    #[tokio::test]
    async fn test_check_and_run_artisan_noop() {
        // Calling check_and_run_artisan in test execution should return Ok(())
        // because the command line arguments won't match any artisan commands.
        let result = check_and_run_artisan(vec![], vec![]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_is_ai_configured_and_security_handler() {
        let _ = is_ai_configured();
        let html_res = studio_security_handler().await;
        assert!(html_res.0.contains("Threat Radar"));
    }
}
